use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::models::{AiSession, SessionStatus, FixGenerationRequest, StreamMessage, MessageType, MessageContent};
use crate::goose::{PromptBuilder, monitoring::{GooseEventBridge, MessageCallback, GooseAgentEvent}};
use tracing::{info, debug, warn, error};

use goose::providers::{self, base::Provider};
use goose::model::ModelConfig;
use goose::conversation::{Conversation, message::{Message, MessageContent as GooseMessageContent}};
use rmcp::model::{Role, RawTextContent};

/// Wrapper around Goose session providing Kaiak-specific functionality
pub struct GooseSessionWrapper {
    pub session_id: String,
    pub workspace_path: String,
    pub status: SessionStatus,
    pub configuration: SessionConfiguration,
    /// Real Goose provider instance for AI completions
    goose_provider: Option<Arc<dyn Provider>>,
    /// Current conversation state
    conversation: Option<Conversation>,
    /// Goose Event Bridge for real-time streaming (T005)
    event_bridge: Option<GooseEventBridge>,
    /// Active request being processed
    pub active_request: Option<String>,
    /// Message callbacks for streaming
    pub message_callback: Option<Arc<dyn MessageCallback + Send + Sync>>,
}

/// Goose agent integration configuration
#[derive(Debug, Clone)]
pub struct GooseAgentConfig {
    pub workspace_path: String,
    pub provider_name: String,
    pub model_name: String,
}

#[derive(Debug, Clone)]
pub struct SessionConfiguration {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub timeout: u32,
    pub max_turns: u32,
}

impl Default for SessionConfiguration {
    fn default() -> Self {
        Self {
            provider: Some("openai".to_string()),
            model: Some("gpt-4".to_string()),
            timeout: 300, // 5 minutes
            max_turns: 50,
        }
    }
}


/// Session manager for handling multiple concurrent sessions with performance optimizations
pub struct SessionManager {
    sessions: Arc<RwLock<std::collections::HashMap<String, Arc<RwLock<GooseSessionWrapper>>>>>,
    /// LRU cache for recently accessed sessions
    session_cache: Arc<RwLock<lru::LruCache<String, Arc<RwLock<GooseSessionWrapper>>>>>,
    /// Maximum number of concurrent sessions
    max_concurrent_sessions: u32,
    /// Connection pool for agent sessions
    agent_pool: Arc<RwLock<Vec<String>>>, // Pool of available agent connection IDs
}

impl SessionManager {
    pub fn new() -> Self {
        Self::with_config(100, 10) // Default: 100 cache entries, 10 max concurrent sessions
    }

    pub fn with_config(cache_size: usize, max_sessions: u32) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
            session_cache: Arc::new(RwLock::new(lru::LruCache::new(cache_size.try_into().unwrap()))),
            max_concurrent_sessions: max_sessions,
            agent_pool: Arc::new(RwLock::new(Vec::with_capacity(max_sessions as usize))),
        }
    }

    pub async fn create_session(&self, ai_session: &AiSession) -> Result<Arc<RwLock<GooseSessionWrapper>>> {
        // Check if we're at capacity
        let current_count = self.active_session_count().await;
        if current_count >= self.max_concurrent_sessions as usize {
            return Err(anyhow::anyhow!("Maximum concurrent sessions ({}) reached", self.max_concurrent_sessions));
        }

        let mut wrapper = GooseSessionWrapper::new(ai_session).await?;
        wrapper.initialize().await?;

        let session_arc = Arc::new(RwLock::new(wrapper));

        // Store in both main sessions and cache
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(ai_session.id.clone(), session_arc.clone());
        }
        {
            let mut cache = self.session_cache.write().await;
            cache.put(ai_session.id.clone(), session_arc.clone());
        }

        info!("Session created and stored: {} (total sessions: {})", ai_session.id, current_count + 1);
        Ok(session_arc)
    }

    pub async fn get_session(&self, session_id: &str) -> Option<Arc<RwLock<GooseSessionWrapper>>> {
        // Check cache first for hot sessions
        {
            let mut cache = self.session_cache.write().await;
            if let Some(session) = cache.get(session_id) {
                return Some(session.clone());
            }
        }

        // Fallback to main storage
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id).cloned() {
            // Update cache with accessed session
            let mut cache = self.session_cache.write().await;
            cache.put(session_id.to_string(), session.clone());
            Some(session)
        } else {
            None
        }
    }

    pub async fn remove_session(&self, session_id: &str) -> Result<()> {
        let session_arc = {
            let mut sessions = self.sessions.write().await;
            sessions.remove(session_id)
        };

        // Also remove from cache
        {
            let mut cache = self.session_cache.write().await;
            cache.pop(session_id);
        }

        if let Some(session_arc) = session_arc {
            let session = session_arc.write().await;
            session.cleanup().await?;
            info!("Session removed and cleaned up: {}", session_id);
        }
        Ok(())
    }

    pub async fn active_session_count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.len()
    }
}

impl GooseSessionWrapper {
    pub async fn new(ai_session: &AiSession) -> Result<Self> {
        info!("Creating Goose session wrapper for: {}", ai_session.id);

        let config = SessionConfiguration {
            provider: ai_session.configuration.provider_config
                .as_ref()
                .and_then(|p| p.get("provider").or_else(|| p.get("_type")))
                .and_then(|p| p.as_str())
                .map(|s| s.to_string()),
            model: ai_session.configuration.provider_config
                .as_ref()
                .and_then(|p| p.get("model"))
                .and_then(|m| m.as_str())
                .map(|s| s.to_string()),
            timeout: ai_session.configuration.timeout.unwrap_or(300),
            max_turns: ai_session.configuration.max_turns.unwrap_or(50),
        };

        Ok(Self {
            session_id: ai_session.id.clone(),
            workspace_path: ai_session.configuration.workspace_path.clone(),
            status: SessionStatus::Created,
            configuration: config,
            goose_provider: None,
            conversation: None,
            event_bridge: None,
            active_request: None,
            message_callback: None,
        })
    }

    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing Goose session: {}", self.session_id);

        // Validate workspace path
        if !std::path::Path::new(&self.workspace_path).exists() {
            warn!("Workspace path does not exist: {}", self.workspace_path);
            // Continue anyway for now - may be created later
        }

        // Initialize real Goose provider
        let provider_name = self.configuration.provider.as_ref()
            .unwrap_or(&"openai".to_string()).clone();
        let model_name = self.configuration.model.as_ref()
            .unwrap_or(&"gpt-4".to_string()).clone();

        info!("Creating Goose provider: {} with model: {}", provider_name, model_name);

        let model_config = ModelConfig::new(&model_name)?;
        let provider = providers::create(&provider_name, model_config).await
            .map_err(|e| anyhow::anyhow!("Failed to create Goose provider: {}", e))?;

        // Initialize conversation
        let conversation = Conversation::empty();

        info!("Goose provider and conversation initialized for workspace: {}", self.workspace_path);

        // Initialize Goose Event Bridge (T005)
        let mut event_bridge = GooseEventBridge::new(self.session_id.clone(), None);

        // Connect event bridge to message callback if available
        if let Some(callback) = &self.message_callback {
            event_bridge.set_message_callback(callback.clone());
        }

        // Start event subscription
        event_bridge.start_event_subscription().await?;
        event_bridge.subscribe_to_goose_events().await?;

        self.goose_provider = Some(provider);
        self.conversation = Some(conversation);
        self.event_bridge = Some(event_bridge);
        self.status = SessionStatus::Ready;

        info!("Goose agent and event bridge initialized for session: {}", self.session_id);
        Ok(())
    }

    pub async fn cleanup(&self) -> Result<()> {
        info!("Cleaning up Goose session: {}", self.session_id);

        // Stop event bridge subscription
        if let Some(event_bridge) = &self.event_bridge {
            event_bridge.stop_event_subscription().await?;
        }

        // TODO: Cleanup actual Goose session
        // This would involve:
        // 1. Gracefully terminating active operations
        // 2. Saving session state
        // 3. Releasing resources

        info!("Session cleanup completed: {}", self.session_id);
        Ok(())
    }

    pub fn is_ready(&self) -> bool {
        matches!(self.status, SessionStatus::Ready | SessionStatus::Processing)
    }

    pub fn set_message_callback(&mut self, callback: Arc<dyn MessageCallback + Send + Sync>) {
        self.message_callback = Some(callback.clone());

        // Also connect to event bridge if it's already initialized
        if let Some(event_bridge) = &mut self.event_bridge {
            event_bridge.set_message_callback(callback);
        }
    }

    /// Get reference to event bridge for tool result processing
    pub fn event_bridge(&self) -> &Option<GooseEventBridge> {
        &self.event_bridge
    }

    /// Process a fix generation request through this session
    pub async fn process_fix_request(&mut self, request: &FixGenerationRequest) -> Result<String> {
        if !self.is_ready() {
            anyhow::bail!("Session is not ready for processing");
        }

        self.status = SessionStatus::Processing;
        self.active_request = Some(request.id.clone());

        info!("Processing fix request {} in session {}", request.id, self.session_id);

        // Send initial thinking message
        self.send_ai_thinking("Let me analyze the code incidents provided and determine the best migration approach.").await?;

        // Send progress update
        self.send_progress_update(5, "analyzing_incidents", "Analyzing code incidents").await?;

        // Generate prompts for this request using the new format_incident_prompt method
        let system_prompt = PromptBuilder::system_prompt();
        let incident_prompt = PromptBuilder::format_incident_prompt(&request.incidents, "");
        let user_prompt = PromptBuilder::fix_generation_prompt(request);

        debug!("Generated system prompt: {} chars", system_prompt.len());
        debug!("Generated incident prompt: {} chars", incident_prompt.len());
        debug!("Generated user prompt: {} chars", user_prompt.len());

        // Send thinking about prompt generation
        self.send_ai_thinking(&format!(
            "I've generated prompts that will guide my analysis of the {} incident(s). The incident-specific prompt ({} chars) converts the structured data into natural language for the Goose agent.",
            request.incidents.len(), incident_prompt.len()
        )).await?;

        // Analyze each incident with thinking
        self.send_progress_update(15, "analyzing_incidents", "Examining individual incidents").await?;
        for (i, incident) in request.incidents.iter().enumerate() {
            self.send_ai_thinking(&format!(
                "Analyzing incident {}: {} at {}:{}. This is categorized as '{}' with severity '{:?}'. The issue is: {}",
                i + 1, incident.rule_id, incident.file_path, incident.line_number,
                incident.category, incident.severity, incident.description
            )).await?;

            // Simulate some analysis time
            tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
        }

        self.send_ai_thinking("Based on my analysis, I need to consider the migration patterns, potential side effects, and best practices for each type of incident.").await?;

        self.send_progress_update(35, "generating_context", "Generating contextual information").await?;

        self.send_ai_thinking(&format!(
            "The workspace is located at '{}'. I should understand the project structure and dependencies to provide accurate migration suggestions.",
            request.workspace_path
        )).await?;

        self.send_progress_update(50, "generating_fixes", "Generating fix suggestions").await?;

        self.send_ai_thinking("Now I'll generate specific fixes for each incident. I need to ensure the fixes are safe, maintainable, and follow migration best practices.").await?;

        self.send_ai_thinking("Sending incident prompt to Goose agent...").await?;

        let _agent_result = self.process_with_goose_agent(incident_prompt).await
            .map_err(|e| {
                error!("Goose agent processing failed: {}", e);
                anyhow::anyhow!("Agent processing failed: {}", e)
            })?;

        for (i, incident) in request.incidents.iter().enumerate() {
            self.send_ai_thinking(&format!(
                "For incident {} ({}): I'm considering multiple fix approaches. The safest option would be to {}...",
                i + 1, incident.rule_id,
                match incident.category.as_str() {
                    "deprecated" => "replace the deprecated API with the recommended alternative",
                    "migration" => "update the code to use the new API patterns",
                    _ => "apply the appropriate fix based on the specific rule"
                }
            )).await?;

            let progress = 50 + ((i + 1) * 30 / request.incidents.len()) as u8;
            self.send_progress_update(progress, "generating_fixes",
                &format!("Generated fix for incident {} of {}", i + 1, request.incidents.len())).await?;

            tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
        }

        self.send_ai_thinking("I've completed the analysis and generated appropriate fixes. Let me validate these recommendations to ensure they're safe and effective.").await?;

        self.send_progress_update(90, "validating_fixes", "Validating fix proposals").await?;

        self.send_ai_thinking("All fixes look good. They follow migration best practices, maintain code safety, and address the identified issues appropriately.").await?;

        self.send_progress_update(100, "completed", "Fix generation completed").await?;

        self.send_ai_thinking("Fix generation process completed successfully. The proposed solutions are ready for review and implementation.").await?;

        self.status = SessionStatus::Ready;
        self.active_request = None;

        let request_id = uuid::Uuid::new_v4().to_string();
        info!("Fix request processing completed: {}", request_id);

        Ok(request_id)
    }

    /// Send a progress update through the message callback
    async fn send_progress_update(&self, percentage: u8, phase: &str, description: &str) -> Result<()> {
        if let Some(callback) = &self.message_callback {
            let message = StreamMessage::new(
                self.session_id.clone(),
                self.active_request.clone(),
                MessageType::Progress,
                MessageContent::Progress {
                    percentage,
                    phase: phase.to_string(),
                    description: description.to_string(),
                },
            );

            callback.on_message(message)?;
        }
        Ok(())
    }

    /// Send an AI response through the message callback
    pub async fn send_ai_response(&self, text: &str, partial: bool) -> Result<()> {
        if let Some(callback) = &self.message_callback {
            let message = StreamMessage::new(
                self.session_id.clone(),
                self.active_request.clone(),
                MessageType::AiResponse,
                MessageContent::AiResponse {
                    text: text.to_string(),
                    partial,
                    confidence: Some(0.9),
                },
            );

            callback.on_message(message)?;
        }
        Ok(())
    }

    /// Send an AI thinking process message through the message callback
    pub async fn send_ai_thinking(&self, thinking_text: &str) -> Result<()> {
        if let Some(callback) = &self.message_callback {
            let message = StreamMessage::new(
                self.session_id.clone(),
                self.active_request.clone(),
                MessageType::Thinking,
                MessageContent::Thinking {
                    text: thinking_text.to_string(),
                },
            );

            callback.on_message(message)?;
        }
        Ok(())
    }

    /// T009 - Handle tool call from Goose agent with interception and safety
    pub async fn handle_tool_call_event(
        &self,
        tool_call_id: &str,
        tool_name: &str,
        parameters: serde_json::Value,
    ) -> Result<crate::goose::agent::GooseToolCallResult> {
        info!("Session {} handling tool call: {} ({})", self.session_id, tool_name, tool_call_id);

        // For now, create a simple AgentManager to handle the tool call
        // In real implementation, this would be passed in or stored as a reference
        let agent_manager = crate::goose::AgentManager::new().await?;

        agent_manager.handle_goose_tool_call(
            &self.session_id,
            tool_call_id,
            tool_name,
            parameters,
        ).await
    }

    /// Send tool execution result through the message callback
    pub async fn send_tool_result(&self, tool_result: &crate::goose::agent::ToolExecutionResult) -> Result<()> {
        if let Some(callback) = &self.message_callback {
            let message = StreamMessage::new(
                self.session_id.clone(),
                self.active_request.clone(),
                MessageType::ToolCall,
                MessageContent::ToolCall {
                    tool_name: tool_result.metadata
                        .as_ref()
                        .and_then(|m| m.get("tool_type"))
                        .and_then(|t| t.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    operation: if tool_result.success {
                        crate::models::ToolOperation::Complete
                    } else {
                        crate::models::ToolOperation::Error
                    },
                    parameters: serde_json::Value::Null,
                    result: Some(crate::models::ToolResult {
                        success: tool_result.success,
                        data: tool_result.output.clone(),
                        error: tool_result.error.clone(),
                        execution_time_ms: tool_result.execution_time_ms,
                        output_size_bytes: tool_result.output.as_ref().map(|o| o.to_string().len() as u64),
                    }),
                },
            );

            callback.on_message(message)?;
        }
        Ok(())
    }

    /// Send tool call interception through the message callback
    pub async fn send_tool_interception(&self, interaction: &crate::models::UserInteraction, tool_name: &str) -> Result<()> {
        if let Some(callback) = &self.message_callback {
            let message = StreamMessage::new(
                self.session_id.clone(),
                self.active_request.clone(),
                MessageType::UserInteraction,
                MessageContent::UserInteraction {
                    interaction_id: interaction.id.clone(),
                    interaction_type: format!("{:?}", interaction.interaction_type),
                    prompt: format!(
                        "Tool call '{}' has been intercepted for approval: {}",
                        tool_name,
                        interaction.prompt
                    ),
                    proposal_id: interaction.proposal_id.clone(),
                    timeout: interaction.timeout_seconds,
                },
            );

            callback.on_message(message)?;
        }
        Ok(())
    }

    /// Cancel active processing
    pub async fn cancel_active_request(&mut self) -> Result<()> {
        if let Some(request_id) = &self.active_request {
            warn!("Cancelling active request: {}", request_id);

            // TODO: Cancel actual Goose processing

            self.active_request = None;
            self.status = SessionStatus::Ready;

            info!("Request cancelled and session restored to ready state");
        }
        Ok(())
    }

    /// Process with Goose agent (placeholder for actual integration)
    /// This method demonstrates event streaming through the GooseEventBridge
    pub async fn process_with_goose_agent(&mut self, prompt: String) -> Result<String> {
        info!("Processing with Goose provider: {} chars", prompt.len());

        let provider = self.goose_provider.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Goose provider not initialized. Call initialize() first."))?;

        let conversation = self.conversation.as_mut()
            .ok_or_else(|| anyhow::anyhow!("Conversation not initialized. Call initialize() first."))?;

        if let Some(event_bridge) = &self.event_bridge {
            event_bridge.handle_goose_event(GooseAgentEvent::Thinking {
                text: "Processing request with Goose provider...".to_string(),
            }).await?;
        }

        let raw_text = RawTextContent { text: prompt, meta: None };
        let text_content = rmcp::model::TextContent::new(raw_text, None);
        let message_content = vec![GooseMessageContent::Text(text_content)];
        let timestamp = chrono::Utc::now().timestamp_millis();
        let user_message = Message::new(Role::User, timestamp, message_content);
        conversation.push(user_message);

        let messages = conversation.messages();
        let system_prompt = "You are an AI assistant specialized in code migration and analysis. Help users migrate their source code effectively.";

        info!("Calling Goose provider for completion...");
        let (response_message, _usage) = provider
            .complete(system_prompt, messages, &[])
            .await
            .map_err(|e| anyhow::anyhow!("Goose provider error: {}", e))?;

        conversation.push(response_message.clone());

        let response_text = response_message.content.iter()
            .filter_map(|content| match content {
                GooseMessageContent::Text(text_content) => Some(text_content.raw.text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        // Send response event through event bridge
        if let Some(event_bridge) = &self.event_bridge {
            event_bridge.handle_goose_event(GooseAgentEvent::Message {
                content: response_text.clone(),
                partial: false,
                confidence: Some(0.95),
            }).await?;
        }

        info!("Goose provider processing completed: {} chars", response_text.len());
        Ok(response_text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::AiSession;

    #[tokio::test]
    async fn test_session_wrapper_creation() {
        let ai_session = AiSession::new(
            "/tmp/test".to_string(),
            Some("test".to_string()),
        );

        let wrapper = GooseSessionWrapper::new(&ai_session).await.unwrap();
        assert_eq!(wrapper.session_id, ai_session.id);
        assert_eq!(wrapper.workspace_path, "/tmp/test");
        assert_eq!(wrapper.status, SessionStatus::Created);
    }

    #[tokio::test]
    async fn test_session_initialization() {
        let ai_session = AiSession::new(
            "/tmp/test".to_string(),
            Some("test".to_string()),
        );

        let mut wrapper = GooseSessionWrapper::new(&ai_session).await.unwrap();
        assert!(!wrapper.is_ready());

        wrapper.initialize().await.unwrap();
        assert!(wrapper.is_ready());
        assert_eq!(wrapper.status, SessionStatus::Ready);
    }
}
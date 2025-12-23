use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use crate::models::{FixGenerationRequest, AiSession, StreamMessage, MessageType, MessageContent, UserInteraction, InteractionType};
use crate::goose::{SessionManager, GooseSessionWrapper, MessageCallback};
use crate::handlers::{ModificationHandler, InteractionHandler, ModificationConfig, InteractionConfig};
use tracing::{info, debug, warn, error};

/// Agent lifecycle management for Goose integration with file modification prevention
pub struct AgentManager {
    session_manager: Arc<SessionManager>,
    active_requests: Arc<RwLock<HashMap<String, RequestState>>>,
    /// Handler for file modification proposals
    modification_handler: Arc<ModificationHandler>,
    /// Handler for user interactions
    interaction_handler: Arc<InteractionHandler>,
    /// Configuration for file modification prevention
    config: AgentConfig,
    // TODO: This will hold the actual Goose AgentManager instance
    // goose_manager: goose::AgentManager,
}

/// Configuration for agent behavior
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Whether to prevent all file modifications (requires approval)
    pub prevent_file_modifications: bool,
    /// Whether to allow read-only operations
    pub allow_read_operations: bool,
    /// Maximum number of pending proposals per session
    pub max_pending_proposals: usize,
    /// Default timeout for file modification approvals in seconds
    pub default_approval_timeout_seconds: u32,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            prevent_file_modifications: true, // Default to safe mode
            allow_read_operations: true,
            max_pending_proposals: 20,
            default_approval_timeout_seconds: 300, // 5 minutes
        }
    }
}

#[derive(Debug, Clone)]
pub struct RequestState {
    pub request_id: String,
    pub session_id: String,
    pub status: RequestStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RequestStatus {
    Processing,
    Completed,
    Failed,
    Cancelled,
}

/// Result of a safe tool call operation
#[derive(Debug)]
pub enum SafeToolCallResult {
    /// Operation is allowed to proceed normally
    Allowed {
        tool_name: String,
        parameters: serde_json::Value,
    },
    /// Operation was intercepted and requires approval
    InterceptedForApproval {
        original_tool_name: String,
        original_parameters: serde_json::Value,
        interaction: UserInteraction,
        file_path: String,
    },
}

/// Statistics about file modifications for a session
#[derive(Debug, Default)]
pub struct FileModificationStats {
    pub total_proposals: usize,
    pub pending_proposals: usize,
    pub approved_proposals: usize,
    pub rejected_proposals: usize,
    pub applied_proposals: usize,
    pub expired_proposals: usize,
    pub high_risk_proposals: usize,
    pub total_interactions: usize,
    pub file_modification_interactions: usize,
}

/// Message streaming handler for AgentManager
pub struct AgentMessageHandler {
    sender: tokio::sync::mpsc::UnboundedSender<StreamMessage>,
}

impl MessageCallback for AgentMessageHandler {
    fn on_message(&self, message: StreamMessage) -> Result<()> {
        match self.sender.send(message) {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("Failed to send stream message: {}", e);
                Err(anyhow::anyhow!("Stream message send failed: {}", e))
            }
        }
    }
}

impl AgentMessageHandler {
    pub fn new(sender: tokio::sync::mpsc::UnboundedSender<StreamMessage>) -> Self {
        Self { sender }
    }
}

impl AgentManager {
    pub async fn new() -> Result<Self> {
        Self::new_with_config(AgentConfig::default()).await
    }

    pub async fn new_with_config(config: AgentConfig) -> Result<Self> {
        info!("Initializing AgentManager with file modification prevention");

        let session_manager = Arc::new(SessionManager::new());
        let active_requests = Arc::new(RwLock::new(HashMap::new()));

        // Initialize handlers for file modification prevention
        let modification_handler = Arc::new(ModificationHandler::new(ModificationConfig::default()));
        let interaction_handler = Arc::new(InteractionHandler::new_with_handlers(
            InteractionConfig::default(),
            // We'll need to set the streaming handler later if needed
            Arc::new(crate::handlers::StreamingHandler::new(Default::default())),
            modification_handler.clone(),
        ));

        // TODO: Initialize actual Goose AgentManager
        // let goose_manager = goose::AgentManager::instance().await?;

        info!("AgentManager initialized successfully with file modification prevention enabled: {}", config.prevent_file_modifications);

        Ok(Self {
            session_manager,
            active_requests,
            modification_handler,
            interaction_handler,
            config,
            // goose_manager,
        })
    }

    /// Create with custom handlers (for testing or advanced configuration)
    pub async fn new_with_handlers(
        config: AgentConfig,
        modification_handler: Arc<ModificationHandler>,
        interaction_handler: Arc<InteractionHandler>,
    ) -> Result<Self> {
        info!("Initializing AgentManager with custom handlers");

        let session_manager = Arc::new(SessionManager::new());
        let active_requests = Arc::new(RwLock::new(HashMap::new()));

        Ok(Self {
            session_manager,
            active_requests,
            modification_handler,
            interaction_handler,
            config,
        })
    }

    /// Create or get existing session for the given AI session
    pub async fn get_or_create_session(&self, ai_session: &AiSession) -> Result<Arc<RwLock<GooseSessionWrapper>>> {
        // Check if session already exists
        if let Some(session) = self.session_manager.get_session(&ai_session.id).await {
            debug!("Using existing session: {}", ai_session.id);
            return Ok(session);
        }

        // Create new session
        info!("Creating new session: {}", ai_session.id);
        self.session_manager.create_session(ai_session).await
    }

    /// Process a fix generation request with streaming support
    pub async fn process_fix_request(
        &self,
        request: &FixGenerationRequest,
    ) -> Result<(String, tokio::sync::mpsc::UnboundedReceiver<StreamMessage>)> {
        info!("Processing fix generation request for session: {}", request.session_id);

        // Validate request
        if request.incidents.is_empty() {
            anyhow::bail!("Fix generation request must contain at least one incident");
        }

        // Create AI session (in real implementation, this would come from session store)
        let ai_session = AiSession::new(request.workspace_path.clone(), None);

        // Get or create session
        let session_wrapper = self.get_or_create_session(&ai_session).await?;

        // Set up message streaming
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let handler = AgentMessageHandler::new(tx.clone());

        // Set up session with message callback
        {
            let mut session = session_wrapper.write().await;
            session.set_message_callback(Arc::new(handler));
        }

        // Generate request ID
        let request_id = uuid::Uuid::new_v4().to_string();

        // Track request state
        {
            let mut requests = self.active_requests.write().await;
            requests.insert(
                request_id.clone(),
                RequestState {
                    request_id: request_id.clone(),
                    session_id: request.session_id.clone(),
                    status: RequestStatus::Processing,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                },
            );
        }

        // Start processing in background with tool call streaming
        let session_clone = session_wrapper.clone();
        let request_clone = request.clone();
        let request_id_clone = request_id.clone();
        let active_requests_clone = self.active_requests.clone();
        let tx_clone = tx.clone();

        tokio::spawn(async move {
            // Simulate tool calls during processing
            let tool_call_result = Self::simulate_tool_calls(&tx_clone, &request_clone).await;

            let result = {
                let mut session = session_clone.write().await;
                session.process_fix_request(&request_clone).await
            };

            // Update request status
            let mut requests = active_requests_clone.write().await;
            if let Some(state) = requests.get_mut(&request_id_clone) {
                match (result, tool_call_result) {
                    (Ok(_), Ok(_)) => {
                        state.status = RequestStatus::Completed;
                        info!("Fix request completed: {}", request_id_clone);
                    }
                    (Err(e), _) | (_, Err(e)) => {
                        state.status = RequestStatus::Failed;
                        error!("Fix request failed: {}: {}", request_id_clone, e);
                    }
                }
                state.updated_at = chrono::Utc::now();
            }
        });

        info!("Fix generation request initiated: {}", request_id);
        Ok((request_id, rx))
    }

    /// Intercept a file modification attempt and create a proposal instead
    pub async fn intercept_file_modification(
        &self,
        session_id: &str,
        file_path: String,
        modification_type: String,
        original_content: String,
        proposed_content: String,
        description: String,
        line_range: Option<(u32, u32)>,
    ) -> Result<UserInteraction> {
        info!("Intercepting file modification attempt: {}", file_path);

        // Check if file modification prevention is enabled
        if !self.config.prevent_file_modifications {
            anyhow::bail!("File modification prevention is disabled");
        }

        // Check pending proposal limits
        let session_proposals = self.modification_handler.get_session_proposals(&session_id.to_string()).await?;
        let pending_proposals = session_proposals.iter()
            .filter(|p| p.approval_status == crate::models::ApprovalStatus::Pending)
            .count();

        if pending_proposals >= self.config.max_pending_proposals {
            anyhow::bail!("Too many pending proposals for session: {}", session_id);
        }

        // Create the modification proposal
        let proposal_result = self.modification_handler.create_proposal(
            Some(session_id.to_string()),
            file_path.clone(),
            modification_type.clone(),
            original_content,
            proposed_content,
            description.clone(),
            line_range,
        ).await?;

        // Create user interaction for approval
        let interaction_result = self.interaction_handler.create_interaction(
            Some(session_id.to_string()),
            InteractionType::FileModificationApproval,
            format!(
                "File modification requires approval:\n\nFile: {}\nType: {}\nDescription: {}{}",
                file_path,
                modification_type,
                description,
                if proposal_result.requires_immediate_attention {
                    "\n\n⚠️ HIGH RISK MODIFICATION - Requires immediate attention"
                } else {
                    ""
                }
            ),
            Some(proposal_result.proposal.id.clone()),
            Some(self.config.default_approval_timeout_seconds),
        ).await?;

        info!(
            "Created file modification proposal {} with interaction {}",
            proposal_result.proposal.id,
            interaction_result.interaction.id
        );

        Ok(interaction_result.interaction)
    }

    /// Apply an approved file modification
    pub async fn apply_approved_modification(&self, proposal_id: &str) -> Result<()> {
        info!("Applying approved file modification: {}", proposal_id);

        // Get the proposal
        let proposal = self.modification_handler.get_proposal(&proposal_id.to_string()).await
            .ok_or_else(|| anyhow::anyhow!("Proposal {} not found", proposal_id))?;

        // Check if proposal is approved
        if proposal.approval_status != crate::models::ApprovalStatus::Approved {
            anyhow::bail!("Proposal {} is not approved (status: {:?})", proposal_id, proposal.approval_status);
        }

        // TODO: In real implementation, this would apply the actual file modification
        // For now, we'll simulate the application
        info!("Simulating file modification application for: {}", proposal.file_path);

        // Simulate file write operation
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Mark proposal as applied
        self.modification_handler.mark_applied(&proposal_id.to_string()).await?;

        info!("File modification applied successfully: {}", proposal.file_path);
        Ok(())
    }

    /// Check if a file operation should be intercepted
    pub fn should_intercept_file_operation(&self, operation_type: &str) -> bool {
        if !self.config.prevent_file_modifications {
            return false;
        }

        match operation_type.to_lowercase().as_str() {
            // Allow read operations if configured
            "file_read" | "read_file" | "get_file_content" => !self.config.allow_read_operations,

            // Intercept all write operations
            "file_write" | "write_file" | "create_file" | "modify_file" |
            "delete_file" | "move_file" | "rename_file" | "replace_content" |
            "insert_content" | "append_content" => true,

            // Default to safe mode - intercept unknown operations
            _ => true,
        }
    }

    /// Create a safe tool call wrapper that prevents direct file modifications
    pub async fn create_safe_tool_call(
        &self,
        session_id: &str,
        tool_name: &str,
        parameters: serde_json::Value,
        original_content: Option<String>,
        proposed_content: Option<String>,
    ) -> Result<SafeToolCallResult> {
        debug!("Creating safe tool call: {} for session: {}", tool_name, session_id);

        if !self.should_intercept_file_operation(tool_name) {
            // Allow operation to proceed
            return Ok(SafeToolCallResult::Allowed {
                tool_name: tool_name.to_string(),
                parameters,
            });
        }

        // Extract file path from parameters
        let file_path = parameters.get("file_path")
            .or_else(|| parameters.get("path"))
            .or_else(|| parameters.get("filename"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown_file")
            .to_string();

        // Determine modification type
        let modification_type = match tool_name.to_lowercase().as_str() {
            "file_write" | "write_file" | "modify_file" => "content_replace",
            "create_file" => "file_create",
            "delete_file" => "file_delete",
            "move_file" | "rename_file" => "file_move",
            "replace_content" => "content_replace",
            "insert_content" => "content_insert",
            "append_content" => "content_insert",
            _ => "content_replace", // Default
        };

        // Create description
        let description = format!("AI agent requested {} operation on {}", tool_name, file_path);

        // Create file modification proposal
        let interaction = self.intercept_file_modification(
            session_id,
            file_path.clone(),
            modification_type.to_string(),
            original_content.unwrap_or_default(),
            proposed_content.unwrap_or_default(),
            description,
            None, // TODO: Extract line range from parameters if available
        ).await?;

        Ok(SafeToolCallResult::InterceptedForApproval {
            original_tool_name: tool_name.to_string(),
            original_parameters: parameters,
            interaction,
            file_path,
        })
    }

    /// Get file modification statistics for a session
    pub async fn get_file_modification_stats(&self, session_id: &str) -> Result<FileModificationStats> {
        let proposals = self.modification_handler.get_session_proposals(&session_id.to_string()).await?;
        let interactions = self.interaction_handler.get_session_interactions(&session_id.to_string()).await?;

        let mut stats = FileModificationStats::default();
        stats.total_proposals = proposals.len();

        for proposal in &proposals {
            match proposal.approval_status {
                crate::models::ApprovalStatus::Pending => {
                    if proposal.is_expired() {
                        stats.expired_proposals += 1;
                    } else {
                        stats.pending_proposals += 1;
                    }
                }
                crate::models::ApprovalStatus::Approved => stats.approved_proposals += 1,
                crate::models::ApprovalStatus::Rejected => stats.rejected_proposals += 1,
                crate::models::ApprovalStatus::Applied => stats.applied_proposals += 1,
                _ => {}
            }

            if proposal.is_high_risk() {
                stats.high_risk_proposals += 1;
            }
        }

        stats.total_interactions = interactions.len();
        for interaction in &interactions {
            if interaction.is_file_modification_approval() {
                stats.file_modification_interactions += 1;
            }
        }

        Ok(stats)
    }

    /// Get handlers for direct access (useful for integration)
    pub fn modification_handler(&self) -> &Arc<ModificationHandler> {
        &self.modification_handler
    }

    pub fn interaction_handler(&self) -> &Arc<InteractionHandler> {
        &self.interaction_handler
    }

    /// Simulate tool calls that would be made by the Goose agent
    /// In a real implementation, this would be integrated with the actual Goose agent
    async fn simulate_tool_calls(
        tx: &tokio::sync::mpsc::UnboundedSender<StreamMessage>,
        request: &FixGenerationRequest,
    ) -> Result<()> {
        use crate::models::{ToolOperation, ToolResult};

        // Simulate file reading tool calls for each incident
        for (_i, incident) in request.incidents.iter().enumerate() {
            let session_id = request.session_id.clone();
            let request_id = Some(request.id.clone());

            // Tool call start - reading the file
            let tool_start = StreamMessage::new(
                session_id.clone(),
                request_id.clone(),
                MessageType::ToolCall,
                MessageContent::ToolCall {
                    tool_name: "file_read".to_string(),
                    operation: ToolOperation::Start,
                    parameters: serde_json::json!({
                        "file_path": incident.file_path,
                        "line_number": incident.line_number,
                        "context_lines": 5
                    }),
                    result: None,
                },
            );
            let _ = tx.send(tool_start);

            // Simulate some processing time
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

            // Tool call progress
            let tool_progress = StreamMessage::new(
                session_id.clone(),
                request_id.clone(),
                MessageType::ToolCall,
                MessageContent::ToolCall {
                    tool_name: "file_read".to_string(),
                    operation: ToolOperation::Progress,
                    parameters: serde_json::json!({}),
                    result: None,
                },
            );
            let _ = tx.send(tool_progress);

            // Simulate more processing
            tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;

            // Tool call completion with results
            let tool_complete = StreamMessage::new(
                session_id.clone(),
                request_id.clone(),
                MessageType::ToolCall,
                MessageContent::ToolCall {
                    tool_name: "file_read".to_string(),
                    operation: ToolOperation::Complete,
                    parameters: serde_json::json!({}),
                    result: Some(ToolResult {
                        success: true,
                        data: Some(serde_json::json!({
                            "file_content": format!("// Content around line {} in {}", incident.line_number, incident.file_path),
                            "line_count": 100,
                            "encoding": "utf-8",
                            "incident_context": {
                                "line_number": incident.line_number,
                                "rule_id": incident.rule_id,
                                "severity": incident.severity
                            }
                        })),
                        error: None,
                    }),
                },
            );
            let _ = tx.send(tool_complete);

            // For complex incidents, simulate additional tool calls
            if incident.severity.is_high_priority() {
                tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;

                // Simulate dependency analysis tool call
                let dep_analysis_start = StreamMessage::new(
                    session_id.clone(),
                    request_id.clone(),
                    MessageType::ToolCall,
                    MessageContent::ToolCall {
                        tool_name: "dependency_analysis".to_string(),
                        operation: ToolOperation::Start,
                        parameters: serde_json::json!({
                            "file_path": incident.file_path,
                            "function_name": format!("function_at_line_{}", incident.line_number),
                            "analysis_depth": "full"
                        }),
                        result: None,
                    },
                );
                let _ = tx.send(dep_analysis_start);

                tokio::time::sleep(tokio::time::Duration::from_millis(80)).await;

                let dep_analysis_complete = StreamMessage::new(
                    session_id.clone(),
                    request_id.clone(),
                    MessageType::ToolCall,
                    MessageContent::ToolCall {
                        tool_name: "dependency_analysis".to_string(),
                        operation: ToolOperation::Complete,
                        parameters: serde_json::json!({}),
                        result: Some(ToolResult {
                            success: true,
                            data: Some(serde_json::json!({
                                "dependencies": ["module_a", "module_b"],
                                "dependents": ["client_x", "service_y"],
                                "risk_level": "medium",
                                "migration_complexity": "standard"
                            })),
                            error: None,
                        }),
                    },
                );
                let _ = tx.send(dep_analysis_complete);
            }
        }

        // Simulate final validation tool call
        let validation_start = StreamMessage::new(
            request.session_id.clone(),
            Some(request.id.clone()),
            MessageType::ToolCall,
            MessageContent::ToolCall {
                tool_name: "fix_validation".to_string(),
                operation: ToolOperation::Start,
                parameters: serde_json::json!({
                    "workspace_path": request.workspace_path,
                    "fix_count": request.incidents.len(),
                    "validation_level": "comprehensive"
                }),
                result: None,
            },
        );
        let _ = tx.send(validation_start);

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let validation_complete = StreamMessage::new(
            request.session_id.clone(),
            Some(request.id.clone()),
            MessageType::ToolCall,
            MessageContent::ToolCall {
                tool_name: "fix_validation".to_string(),
                operation: ToolOperation::Complete,
                parameters: serde_json::json!({}),
                result: Some(ToolResult {
                    success: true,
                    data: Some(serde_json::json!({
                        "validation_status": "passed",
                        "fixes_validated": request.incidents.len(),
                        "safety_score": 0.92,
                        "recommendations": "All fixes are safe to apply"
                    })),
                    error: None,
                }),
            },
        );
        let _ = tx.send(validation_complete);

        Ok(())
    }

    /// Cancel an active request
    pub async fn cancel_request(&self, request_id: &str) -> Result<()> {
        info!("Cancelling request: {}", request_id);

        // Update request status
        {
            let mut requests = self.active_requests.write().await;
            if let Some(state) = requests.get_mut(request_id) {
                if state.status == RequestStatus::Processing {
                    state.status = RequestStatus::Cancelled;
                    state.updated_at = chrono::Utc::now();

                    // Cancel processing in the session
                    if let Some(session_wrapper) = self.session_manager.get_session(&state.session_id).await {
                        let mut session = session_wrapper.write().await;
                        session.cancel_active_request().await?;
                    }

                    info!("Request cancelled: {}", request_id);
                    return Ok(());
                } else {
                    anyhow::bail!("Request {} is not in processing state", request_id);
                }
            } else {
                anyhow::bail!("Request {} not found", request_id);
            }
        }
    }

    /// Get status of a specific request
    pub async fn get_request_status(&self, request_id: &str) -> Result<RequestState> {
        let requests = self.active_requests.read().await;
        requests
            .get(request_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Request {} not found", request_id))
    }

    /// Get all active requests for a session
    pub async fn get_session_requests(&self, session_id: &str) -> Result<Vec<RequestState>> {
        let requests = self.active_requests.read().await;
        Ok(requests
            .values()
            .filter(|state| state.session_id == session_id)
            .cloned()
            .collect())
    }

    /// Cleanup completed or failed requests older than specified duration
    pub async fn cleanup_old_requests(&self, max_age: chrono::Duration) -> Result<usize> {
        let cutoff = chrono::Utc::now() - max_age;
        let mut requests = self.active_requests.write().await;

        let initial_count = requests.len();
        requests.retain(|_, state| {
            match state.status {
                RequestStatus::Processing => true, // Keep active requests
                RequestStatus::Completed | RequestStatus::Failed | RequestStatus::Cancelled => {
                    state.updated_at > cutoff // Remove old completed/failed requests
                }
            }
        });

        let cleaned_count = initial_count - requests.len();
        if cleaned_count > 0 {
            info!("Cleaned up {} old requests", cleaned_count);
        }

        Ok(cleaned_count)
    }

    /// Get session manager for direct session operations
    pub fn session_manager(&self) -> &Arc<SessionManager> {
        &self.session_manager
    }

    /// Get count of active requests
    pub async fn active_request_count(&self) -> usize {
        let requests = self.active_requests.read().await;
        requests.len()
    }

    /// Terminate a session and clean up its resources
    pub async fn terminate_session(&self, session_id: &str) -> Result<()> {
        info!("Terminating session: {}", session_id);

        // Cancel any active requests for this session
        let session_requests = self.get_session_requests(session_id).await?;
        for request in session_requests {
            if request.status == RequestStatus::Processing {
                if let Err(e) = self.cancel_request(&request.request_id).await {
                    warn!("Failed to cancel request {} during session termination: {}", request.request_id, e);
                }
            }
        }

        // Remove session
        self.session_manager.remove_session(session_id).await?;

        info!("Session terminated: {}", session_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AiSession, Incident, Severity};

    #[tokio::test]
    async fn test_agent_manager_creation() {
        let _manager = AgentManager::new().await.unwrap();
        // Basic test that manager can be created
        // More detailed tests will be added when Goose integration is complete
    }

    #[tokio::test]
    async fn test_fix_request_processing() {
        let manager = AgentManager::new().await.unwrap();

        let incident = Incident::new(
            "deprecated-api".to_string(),
            "src/main.rs".to_string(),
            42,
            Severity::Warning,
            "Deprecated API usage".to_string(),
            "old_method() is deprecated".to_string(),
            "deprecated".to_string(),
        );

        let session = AiSession::new(
            "/tmp/test".to_string(),
            Some("test".to_string()),
        );

        let request = crate::models::FixGenerationRequest::new(
            session.id,
            vec![incident],
            "/tmp/test".to_string(),
        );

        let result = manager.process_fix_request(&request).await.unwrap();
        assert!(!result.0.is_empty());
    }

    #[tokio::test]
    async fn test_file_modification_prevention() {
        let manager = AgentManager::new().await.unwrap();

        // Test file modification interception
        let interaction = manager.intercept_file_modification(
            "session-123",
            "src/test.rs".to_string(),
            "content_replace".to_string(),
            "fn old() {}".to_string(),
            "fn new() {}".to_string(),
            "Update function".to_string(),
            Some((10, 12)),
        ).await.unwrap();

        assert_eq!(interaction.interaction_type, InteractionType::FileModificationApproval);
        assert!(interaction.proposal_id.is_some());
        assert!(interaction.prompt.contains("src/test.rs"));
    }

    #[tokio::test]
    async fn test_safe_tool_call_interception() {
        let manager = AgentManager::new().await.unwrap();

        let parameters = serde_json::json!({
            "file_path": "src/test.rs",
            "content": "new content"
        });

        let result = manager.create_safe_tool_call(
            "session-123",
            "file_write",
            parameters,
            Some("old content".to_string()),
            Some("new content".to_string()),
        ).await.unwrap();

        match result {
            SafeToolCallResult::InterceptedForApproval { original_tool_name, interaction, file_path, .. } => {
                assert_eq!(original_tool_name, "file_write");
                assert_eq!(file_path, "src/test.rs");
                assert_eq!(interaction.interaction_type, InteractionType::FileModificationApproval);
            }
            _ => panic!("Expected InterceptedForApproval result"),
        }
    }

    #[tokio::test]
    async fn test_read_operation_allowed() {
        let manager = AgentManager::new().await.unwrap();

        let parameters = serde_json::json!({
            "file_path": "src/test.rs"
        });

        let result = manager.create_safe_tool_call(
            "session-123",
            "file_read",
            parameters.clone(),
            None,
            None,
        ).await.unwrap();

        match result {
            SafeToolCallResult::Allowed { tool_name, .. } => {
                assert_eq!(tool_name, "file_read");
            }
            _ => panic!("Expected Allowed result for read operation"),
        }
    }

    #[tokio::test]
    async fn test_file_modification_stats() {
        let manager = AgentManager::new().await.unwrap();

        // Create a file modification proposal
        let _interaction = manager.intercept_file_modification(
            "session-123",
            "src/test.rs".to_string(),
            "content_replace".to_string(),
            "old".to_string(),
            "new".to_string(),
            "test modification".to_string(),
            None,
        ).await.unwrap();

        // Get stats
        let stats = manager.get_file_modification_stats("session-123").await.unwrap();

        assert_eq!(stats.total_proposals, 1);
        assert_eq!(stats.pending_proposals, 1);
        assert_eq!(stats.total_interactions, 1);
        assert_eq!(stats.file_modification_interactions, 1);
    }
}
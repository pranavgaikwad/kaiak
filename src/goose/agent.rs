use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use crate::models::{FixGenerationRequest, AiSession, StreamMessage, MessageType, MessageContent};
use crate::goose::{SessionManager, GooseSessionWrapper, MessageCallback};
use tracing::{info, debug, warn, error};

/// Agent lifecycle management for Goose integration
pub struct AgentManager {
    session_manager: Arc<SessionManager>,
    active_requests: Arc<RwLock<HashMap<String, RequestState>>>,
    // TODO: This will hold the actual Goose AgentManager instance
    // goose_manager: goose::AgentManager,
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
        info!("Initializing AgentManager");

        let session_manager = Arc::new(SessionManager::new());
        let active_requests = Arc::new(RwLock::new(HashMap::new()));

        // TODO: Initialize actual Goose AgentManager
        // let goose_manager = goose::AgentManager::instance().await?;

        info!("AgentManager initialized successfully");

        Ok(Self {
            session_manager,
            active_requests,
            // goose_manager,
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
        assert!(!result.is_empty());
    }
}
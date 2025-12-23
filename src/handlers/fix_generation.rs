use anyhow::Result;
use std::sync::Arc;
use crate::models::{FixGenerationRequest, StreamMessage};
use crate::goose::AgentManager;
use crate::handlers::{StreamingHandler, StreamingConfig, ProgressPhases};
use tracing::{info, error, debug};

/// Handler for fix generation requests with agent integration and streaming
pub struct FixGenerationHandler {
    agent_manager: Arc<AgentManager>,
    streaming_handler: Arc<StreamingHandler>,
}

impl FixGenerationHandler {
    pub async fn new() -> Result<Self> {
        let agent_manager = Arc::new(AgentManager::new().await?);
        let streaming_handler = Arc::new(StreamingHandler::new(StreamingConfig::default()));
        Ok(Self {
            agent_manager,
            streaming_handler,
        })
    }

    pub async fn new_with_streaming(streaming_handler: Arc<StreamingHandler>) -> Result<Self> {
        let agent_manager = Arc::new(AgentManager::new().await?);
        Ok(Self {
            agent_manager,
            streaming_handler,
        })
    }

    /// Handle a fix generation request and return request ID with streaming receiver
    pub async fn handle_request(
        &self,
        request: &FixGenerationRequest,
    ) -> Result<(String, tokio::sync::mpsc::UnboundedReceiver<StreamMessage>)> {
        info!("Processing fix generation request for session: {}", request.session_id);
        debug!("Request contains {} incidents", request.incidents.len());

        // Validate request
        if request.incidents.is_empty() {
            self.streaming_handler.send_error(
                &request.session_id,
                Some(request.id.clone()),
                "VALIDATION_ERROR",
                "Fix generation request must contain at least one incident",
                None,
                false,
            ).await?;
            anyhow::bail!("Fix generation request must contain at least one incident");
        }

        if request.workspace_path.trim().is_empty() {
            self.streaming_handler.send_error(
                &request.session_id,
                Some(request.id.clone()),
                "VALIDATION_ERROR",
                "Workspace path cannot be empty",
                None,
                false,
            ).await?;
            anyhow::bail!("Workspace path cannot be empty");
        }

        // Create streaming session for this request
        let stream_receiver = self.streaming_handler.create_stream(request.session_id.clone()).await?;

        // Start progress tracking
        let operation_id = request.id.clone();
        self.streaming_handler
            .progress_tracker()
            .start_operation(
                operation_id.clone(),
                request.session_id.clone(),
                Some(request.id.clone()),
                "Starting fix generation process",
            )
            .await?;

        // Send initial system message
        self.streaming_handler.send_system_event(
            &request.session_id,
            Some(request.id.clone()),
            "fix_generation_started",
            "processing",
            Some(serde_json::json!({
                "request_id": request.id,
                "incident_count": request.incidents.len(),
                "workspace_path": request.workspace_path
            })),
        ).await?;

        // Update progress to analyzing phase
        self.streaming_handler
            .progress_tracker()
            .update_progress(
                &operation_id,
                ProgressPhases::ANALYZING_INCIDENTS,
                ProgressPhases::get_typical_percentage(ProgressPhases::ANALYZING_INCIDENTS),
                &format!("Analyzing {} incident(s)", request.incidents.len()),
            )
            .await?;

        // Send AI thinking message about the analysis
        self.streaming_handler.send_thinking(
            &request.session_id,
            Some(request.id.clone()),
            &format!("I need to analyze {} incident(s) in the workspace at {}. Let me examine each one to understand the migration context.",
                    request.incidents.len(), request.workspace_path),
        ).await?;

        // Process the request through the agent manager
        let agent_result = {
            // Update progress to agent processing phase
            self.streaming_handler
                .progress_tracker()
                .update_progress(
                    &operation_id,
                    ProgressPhases::CALLING_AI_AGENT,
                    ProgressPhases::get_typical_percentage(ProgressPhases::CALLING_AI_AGENT),
                    "Sending request to AI agent",
                )
                .await?;

            self.agent_manager.process_fix_request(request).await
        };

        match agent_result {
            Ok((request_id, _agent_receiver)) => {
                // Update progress to generating fixes phase
                self.streaming_handler
                    .progress_tracker()
                    .update_progress(
                        &operation_id,
                        ProgressPhases::GENERATING_FIXES,
                        ProgressPhases::get_typical_percentage(ProgressPhases::GENERATING_FIXES),
                        "Processing AI agent response and generating fixes",
                    )
                    .await?;

                // Send AI response about the fix generation
                self.streaming_handler.send_ai_response(
                    &request.session_id,
                    Some(request.id.clone()),
                    "I've analyzed the incidents and am now generating appropriate fixes based on the migration patterns and best practices.",
                    false,
                    Some(0.85),
                ).await?;

                // Complete progress tracking
                self.streaming_handler
                    .progress_tracker()
                    .complete_operation(&operation_id, "Fix generation completed successfully")
                    .await?;

                // Send completion system message
                self.streaming_handler.send_system_event(
                    &request.session_id,
                    Some(request.id.clone()),
                    "fix_generation_completed",
                    "completed",
                    Some(serde_json::json!({
                        "request_id": request_id,
                        "status": "success"
                    })),
                ).await?;

                info!("Fix generation request completed: {}", request_id);
                Ok((request_id, stream_receiver))
            }
            Err(e) => {
                error!("Failed to process fix generation request: {}", e);

                // Update progress to failed
                self.streaming_handler
                    .progress_tracker()
                    .fail_operation(&operation_id, &e.to_string())
                    .await?;

                // Send error message
                self.streaming_handler.send_error(
                    &request.session_id,
                    Some(request.id.clone()),
                    "AGENT_ERROR",
                    &format!("Agent processing failed: {}", e),
                    Some(&e.to_string()),
                    true,
                ).await?;

                // Send failure system message
                self.streaming_handler.send_system_event(
                    &request.session_id,
                    Some(request.id.clone()),
                    "fix_generation_failed",
                    "failed",
                    Some(serde_json::json!({
                        "error": e.to_string(),
                        "recoverable": true
                    })),
                ).await?;

                Err(e)
            }
        }
    }

    /// Cancel an active fix generation request
    pub async fn cancel_request(&self, request_id: &str) -> Result<()> {
        info!("Cancelling fix generation request: {}", request_id);
        self.agent_manager.cancel_request(request_id).await
    }

    /// Get the status of a fix generation request
    pub async fn get_request_status(&self, request_id: &str) -> Result<crate::goose::RequestState> {
        debug!("Getting status for request: {}", request_id);
        self.agent_manager.get_request_status(request_id).await
    }

    /// Get all requests for a specific session
    pub async fn get_session_requests(&self, session_id: &str) -> Result<Vec<crate::goose::RequestState>> {
        debug!("Getting requests for session: {}", session_id);
        self.agent_manager.get_session_requests(session_id).await
    }

    /// Get count of active requests
    pub async fn active_request_count(&self) -> usize {
        self.agent_manager.active_request_count().await
    }

    /// Cleanup old completed requests
    pub async fn cleanup_old_requests(&self, max_age: chrono::Duration) -> Result<usize> {
        info!("Cleaning up requests older than: {:?}", max_age);
        self.agent_manager.cleanup_old_requests(max_age).await
    }

    /// Get access to the underlying agent manager
    pub fn agent_manager(&self) -> &Arc<AgentManager> {
        &self.agent_manager
    }
}
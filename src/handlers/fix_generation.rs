use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use crate::models::{FixGenerationRequest, StreamMessage, Id};
use crate::goose::{AgentManager, AgentConfig};
use crate::handlers::{StreamingHandler, StreamingConfig};
use tracing::{info, error, debug, warn};

/// Handler for fix generation requests with agent integration, streaming, and approval workflow
pub struct FixGenerationHandler {
    agent_manager: Arc<AgentManager>,
    streaming_handler: Arc<StreamingHandler>,
    /// Active fix generation sessions with their approval state
    active_sessions: Arc<RwLock<HashMap<Id, FixGenerationSession>>>,
    /// Configuration for approval workflow integration
    config: FixGenerationConfig,
}

/// Configuration for fix generation with approval workflow
#[derive(Debug, Clone)]
pub struct FixGenerationConfig {
    /// Whether to require approval for all file modifications
    pub require_approval_for_modifications: bool,
    /// Timeout for approval requests in seconds
    pub approval_timeout_seconds: u32,
    /// Whether to auto-apply approved modifications
    pub auto_apply_approved_modifications: bool,
    /// Maximum concurrent approval requests per session
    pub max_concurrent_approvals: usize,
}

impl Default for FixGenerationConfig {
    fn default() -> Self {
        Self {
            require_approval_for_modifications: true,
            approval_timeout_seconds: 300, // 5 minutes
            auto_apply_approved_modifications: true,
            max_concurrent_approvals: 10,
        }
    }
}

/// State of a fix generation session with approval workflow
#[derive(Debug, Clone)]
pub struct FixGenerationSession {
    /// Session identifier
    pub session_id: Id,
    /// Original fix generation request
    pub request: FixGenerationRequest,
    /// Current status of the session
    pub status: FixGenerationStatus,
    /// File modification proposals created during this session
    pub proposals: Vec<Id>,
    /// User interactions for approvals
    pub interactions: Vec<Id>,
    /// Approved modifications ready to be applied
    pub approved_modifications: Vec<Id>,
    /// When the session was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the session was last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Status of a fix generation session
#[derive(Debug, Clone, PartialEq)]
pub enum FixGenerationStatus {
    /// Initial processing by AI agent
    Processing,
    /// Waiting for user approvals
    WaitingForApprovals,
    /// Applying approved modifications
    ApplyingModifications,
    /// Session completed successfully
    Completed,
    /// Session failed due to error
    Failed,
    /// Session cancelled by user
    Cancelled,
}

impl FixGenerationHandler {
    pub async fn new() -> Result<Self> {
        Self::new_with_config(FixGenerationConfig::default()).await
    }

    pub async fn new_with_config(config: FixGenerationConfig) -> Result<Self> {
        // Create agent manager with file modification prevention enabled
        let agent_config = AgentConfig {
            prevent_file_modifications: config.require_approval_for_modifications,
            default_approval_timeout_seconds: config.approval_timeout_seconds,
            ..Default::default()
        };
        let agent_manager = Arc::new(AgentManager::new_with_config(agent_config).await?);
        let streaming_handler = Arc::new(StreamingHandler::new(StreamingConfig::default()));

        Ok(Self {
            agent_manager,
            streaming_handler,
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            config,
        })
    }

    pub async fn new_with_streaming(streaming_handler: Arc<StreamingHandler>) -> Result<Self> {
        Self::new_with_streaming_and_config(streaming_handler, FixGenerationConfig::default()).await
    }

    pub async fn new_with_streaming_and_config(
        streaming_handler: Arc<StreamingHandler>,
        config: FixGenerationConfig,
    ) -> Result<Self> {
        // Create agent manager with file modification prevention enabled
        let agent_config = AgentConfig {
            prevent_file_modifications: config.require_approval_for_modifications,
            default_approval_timeout_seconds: config.approval_timeout_seconds,
            ..Default::default()
        };
        let agent_manager = Arc::new(AgentManager::new_with_config(agent_config).await?);

        Ok(Self {
            agent_manager,
            streaming_handler,
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            config,
        })
    }

    /// Handle a fix generation request with approval workflow integration
    pub async fn handle_request(
        &self,
        request: &FixGenerationRequest,
    ) -> Result<(String, tokio::sync::mpsc::UnboundedReceiver<StreamMessage>)> {
        info!("Processing fix generation request with approval workflow for session: {}", request.session_id);
        debug!("Request contains {} incidents, approval required: {}",
               request.incidents.len(), self.config.require_approval_for_modifications);

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

        // Create fix generation session
        let session = FixGenerationSession {
            session_id: request.session_id.clone(),
            request: request.clone(),
            status: FixGenerationStatus::Processing,
            proposals: Vec::new(),
            interactions: Vec::new(),
            approved_modifications: Vec::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        // Store session
        {
            let mut sessions = self.active_sessions.write().await;
            sessions.insert(request.id.clone(), session);
        }

        // Start progress tracking
        let operation_id = request.id.clone();
        self.streaming_handler
            .progress_tracker()
            .start_operation(
                operation_id.clone(),
                request.session_id.clone(),
                Some(request.id.clone()),
                "Starting fix generation with approval workflow",
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
                "workspace_path": request.workspace_path,
                "approval_required": self.config.require_approval_for_modifications
            })),
        ).await?;

        // Process in background by capturing all needed state
        let active_sessions_clone = self.active_sessions.clone();
        let agent_manager_clone = self.agent_manager.clone();
        let streaming_handler_clone = self.streaming_handler.clone();
        let config_clone = self.config.clone();
        let request_clone = request.clone();
        let operation_id_clone = operation_id.clone();

        tokio::spawn(async move {
            // Create a temporary handler struct to encapsulate the logic
            let temp_handler = FixGenerationHandler {
                agent_manager: agent_manager_clone,
                streaming_handler: streaming_handler_clone,
                active_sessions: active_sessions_clone.clone(),
                config: config_clone,
            };

            match temp_handler.process_fix_generation_with_approval(&request_clone, operation_id_clone).await {
                Ok(_) => {
                    info!("Fix generation with approval workflow completed: {}", request_clone.id);
                }
                Err(e) => {
                    error!("Fix generation with approval workflow failed: {}: {}", request_clone.id, e);

                    // Update session status to failed
                    {
                        let mut sessions = active_sessions_clone.write().await;
                        if let Some(session) = sessions.get_mut(&request_clone.id) {
                            session.status = FixGenerationStatus::Failed;
                            session.updated_at = chrono::Utc::now();
                        }
                    }
                }
            }
        });

        info!("Fix generation request initiated with approval workflow: {}", request.id);
        Ok((request.id.clone(), stream_receiver))
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

    /// Process fix generation with approval workflow integration
    async fn process_fix_generation_with_approval(
        &self,
        request: &FixGenerationRequest,
        operation_id: String,
    ) -> Result<()> {
        info!("Starting fix generation with approval workflow for request: {}", request.id);

        // Update session status
        {
            let mut sessions = self.active_sessions.write().await;
            if let Some(session) = sessions.get_mut(&request.id) {
                session.status = FixGenerationStatus::Processing;
                session.updated_at = chrono::Utc::now();
            }
        }

        // Update progress to analysis phase
        self.streaming_handler
            .progress_tracker()
            .update_progress(
                &operation_id,
                "analysis",
                25,
                "Analyzing migration incidents",
            )
            .await?;

        // Process each incident with the agent
        let incident_count = request.incidents.len();
        for (i, incident) in request.incidents.iter().enumerate() {
            let progress = 25 + ((i + 1) * 25 / incident_count) as u8;

            self.streaming_handler
                .progress_tracker()
                .update_progress(
                    &operation_id,
                    "analysis",
                    progress,
                    &format!("Processing incident {}/{}: {}", i + 1, incident_count, incident.description),
                )
                .await?;

            // Simulate agent processing for the incident
            self.process_incident_with_agent(&request, incident, &operation_id).await?;
        }

        // Update progress to approval phase
        self.streaming_handler
            .progress_tracker()
            .update_progress(
                &operation_id,
                "approval",
                50,
                "Waiting for user approvals",
            )
            .await?;

        // Update session status to waiting for approvals
        {
            let mut sessions = self.active_sessions.write().await;
            if let Some(session) = sessions.get_mut(&request.id) {
                session.status = FixGenerationStatus::WaitingForApprovals;
                session.updated_at = chrono::Utc::now();
            }
        }

        // Wait for all approvals to complete
        self.wait_for_approvals(&request, &operation_id).await?;

        // Update progress to application phase
        self.streaming_handler
            .progress_tracker()
            .update_progress(
                &operation_id,
                "application",
                75,
                "Applying approved modifications",
            )
            .await?;

        // Update session status to applying modifications
        {
            let mut sessions = self.active_sessions.write().await;
            if let Some(session) = sessions.get_mut(&request.id) {
                session.status = FixGenerationStatus::ApplyingModifications;
                session.updated_at = chrono::Utc::now();
            }
        }

        // Apply approved modifications
        self.apply_approved_modifications(&request, &operation_id).await?;

        // Complete the operation
        self.streaming_handler
            .progress_tracker()
            .complete_operation(
                &operation_id,
                "Fix generation with approval workflow completed successfully",
            )
            .await?;

        // Update session status to completed
        {
            let mut sessions = self.active_sessions.write().await;
            if let Some(session) = sessions.get_mut(&request.id) {
                session.status = FixGenerationStatus::Completed;
                session.updated_at = chrono::Utc::now();
            }
        }

        // Send completion notification
        self.streaming_handler.send_system_event(
            &request.session_id,
            Some(request.id.clone()),
            "fix_generation_completed",
            "completed",
            Some(serde_json::json!({
                "request_id": request.id,
                "total_incidents": request.incidents.len(),
                "workspace_path": request.workspace_path
            })),
        ).await?;

        info!("Fix generation with approval workflow completed: {}", request.id);
        Ok(())
    }

    /// Process a single incident with the agent
    async fn process_incident_with_agent(
        &self,
        request: &FixGenerationRequest,
        incident: &crate::models::Incident,
        _operation_id: &String,
    ) -> Result<()> {
        debug!("Processing incident: {} at {}:{}", incident.rule_id, incident.file_path, incident.line_number);

        // Send incident processing notification
        self.streaming_handler.send_system_event(
            &request.session_id,
            Some(request.id.clone()),
            "incident_processing",
            "analyzing",
            Some(serde_json::json!({
                "incident": {
                    "rule_id": incident.rule_id,
                    "file_path": incident.file_path,
                    "line_number": incident.line_number,
                    "severity": incident.severity,
                    "description": incident.description
                }
            })),
        ).await?;

        // Simulate AI agent analysis and potential file modification proposal
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // For high-priority incidents, create a file modification proposal
        if incident.severity.is_high_priority() {
            let proposal_result = self.agent_manager.intercept_file_modification(
                &request.session_id,
                incident.file_path.clone(),
                "incident_fix".to_string(),
                format!("// Original code with issue: {}", incident.description),
                format!("// Fixed code for rule: {}", incident.rule_id),
                format!("Fix for {} in {}", incident.rule_id, incident.file_path),
                Some((incident.line_number, incident.line_number + 1)),
            ).await?;

            // Track the proposal in the session
            {
                let mut sessions = self.active_sessions.write().await;
                if let Some(session) = sessions.get_mut(&request.id) {
                    session.proposals.push(proposal_result.id.clone());
                    session.interactions.push(proposal_result.id.clone());
                }
            }

            info!("Created file modification proposal for incident: {}", incident.rule_id);
        }

        Ok(())
    }

    /// Wait for all pending approvals to complete
    async fn wait_for_approvals(&self, request: &FixGenerationRequest, operation_id: &String) -> Result<()> {
        info!("Waiting for user approvals for request: {}", request.id);

        let max_wait_duration = tokio::time::Duration::from_secs(self.config.approval_timeout_seconds as u64);
        let check_interval = tokio::time::Duration::from_secs(5);
        let start_time = tokio::time::Instant::now();

        loop {
            // Check if we've exceeded the maximum wait time
            if start_time.elapsed() > max_wait_duration {
                warn!("Approval timeout exceeded for request: {}", request.id);
                self.streaming_handler.send_system_event(
                    &request.session_id,
                    Some(request.id.clone()),
                    "approval_timeout",
                    "timeout",
                    Some(serde_json::json!({
                        "request_id": request.id,
                        "timeout_seconds": self.config.approval_timeout_seconds
                    })),
                ).await?;
                break;
            }

            // Check approval status of all proposals
            let session_proposals = {
                let sessions = self.active_sessions.read().await;
                if let Some(session) = sessions.get(&request.id) {
                    session.proposals.clone()
                } else {
                    Vec::new()
                }
            };

            let mut all_completed = true;
            let mut approved_count = 0;
            let mut rejected_count = 0;

            for proposal_id in &session_proposals {
                if let Some(proposal) = self.agent_manager.modification_handler().get_proposal(proposal_id).await {
                    match proposal.approval_status {
                        crate::models::ApprovalStatus::Pending => {
                            if !proposal.is_expired() {
                                all_completed = false;
                            }
                        }
                        crate::models::ApprovalStatus::Approved => approved_count += 1,
                        crate::models::ApprovalStatus::Rejected => rejected_count += 1,
                        _ => {} // Already processed
                    }
                }
            }

            if all_completed {
                info!("All approvals completed - Approved: {}, Rejected: {}", approved_count, rejected_count);

                // Update approved modifications in session
                {
                    let mut sessions = self.active_sessions.write().await;
                    if let Some(session) = sessions.get_mut(&request.id) {
                        for proposal_id in &session_proposals {
                            if let Some(proposal) = self.agent_manager.modification_handler().get_proposal(proposal_id).await {
                                if proposal.approval_status == crate::models::ApprovalStatus::Approved {
                                    session.approved_modifications.push(proposal_id.clone());
                                }
                            }
                        }
                    }
                }

                self.streaming_handler.send_system_event(
                    &request.session_id,
                    Some(request.id.clone()),
                    "approvals_completed",
                    "completed",
                    Some(serde_json::json!({
                        "request_id": request.id,
                        "approved_count": approved_count,
                        "rejected_count": rejected_count,
                        "total_proposals": session_proposals.len()
                    })),
                ).await?;
                break;
            }

            // Update progress with current approval status
            let pending_count = session_proposals.len() - approved_count - rejected_count;
            let progress_message = if pending_count > 0 {
                format!("Waiting for {} pending approvals (Approved: {}, Rejected: {})",
                        pending_count, approved_count, rejected_count)
            } else {
                "Processing final approvals".to_string()
            };

            let progress_percent = 50 + ((approved_count + rejected_count) * 20 / session_proposals.len().max(1)) as u8;
            self.streaming_handler
                .progress_tracker()
                .update_progress(
                    operation_id,
                    "approval",
                    progress_percent,
                    &progress_message,
                )
                .await?;

            tokio::time::sleep(check_interval).await;
        }

        Ok(())
    }

    /// Apply all approved modifications
    async fn apply_approved_modifications(&self, request: &FixGenerationRequest, operation_id: &String) -> Result<()> {
        let approved_modifications = {
            let sessions = self.active_sessions.read().await;
            if let Some(session) = sessions.get(&request.id) {
                session.approved_modifications.clone()
            } else {
                Vec::new()
            }
        };

        if approved_modifications.is_empty() {
            info!("No approved modifications to apply for request: {}", request.id);
            return Ok(());
        }

        info!("Applying {} approved modifications for request: {}", approved_modifications.len(), request.id);

        for (i, modification_id) in approved_modifications.iter().enumerate() {
            let progress = 75 + ((i + 1) * 20 / approved_modifications.len()) as u8;

            self.streaming_handler
                .progress_tracker()
                .update_progress(
                    operation_id,
                    "application",
                    progress,
                    &format!("Applying modification {}/{}", i + 1, approved_modifications.len()),
                )
                .await?;

            // Apply the modification
            if let Err(e) = self.agent_manager.apply_approved_modification(modification_id).await {
                error!("Failed to apply modification {}: {}", modification_id, e);

                // Send error notification but continue with other modifications
                self.streaming_handler.send_system_event(
                    &request.session_id,
                    Some(request.id.clone()),
                    "modification_error",
                    "error",
                    Some(serde_json::json!({
                        "modification_id": modification_id,
                        "error": e.to_string()
                    })),
                ).await?;
            } else {
                debug!("Successfully applied modification: {}", modification_id);
            }

            // Small delay between applications
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        self.streaming_handler.send_system_event(
            &request.session_id,
            Some(request.id.clone()),
            "modifications_applied",
            "completed",
            Some(serde_json::json!({
                "request_id": request.id,
                "applied_count": approved_modifications.len()
            })),
        ).await?;

        Ok(())
    }

    /// Get access to the underlying agent manager
    pub fn agent_manager(&self) -> &Arc<AgentManager> {
        &self.agent_manager
    }
}
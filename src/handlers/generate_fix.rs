use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use validator::Validate;

use crate::models::incidents::MigrationIncident;
use crate::agents::GooseAgentManager;
use crate::KaiakResult;

/// Request type for kaiak/generate_fix endpoint
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct GenerateFixRequest {
    /// Session identifier for agent execution
    #[validate(length(min = 1, message = "Session ID cannot be empty"))]
    #[validate(custom(function = "validate_uuid_format"))]
    pub session_id: String,
    /// Array of migration incidents to process
    #[validate(length(min = 1, max = 1000, message = "Must provide 1-1000 incidents"))]
    #[validate(nested)]
    pub incidents: Vec<MigrationIncident>,
    /// Optional context for the migration process
    pub migration_context: Option<serde_json::Value>,
}

/// Response type for kaiak/generate_fix endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateFixResponse {
    /// Unique request identifier for tracking
    pub request_id: String,
    /// Session identifier
    pub session_id: String,
    /// Processing status
    pub status: GenerateFixStatus,
    /// Number of incidents being processed
    pub incident_count: usize,
    /// Request creation timestamp
    pub created_at: String,
    /// Estimated completion time if available
    pub estimated_completion: Option<String>,
}

/// Status of fix generation request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerateFixStatus {
    /// Request accepted and queued for processing
    Accepted,
    /// Processing in progress
    Processing,
    /// Processing completed successfully
    Completed,
    /// Processing failed with errors
    Failed,
    /// Request was rejected due to validation errors
    Rejected,
}

/// Handler for kaiak/generate_fix endpoint
/// Coordinates with Goose agent to process migration incidents
pub struct GenerateFixHandler {
    /// Agent manager for session coordination
    agent_manager: Arc<GooseAgentManager>,
    /// Active requests tracking
    active_requests: Arc<RwLock<std::collections::HashMap<String, GenerateFixRequest>>>,
}

impl GenerateFixHandler {
    pub fn new(agent_manager: Arc<GooseAgentManager>) -> Self {
        Self {
            agent_manager,
            active_requests: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Handle generate fix request
    pub async fn handle_generate_fix(&self, request: GenerateFixRequest) -> KaiakResult<GenerateFixResponse> {
        info!("Processing generate_fix request for session: {}", request.session_id);

        // Validate request using serde validator
        if let Err(validation_errors) = request.validate() {
            error!("Request validation failed: {:?}", validation_errors);
            let error_messages: Vec<String> = validation_errors
                .field_errors()
                .into_iter()
                .flat_map(|(field, errors)| {
                    errors.iter().map(move |error| {
                        format!("Field '{}': {}", field, error.message.as_ref().map(|m| m.as_ref()).unwrap_or("validation error"))
                    })
                })
                .collect();

            return Err(crate::KaiakError::agent(
                format!("Request validation failed: {}", error_messages.join(", ")),
                None,
            ));
        }

        // Additional custom validation
        self.validate_request(&request).await?;

        // Generate unique request ID
        let request_id = Uuid::new_v4().to_string();

        // Store request for tracking
        {
            let mut active = self.active_requests.write().await;
            active.insert(request_id.clone(), request.clone());
        }

        // Count incidents
        let incident_count = request.incidents.len();
        info!("Processing {} migration incidents", incident_count);

        // For User Story 1, we implement the API surface but defer actual agent execution to User Story 3
        // This creates a proper response structure and validates inputs
        match self.initiate_agent_processing(&request_id, &request).await {
            Ok(_) => {
                info!("Generate fix request {} initiated successfully", request_id);
                Ok(GenerateFixResponse {
                    request_id,
                    session_id: request.session_id,
                    status: GenerateFixStatus::Accepted,
                    incident_count,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    estimated_completion: self.estimate_completion_time(incident_count),
                })
            }
            Err(e) => {
                error!("Failed to initiate generate fix request {}: {}", request_id, e);

                // Remove from active requests on failure
                {
                    let mut active = self.active_requests.write().await;
                    active.remove(&request_id);
                }

                Ok(GenerateFixResponse {
                    request_id,
                    session_id: request.session_id,
                    status: GenerateFixStatus::Failed,
                    incident_count,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    estimated_completion: None,
                })
            }
        }
    }

    /// Get status of active requests
    pub async fn get_active_request_count(&self) -> usize {
        let active = self.active_requests.read().await;
        active.len()
    }

    /// Validate generate fix request
    async fn validate_request(&self, request: &GenerateFixRequest) -> KaiakResult<()> {
        // Validate session ID format
        if request.session_id.is_empty() {
            return Err(crate::KaiakError::session("Session ID cannot be empty".to_string(), Some(request.session_id.clone())));
        }

        // Validate UUID format for session ID
        if Uuid::parse_str(&request.session_id).is_err() {
            return Err(crate::KaiakError::session("Session ID must be a valid UUID".to_string(), Some(request.session_id.clone())));
        }

        // Validate incidents array
        if request.incidents.is_empty() {
            return Err(crate::KaiakError::agent("At least one incident must be provided".to_string(), None));
        }

        // Validate individual incidents
        for (i, incident) in request.incidents.iter().enumerate() {
            if incident.id.is_empty() {
                return Err(crate::KaiakError::agent(format!("Incident {} has empty ID", i), None));
            }
            if incident.rule_id.is_empty() {
                return Err(crate::KaiakError::agent(format!("Incident {} has empty rule_id", i), None));
            }
        }

        // Check for reasonable incident count (prevent overload)
        if request.incidents.len() > 1000 {
            return Err(crate::KaiakError::ResourceExhausted("Too many incidents in single request (max: 1000)".to_string()));
        }

        Ok(())
    }

    /// Initiate agent processing with Goose session management
    /// User Story 2: Session creation/lookup integration
    /// User Story 3: Full agent execution (upcoming)
    async fn initiate_agent_processing(&self, request_id: &str, request: &GenerateFixRequest) -> KaiakResult<()> {
        debug!("Initiating agent processing for request: {}", request_id);

        // User Story 2: Use Goose SessionManager for session management
        // Try to lock the session to prevent concurrent access
        match self.agent_manager.lock_session(&request.session_id).await {
            Ok(_) => {
                debug!("Successfully locked session: {}", request.session_id);
            }
            Err(e) => {
                error!("Failed to lock session {}: {}", request.session_id, e);
                return Err(e);
            }
        }

        // Create a default configuration for session creation if needed
        let config = crate::models::configuration::AgentConfiguration::default();

        // Get or create session using Goose SessionManager
        let session_info = match self.agent_manager.get_or_create_session(&request.session_id, &config).await {
            Ok(session_info) => {
                debug!("Session ready for processing: {} (workspace: {:?})",
                       request.session_id,
                       session_info.configuration.workspace.working_dir);
                session_info
            }
            Err(e) => {
                error!("Failed to get or create session {}: {}", request.session_id, e);
                // Unlock session on failure
                if let Err(unlock_err) = self.agent_manager.unlock_session(&request.session_id).await {
                    warn!("Failed to unlock session after error: {}", unlock_err);
                }
                return Err(e);
            }
        };

        // Validate workspace is accessible using actual session configuration
        let workspace_path = &session_info.configuration.workspace.working_dir;
        if !workspace_path.exists() {
            error!("Workspace directory does not exist: {:?}", workspace_path);
            self.agent_manager.unlock_session(&request.session_id).await.ok();
            return Err(crate::KaiakError::workspace(
                "Workspace directory does not exist".to_string(),
                Some(workspace_path.to_string_lossy().to_string())
            ));
        }

        // Log incident processing preparation
        debug!("Prepared {} incidents for processing in workspace: {:?}",
               request.incidents.len(), workspace_path);

        // User Story 2 Achievement: Session is now managed by Goose SessionManager
        // User Story 3: Will implement actual agent execution with streaming responses
        info!("Request {} accepted with Goose session management", request_id);

        // For User Story 2, we unlock the session as we're not actually processing yet
        // User Story 3 will keep the lock during actual agent execution
        if let Err(unlock_err) = self.agent_manager.unlock_session(&request.session_id).await {
            warn!("Failed to unlock session after preparation: {}", unlock_err);
        }

        Ok(())
    }

    /// Estimate completion time based on incident count
    fn estimate_completion_time(&self, incident_count: usize) -> Option<String> {
        // Simple estimation: 30 seconds per incident + base time
        let base_seconds = 60; // 1 minute base processing time
        let per_incident_seconds = 30;
        let total_seconds = base_seconds + (incident_count * per_incident_seconds);

        let estimated_time = chrono::Utc::now() + chrono::Duration::seconds(total_seconds as i64);
        Some(estimated_time.to_rfc3339())
    }

    /// Cancel a generate fix request
    pub async fn cancel_request(&self, request_id: &str) -> KaiakResult<bool> {
        let mut active = self.active_requests.write().await;
        Ok(active.remove(request_id).is_some())
    }
}

/// Custom validation function for UUID format
fn validate_uuid_format(session_id: &str) -> Result<(), validator::ValidationError> {
    if Uuid::parse_str(session_id).is_err() {
        return Err(validator::ValidationError::new("invalid_uuid_format"));
    }
    Ok(())
}
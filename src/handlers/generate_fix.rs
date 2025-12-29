use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use validator::Validate;

use crate::models::{configuration::AgentConfig, incidents::MigrationIncident};
use crate::agent::GooseAgentManager;
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
    pub agent_config: AgentConfig,
}

/// Response type for kaiak/generate_fix endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateFixResponse {
    pub request_id: String,
    pub session_id: String,
    pub created_at: String,
}

/// Handler for kaiak/generate_fix endpoint
/// Coordinates with Goose agent to process migration incidents
pub struct GenerateFixHandler {
    /// Agent manager for session coordination
    agent_manager: Arc<GooseAgentManager>,
    /// Active requests tracking
    active_requests: Arc<RwLock<std::collections::HashMap<String, GenerateFixRequest>>>,
    /// Base configuration of the server
    base_config: Arc<crate::models::configuration::BaseConfig>,
}

impl GenerateFixHandler {
    pub fn new(agent_manager: Arc<GooseAgentManager>, base_config: Arc<crate::models::configuration::BaseConfig>) -> Self {
        Self {
            agent_manager,
            active_requests: Arc::new(RwLock::new(std::collections::HashMap::new())),
            base_config,
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

        let request_id = Uuid::new_v4().to_string();
        {
            let mut active = self.active_requests.write().await;
            active.insert(request_id.clone(), request.clone());
        }

        info!("Processing {} migration incidents", request.incidents.len());

        match self.initiate_agent_processing(&request_id, &request).await {
            Ok(_) => {
                info!("Generate fix request {} initiated successfully", request_id);
                Ok(GenerateFixResponse {
                    request_id,
                    session_id: request.session_id,
                    created_at: chrono::Utc::now().to_rfc3339(),
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
                    created_at: chrono::Utc::now().to_rfc3339(),
                })
            }
        }
    }

    /// Get status of active requests
    pub async fn get_active_request_count(&self) -> usize {
        let active = self.active_requests.read().await;
        active.len()
    }

    /// Initiate agent processing with Goose session management
    async fn initiate_agent_processing(&self, request_id: &str, request: &GenerateFixRequest) -> KaiakResult<()> {
        debug!("Initiating agent processing for request: {}", request_id);

        match self.agent_manager.lock_session(&request.session_id).await {
            Ok(_) => {
                debug!("Successfully locked session: {}", request.session_id);
            }
            Err(e) => {
                error!("Failed to lock session {}: {}", request.session_id, e);
                return Err(e);
            }
        }

        // Get or create session using Goose SessionManager
        let session_info = match self.agent_manager.get_or_create_session(&request.session_id, &request.agent_config).await {
            Ok(session_info) => {
                debug!("Session ready for processing: {} (workspace: {:?})",
                       request.session_id,
                       request.agent_config.workspace);
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
        let agent = self.agent_manager.create_agent(&request.session_id, &request.agent_config).await?;


        if let Err(unlock_err) = self.agent_manager.unlock_session(&request.session_id).await {
            warn!("Failed to unlock session after preparation: {}", unlock_err);
        }

        Ok(())
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
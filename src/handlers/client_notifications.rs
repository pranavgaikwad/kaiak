//! Client notification handler for receiving messages from IDE clients.
//!
//! This module handles all client-to-server messages via the `kaiak/client/user_message`
//! JSON-RPC method. Different `kind` values route to different handlers.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use validator::Validate;
use chrono::{DateTime, Utc};

use crate::agent::GooseAgentManager;
use crate::KaiakResult;
use super::interaction_manager::InteractionManager;
use goose::permission::Permission;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClientNotificationKind {
    /// Generic user input (e.g., follow-up messages)
    UserInput,
    /// Control signals (e.g., cancel, pause)
    ControlSignal,
    /// Response to a tool confirmation request
    ToolConfirmation,
    /// Response to an elicitation request
    ElicitationResponse,
}

/// Request type for kaiak/client/user_message method
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ClientNotificationRequest {
    #[validate(custom(function = "validate_session_id"))]
    pub session_id: String,
    pub kind: ClientNotificationKind,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}

/// Response type for client notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientNotificationResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfirmationPayload {
    pub request_id: String,
    /// The action: "allow_once", "always_allow", or "deny"
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElicitationResponsePayload {
    pub request_id: String,
    pub user_data: serde_json::Value,
}

/// Custom validation for session ID format
fn validate_session_id(session_id: &str) -> Result<(), validator::ValidationError> {
    if session_id.is_empty() {
        let mut error = validator::ValidationError::new("empty_session_id");
        error.message = Some("Session ID cannot be empty".into());
        return Err(error);
    }
    Ok(())
}

/// Handler for client-to-server notifications
pub struct ClientNotificationHandler {
    agent_manager: Arc<GooseAgentManager>,
    interaction_manager: Arc<InteractionManager>,
}

impl ClientNotificationHandler {
    pub fn new(
        agent_manager: Arc<GooseAgentManager>,
        interaction_manager: Arc<InteractionManager>,
    ) -> Self {
        Self {
            agent_manager,
            interaction_manager,
        }
    }

    /// Handle incoming client notification
    pub async fn handle_notification(
        &self,
        request: ClientNotificationRequest,
    ) -> KaiakResult<ClientNotificationResponse> {
        debug!(
            "Received client notification: session_id={}, kind={:?}",
            request.session_id, request.kind
        );

        // Validate request
        if let Err(validation_errors) = request.validate() {
            warn!(
                "Client notification validation failed: {:?}",
                validation_errors
            );
            return Ok(ClientNotificationResponse {
                success: false,
                message: format!("Validation failed: {}", validation_errors),
                notification_id: None,
            });
        }

        // Validate session exists (except for some notification types that may not need it)
        let session_exists = self.agent_manager.session_exists(&request.session_id).await;
        if !session_exists {
            warn!("Session ID not found: {}", request.session_id);
            return Ok(ClientNotificationResponse {
                success: false,
                message: format!("Session ID '{}' not found or invalid", request.session_id),
                notification_id: None,
            });
        }

        // Validate payload size (1MB limit)
        if let Some(ref payload) = request.payload {
            match serde_json::to_string(payload) {
                Ok(payload_str) => {
                    let payload_size = payload_str.len();
                    if payload_size > 1024 * 1024 {
                        warn!("Payload exceeds 1MB limit: {} bytes", payload_size);
                        return Ok(ClientNotificationResponse {
                            success: false,
                            message: "Notification payload exceeds 1MB size limit".to_string(),
                            notification_id: None,
                        });
                    }
                }
                Err(e) => {
                    error!("Failed to serialize payload for size check: {}", e);
                    return Ok(ClientNotificationResponse {
                        success: false,
                        message: "Failed to process payload".to_string(),
                        notification_id: None,
                    });
                }
            }
        }

        match request.kind {
            ClientNotificationKind::ToolConfirmation => {
                self.handle_tool_confirmation(&request).await
            }
            ClientNotificationKind::ElicitationResponse => {
                self.handle_elicitation_response(&request).await
            }
            ClientNotificationKind::UserInput => self.handle_user_input(&request).await,
            ClientNotificationKind::ControlSignal => self.handle_control_signal(&request).await,
        }
    }

    async fn handle_tool_confirmation(
        &self,
        request: &ClientNotificationRequest,
    ) -> KaiakResult<ClientNotificationResponse> {
        let payload = match &request.payload {
            Some(p) => p,
            None => {
                return Ok(ClientNotificationResponse {
                    success: false,
                    message: "Payload required for tool_confirmation".to_string(),
                    notification_id: None,
                });
            }
        };

        let confirmation: ToolConfirmationPayload = match serde_json::from_value(payload.clone()) {
            Ok(c) => c,
            Err(e) => {
                return Ok(ClientNotificationResponse {
                    success: false,
                    message: format!("Invalid tool_confirmation payload: {}", e),
                    notification_id: None,
                });
            }
        };

        let permission = match confirmation.action.as_str() {
            "allow_once" => Permission::AllowOnce,
            "always_allow" => Permission::AlwaysAllow,
            "deny" => Permission::DenyOnce,
            other => {
                return Ok(ClientNotificationResponse {
                    success: false,
                    message: format!(
                        "Invalid action '{}'. Must be 'allow_once', 'always_allow', or 'deny'",
                        other
                    ),
                    notification_id: None,
                });
            }
        };

        info!(
            "Processing tool confirmation: request_id={}, action={}",
            confirmation.request_id, confirmation.action
        );

        match self
            .interaction_manager
            .submit_confirmation(&confirmation.request_id, permission)
            .await
        {
            Ok(()) => Ok(ClientNotificationResponse {
                success: true,
                message: "Tool confirmation submitted".to_string(),
                notification_id: Some(Uuid::new_v4().to_string()),
            }),
            Err(e) => {
                warn!("Failed to submit tool confirmation: {}", e);
                Ok(ClientNotificationResponse {
                    success: false,
                    message: e,
                    notification_id: None,
                })
            }
        }
    }

    async fn handle_elicitation_response(
        &self,
        request: &ClientNotificationRequest,
    ) -> KaiakResult<ClientNotificationResponse> {
        let payload = match &request.payload {
            Some(p) => p,
            None => {
                return Ok(ClientNotificationResponse {
                    success: false,
                    message: "Payload required for elicitation_response".to_string(),
                    notification_id: None,
                });
            }
        };

        let response: ElicitationResponsePayload = match serde_json::from_value(payload.clone()) {
            Ok(r) => r,
            Err(e) => {
                return Ok(ClientNotificationResponse {
                    success: false,
                    message: format!("Invalid elicitation_response payload: {}", e),
                    notification_id: None,
                });
            }
        };

        info!(
            "Processing elicitation response: request_id={}",
            response.request_id
        );

        match self
            .interaction_manager
            .submit_elicitation(&response.request_id, response.user_data)
            .await
        {
            Ok(()) => Ok(ClientNotificationResponse {
                success: true,
                message: "Elicitation response submitted".to_string(),
                notification_id: Some(Uuid::new_v4().to_string()),
            }),
            Err(e) => {
                warn!("Failed to submit elicitation response: {}", e);
                Ok(ClientNotificationResponse {
                    success: false,
                    message: e,
                    notification_id: None,
                })
            }
        }
    }

    /// Handle generic user input
    async fn handle_user_input(
        &self,
        request: &ClientNotificationRequest,
    ) -> KaiakResult<ClientNotificationResponse> {
        // Validate payload exists for user input
        match &request.payload {
            Some(payload) if payload.is_null() => {
                warn!("User input with null payload");
                return Ok(ClientNotificationResponse {
                    success: false,
                    message: "User input must have non-null payload".to_string(),
                    notification_id: None,
                });
            }
            None => {
                warn!("User input with missing payload");
                return Ok(ClientNotificationResponse {
                    success: false,
                    message: "User input must have payload content".to_string(),
                    notification_id: None,
                });
            }
            _ => {}
        }

        let notification_id = Uuid::new_v4().to_string();

        info!(
            "User input received - id: {}, session: {}, payload_size: {}",
            notification_id,
            request.session_id,
            request
                .payload
                .as_ref()
                .map(|p| serde_json::to_string(p).map(|s| s.len()).unwrap_or(0))
                .unwrap_or(0)
        );

        // TODO: Route user input to agent for processing
        // For now, just acknowledge receipt

        Ok(ClientNotificationResponse {
            success: true,
            message: "User input received".to_string(),
            notification_id: Some(notification_id),
        })
    }

    /// Handle control signals (cancel, pause, etc.)
    async fn handle_control_signal(
        &self,
        request: &ClientNotificationRequest,
    ) -> KaiakResult<ClientNotificationResponse> {
        let notification_id = Uuid::new_v4().to_string();

        info!(
            "Control signal received - id: {}, session: {}",
            notification_id, request.session_id
        );

        // TODO: Process control signals (cancel in-progress operations, etc.)

        Ok(ClientNotificationResponse {
            success: true,
            message: "Control signal received".to_string(),
            notification_id: Some(notification_id),
        })
    }
}

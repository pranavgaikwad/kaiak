use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use validator::Validate;
use chrono::{DateTime, Utc};

use crate::agent::GooseAgentManager;
use crate::KaiakResult;

/// Request type for kaiak/client/user_message method
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ClientNotificationRequest {
    #[validate(custom(function = "validate_session_id"))]
    pub session_id: String,
    #[validate(custom(function = "validate_message_type"))]
    pub message_type: String,
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

/// Custom validation for message type
fn validate_message_type(message_type: &str) -> Result<(), validator::ValidationError> {
    match message_type {
        "user_input" | "control_signal" => Ok(()),
        _ => {
            let mut error = validator::ValidationError::new("invalid_message_type");
            error.message = Some("Message type must be 'user_input' or 'control_signal'".into());
            Err(error)
        }
    }
}

/// Custom validation for session ID format
fn validate_session_id(session_id: &str) -> Result<(), validator::ValidationError> {
    // Session ID must be non-empty
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
}

impl ClientNotificationHandler {
    /// Create a new client notification handler
    pub fn new(agent_manager: Arc<GooseAgentManager>) -> Self {
        Self { agent_manager }
    }

    /// Handle incoming client notification
    pub async fn handle_notification(
        &self,
        request: ClientNotificationRequest,
    ) -> KaiakResult<ClientNotificationResponse> {
        debug!("Received client notification: session_id={}, message_type={}",
               request.session_id, request.message_type);

        // Validate request
        if let Err(validation_errors) = request.validate() {
            warn!("Client notification validation failed: {:?}", validation_errors);
            return Ok(ClientNotificationResponse {
                success: false,
                message: format!("Validation failed: {}", validation_errors),
                notification_id: None,
            });
        }

        // Validate session exists
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
                    if payload_size > 1024 * 1024 { // 1MB
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

        // Additional validation: Check for empty or null payload when message type requires content
        if request.message_type == "user_input" {
            match &request.payload {
                Some(payload) => {
                    if payload.is_null() {
                        warn!("User input message type with null payload");
                        return Ok(ClientNotificationResponse {
                            success: false,
                            message: "User input messages must have non-null payload".to_string(),
                            notification_id: None,
                        });
                    }
                }
                None => {
                    warn!("User input message type with missing payload");
                    return Ok(ClientNotificationResponse {
                        success: false,
                        message: "User input messages must have payload content".to_string(),
                        notification_id: None,
                    });
                }
            }
        }

        // Generate notification ID for tracking
        let notification_id = Uuid::new_v4().to_string();

        // Log the notification receipt (but do not forward to agent per requirements)
        info!(
            "Client notification received and validated - id: {}, session: {}, type: {}, payload_size: {}",
            notification_id,
            request.session_id,
            request.message_type,
            request.payload.as_ref()
                .map(|p| serde_json::to_string(p).map(|s| s.len()).unwrap_or(0))
                .unwrap_or(0)
        );

        Ok(ClientNotificationResponse {
            success: true,
            message: "Notification received and validated".to_string(),
            notification_id: Some(notification_id),
        })
    }
}
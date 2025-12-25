use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInteractionRequest {
    pub session_id: String,
    pub interaction_id: String,
    pub request_id: Option<String>,
    pub interaction_type: UserInteractionType,
    pub prompt: String,
    pub context: InteractionContext,
    pub response_options: ResponseOptions,
    pub timeout: Option<u32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionContext {
    pub file_path: Option<PathBuf>,
    pub code_snippet: Option<String>,
    pub incident_id: Option<String>,
    pub tool_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseOptions {
    Confirmation {
        default_response: bool,
    },
    Choice {
        options: Vec<String>,
        allow_custom: bool,
        default_index: Option<usize>,
    },
    TextInput {
        placeholder: Option<String>,
        validation_pattern: Option<String>,
        max_length: Option<u32>,
    },
    FileApproval {
        proposal_id: String,
        diff_preview: String,
        allow_edit: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserInteractionType {
    Confirmation,
    Choice,
    TextInput,
    FileApproval,
    ToolPermission,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInteractionResponse {
    pub session_id: String,
    pub interaction_id: String,
    pub response_type: UserResponseType,
    pub response_data: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserResponseType {
    Approved,
    Denied,
    Custom,
    Timeout,
    Cancelled,
}

impl UserInteractionRequest {
    pub fn new(
        session_id: String,
        interaction_type: UserInteractionType,
        prompt: String,
        response_options: ResponseOptions,
    ) -> Self {
        Self {
            session_id,
            interaction_id: uuid::Uuid::new_v4().to_string(),
            request_id: None,
            interaction_type,
            prompt,
            context: InteractionContext::default(),
            response_options,
            timeout: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_context(mut self, context: InteractionContext) -> Self {
        self.context = context;
        self
    }

    pub fn with_timeout(mut self, timeout_seconds: u32) -> Self {
        self.timeout = Some(timeout_seconds);
        self
    }

    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }
}

impl Default for InteractionContext {
    fn default() -> Self {
        Self {
            file_path: None,
            code_snippet: None,
            incident_id: None,
            tool_name: None,
        }
    }
}

impl UserInteractionResponse {
    pub fn new(
        session_id: String,
        interaction_id: String,
        response_type: UserResponseType,
        response_data: serde_json::Value,
    ) -> Self {
        Self {
            session_id,
            interaction_id,
            response_type,
            response_data,
            timestamp: Utc::now(),
        }
    }

    pub fn approved(session_id: String, interaction_id: String) -> Self {
        Self::new(
            session_id,
            interaction_id,
            UserResponseType::Approved,
            serde_json::json!(true),
        )
    }

    pub fn denied(session_id: String, interaction_id: String) -> Self {
        Self::new(
            session_id,
            interaction_id,
            UserResponseType::Denied,
            serde_json::json!(false),
        )
    }

    pub fn custom(
        session_id: String,
        interaction_id: String,
        data: serde_json::Value,
    ) -> Self {
        Self::new(
            session_id,
            interaction_id,
            UserResponseType::Custom,
            data,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interaction_request_creation() {
        let request = UserInteractionRequest::new(
            "session-123".to_string(),
            UserInteractionType::Confirmation,
            "Do you approve this change?".to_string(),
            ResponseOptions::Confirmation {
                default_response: false,
            },
        );

        assert_eq!(request.session_id, "session-123");
        assert!(!request.interaction_id.is_empty());
        assert!(request.request_id.is_none());
        assert!(request.timeout.is_none());
    }

    #[test]
    fn test_interaction_response_approved() {
        let response = UserInteractionResponse::approved(
            "session-123".to_string(),
            "interaction-456".to_string(),
        );

        assert_eq!(response.session_id, "session-123");
        assert_eq!(response.interaction_id, "interaction-456");
        assert!(matches!(response.response_type, UserResponseType::Approved));
        assert_eq!(response.response_data, serde_json::json!(true));
    }

    #[test]
    fn test_interaction_response_custom() {
        let custom_data = serde_json::json!({
            "choice": "option_2",
            "comment": "This looks good"
        });

        let response = UserInteractionResponse::custom(
            "session-123".to_string(),
            "interaction-456".to_string(),
            custom_data.clone(),
        );

        assert!(matches!(response.response_type, UserResponseType::Custom));
        assert_eq!(response.response_data, custom_data);
    }

    #[test]
    fn test_response_options_serialization() {
        let options = ResponseOptions::Choice {
            options: vec!["Yes".to_string(), "No".to_string()],
            allow_custom: true,
            default_index: Some(0),
        };

        let serialized = serde_json::to_string(&options).unwrap();
        let deserialized: ResponseOptions = serde_json::from_str(&serialized).unwrap();

        match deserialized {
            ResponseOptions::Choice { options, allow_custom, default_index } => {
                assert_eq!(options.len(), 2);
                assert!(allow_custom);
                assert_eq!(default_index, Some(0));
            }
            _ => panic!("Expected Choice variant"),
        }
    }
}
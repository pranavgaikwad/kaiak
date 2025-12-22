use serde::{Deserialize, Serialize};
use super::{Id, Timestamp, Status, Metadata};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSession {
    pub id: Id,
    pub goose_session_id: Option<String>,
    pub status: SessionStatus,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub configuration: SessionConfiguration,
    pub active_request_id: Option<Id>,
    pub message_count: u32,
    pub error_count: u32,
    #[serde(default)]
    pub metadata: Metadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Created,
    Initializing,
    Ready,
    Processing,
    Completed,
    Error,
    Terminated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfiguration {
    pub workspace_path: String,
    pub session_name: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub timeout: Option<u32>,
    pub max_turns: Option<u32>,
    #[serde(default)]
    pub custom: Metadata,
}

impl AiSession {
    pub fn new(workspace_path: String, session_name: Option<String>) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            goose_session_id: None,
            status: SessionStatus::Created,
            created_at: now.clone(),
            updated_at: now,
            configuration: SessionConfiguration {
                workspace_path,
                session_name,
                provider: None,
                model: None,
                timeout: None,
                max_turns: None,
                custom: Metadata::new(),
            },
            active_request_id: None,
            message_count: 0,
            error_count: 0,
            metadata: Metadata::new(),
        }
    }

    pub fn update_status(&mut self, status: SessionStatus) {
        self.status = status;
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    pub fn increment_message_count(&mut self) {
        self.message_count += 1;
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    pub fn increment_error_count(&mut self) {
        self.error_count += 1;
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    pub fn is_active(&self) -> bool {
        matches!(self.status, SessionStatus::Ready | SessionStatus::Processing)
    }
}
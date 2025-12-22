use serde::{Deserialize, Serialize};
use super::{Id, Timestamp, Metadata};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInteraction {
    pub id: Id,
    pub session_id: Id,
    pub interaction_type: InteractionType,
    pub prompt: String,
    pub request_data: InteractionRequestData,
    pub response_data: Option<InteractionResponseData>,
    pub status: InteractionStatus,
    pub timeout: u32,
    pub created_at: Timestamp,
    pub responded_at: Option<Timestamp>,
    #[serde(default)]
    pub metadata: Metadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionType {
    Approval,
    Choice,
    Input,
    Confirmation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum InteractionRequestData {
    #[serde(rename = "approval")]
    Approval {
        default_choice: Option<bool>,
        auto_approve: bool,
    },
    #[serde(rename = "choice")]
    Choice {
        options: Vec<String>,
        multiple: bool,
        default_indices: Vec<usize>,
    },
    #[serde(rename = "input")]
    Input {
        placeholder: Option<String>,
        validation: Option<String>,
        multiline: bool,
    },
    #[serde(rename = "confirmation")]
    Confirmation {
        default_acknowledged: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum InteractionResponseData {
    #[serde(rename = "approval")]
    Approval { approved: bool },
    #[serde(rename = "choice")]
    Choice { selected_indices: Vec<usize> },
    #[serde(rename = "input")]
    Input { text: String },
    #[serde(rename = "confirmation")]
    Confirmation { acknowledged: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InteractionStatus {
    Pending,
    Responded,
    Timeout,
    Cancelled,
    Processed,
    Expired,
}

impl UserInteraction {
    pub fn new(
        session_id: Id,
        interaction_type: InteractionType,
        prompt: String,
        request_data: InteractionRequestData,
        timeout: u32,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id,
            interaction_type,
            prompt,
            request_data,
            response_data: None,
            status: InteractionStatus::Pending,
            timeout,
            created_at: chrono::Utc::now().to_rfc3339(),
            responded_at: None,
            metadata: Metadata::new(),
        }
    }

    pub fn respond(&mut self, response_data: InteractionResponseData) {
        self.response_data = Some(response_data);
        self.status = InteractionStatus::Responded;
        self.responded_at = Some(chrono::Utc::now().to_rfc3339());
    }

    pub fn timeout(&mut self) {
        self.status = InteractionStatus::Timeout;
    }

    pub fn cancel(&mut self) {
        self.status = InteractionStatus::Cancelled;
    }

    pub fn is_pending(&self) -> bool {
        self.status == InteractionStatus::Pending
    }

    pub fn is_responded(&self) -> bool {
        self.status == InteractionStatus::Responded
    }
}
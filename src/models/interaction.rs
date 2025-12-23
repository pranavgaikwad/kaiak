use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::{Id, Timestamp, Metadata, Identifiable};

/// User interaction for approval workflows
///
/// Enhanced model for User Story 3: Interactive File Modification Approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInteraction {
    /// Unique identifier for this interaction
    pub id: Id,

    /// Session this interaction belongs to
    pub session_id: Option<Id>,

    /// Type of interaction being requested
    pub interaction_type: InteractionType,

    /// Human-readable prompt for the user
    pub prompt: String,

    /// ID of the proposal this interaction relates to (if applicable)
    pub proposal_id: Option<Id>,

    /// Timeout in seconds for this interaction
    pub timeout_seconds: Option<u32>,

    /// Legacy timeout field for backwards compatibility
    pub timeout: Option<u32>,

    /// When this interaction was created
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// When this interaction expires
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,

    /// When the user responded (if they did)
    pub responded_at: Option<chrono::DateTime<chrono::Utc>>,

    /// User's response to this interaction
    pub response: Option<InteractionResponse>,

    /// Legacy request data for backwards compatibility
    pub request_data: Option<InteractionRequestData>,

    /// Legacy response data for backwards compatibility
    pub response_data: Option<InteractionResponseData>,

    /// Current status of this interaction
    pub status: InteractionStatus,

    /// Additional metadata
    #[serde(default)]
    pub metadata: Option<Metadata>,
}

/// Types of user interactions (enhanced for file modification approval)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InteractionType {
    /// Request approval for file modifications
    FileModificationApproval,
    /// Legacy approval type
    Approval,
    /// Multiple choice selection
    Choice,
    /// Text input request
    Input,
    /// Simple confirmation
    Confirmation,
}

/// User response to an interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionResponse {
    /// Whether the user approved (for approval interactions)
    pub approved: Option<bool>,
    /// Text response (for input interactions)
    pub text: Option<String>,
    /// Selected choices (for choice interactions)
    pub choices: Option<Vec<String>>,
    /// Optional comment from the user
    pub comment: Option<String>,
    /// When the response was given
    pub responded_at: chrono::DateTime<chrono::Utc>,
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
    /// Create a new user interaction (enhanced version)
    pub fn new(
        interaction_type: InteractionType,
        prompt: String,
        proposal_id: Option<Id>,
        timeout_seconds: Option<u32>,
    ) -> Self {
        let now = chrono::Utc::now();
        let expires_at = timeout_seconds.map(|t| now + chrono::Duration::seconds(t as i64));

        Self {
            id: Self::generate_id(),
            session_id: None,
            interaction_type,
            prompt,
            proposal_id,
            timeout_seconds,
            timeout: timeout_seconds, // For backwards compatibility
            created_at: now,
            expires_at,
            responded_at: None,
            response: None,
            request_data: None,
            response_data: None,
            status: InteractionStatus::Pending,
            metadata: None,
        }
    }

    /// Create a file modification approval interaction
    pub fn new_file_modification_approval(
        prompt: String,
        proposal_id: Id,
        timeout_seconds: u32,
    ) -> Self {
        Self::new(
            InteractionType::FileModificationApproval,
            prompt,
            Some(proposal_id),
            Some(timeout_seconds),
        )
    }

    /// Legacy constructor for backwards compatibility
    pub fn new_legacy(
        session_id: Id,
        interaction_type: InteractionType,
        prompt: String,
        request_data: InteractionRequestData,
        timeout: u32,
    ) -> Self {
        let now = chrono::Utc::now();

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: Some(session_id),
            interaction_type,
            prompt,
            proposal_id: None,
            timeout_seconds: Some(timeout),
            timeout: Some(timeout),
            created_at: now,
            expires_at: Some(now + chrono::Duration::seconds(timeout as i64)),
            responded_at: None,
            response: None,
            request_data: Some(request_data),
            response_data: None,
            status: InteractionStatus::Pending,
            metadata: None,
        }
    }

    /// Respond to this interaction
    pub fn respond(&mut self, response: InteractionResponse) {
        self.response = Some(response);
        self.status = InteractionStatus::Responded;
        self.responded_at = Some(chrono::Utc::now());
    }

    /// Respond with approval
    pub fn approve(&mut self, comment: Option<String>) {
        let response = InteractionResponse {
            approved: Some(true),
            text: None,
            choices: None,
            comment,
            responded_at: chrono::Utc::now(),
        };
        self.respond(response);
    }

    /// Respond with rejection
    pub fn reject(&mut self, comment: Option<String>) {
        let response = InteractionResponse {
            approved: Some(false),
            text: None,
            choices: None,
            comment,
            responded_at: chrono::Utc::now(),
        };
        self.respond(response);
    }

    /// Legacy response method for backwards compatibility
    pub fn respond_legacy(&mut self, response_data: InteractionResponseData) {
        self.response_data = Some(response_data);
        self.status = InteractionStatus::Responded;
        self.responded_at = Some(chrono::Utc::now());
    }

    /// Mark this interaction as timed out
    pub fn timeout(&mut self) {
        self.status = InteractionStatus::Timeout;
        self.responded_at = Some(chrono::Utc::now());
    }

    /// Cancel this interaction
    pub fn cancel(&mut self) {
        self.status = InteractionStatus::Cancelled;
        self.responded_at = Some(chrono::Utc::now());
    }

    /// Mark as processed
    pub fn process(&mut self) {
        self.status = InteractionStatus::Processed;
    }

    /// Mark as expired
    pub fn expire(&mut self) {
        self.status = InteractionStatus::Expired;
        self.responded_at = Some(chrono::Utc::now());
    }

    /// Check if this interaction is pending
    pub fn is_pending(&self) -> bool {
        self.status == InteractionStatus::Pending
    }

    /// Check if user has responded
    pub fn is_responded(&self) -> bool {
        self.status == InteractionStatus::Responded
    }

    /// Check if this interaction has timed out
    pub fn is_timeout(&self) -> bool {
        self.status == InteractionStatus::Timeout
    }

    /// Check if this interaction is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            chrono::Utc::now() > expires_at
        } else {
            false
        }
    }

    /// Get time remaining before expiry
    pub fn time_until_expiry(&self) -> Option<chrono::Duration> {
        if let Some(expires_at) = self.expires_at {
            let now = chrono::Utc::now();
            if now < expires_at {
                Some(expires_at - now)
            } else {
                Some(chrono::Duration::zero())
            }
        } else {
            None
        }
    }

    /// Check if this interaction should timeout and mark it if so
    pub fn check_and_mark_timeout(&mut self) -> bool {
        if self.is_expired() && self.is_pending() {
            self.timeout();
            true
        } else {
            false
        }
    }

    /// Get the effective timeout seconds
    pub fn get_timeout_seconds(&self) -> Option<u32> {
        self.timeout_seconds.or(self.timeout)
    }

    /// Add metadata entry
    pub fn add_metadata(&mut self, key: String, value: serde_json::Value) {
        if self.metadata.is_none() {
            self.metadata = Some(HashMap::new());
        }
        if let Some(ref mut metadata) = self.metadata {
            metadata.insert(key, value);
        }
    }

    /// Get metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&serde_json::Value> {
        self.metadata.as_ref().and_then(|m| m.get(key))
    }

    /// Check if this is a file modification approval interaction
    pub fn is_file_modification_approval(&self) -> bool {
        self.interaction_type == InteractionType::FileModificationApproval
    }

    /// Get the approval result (if this was an approval interaction)
    pub fn get_approval_result(&self) -> Option<bool> {
        self.response.as_ref().and_then(|r| r.approved)
    }
}

impl Identifiable for UserInteraction {}

impl InteractionResponse {
    /// Create a new approval response
    pub fn approval(approved: bool, comment: Option<String>) -> Self {
        Self {
            approved: Some(approved),
            text: None,
            choices: None,
            comment,
            responded_at: chrono::Utc::now(),
        }
    }

    /// Create a new text input response
    pub fn text_input(text: String, comment: Option<String>) -> Self {
        Self {
            approved: None,
            text: Some(text),
            choices: None,
            comment,
            responded_at: chrono::Utc::now(),
        }
    }

    /// Create a new choice response
    pub fn choice_selection(choices: Vec<String>, comment: Option<String>) -> Self {
        Self {
            approved: None,
            text: None,
            choices: Some(choices),
            comment,
            responded_at: chrono::Utc::now(),
        }
    }
}
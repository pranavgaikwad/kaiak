use serde::{Deserialize, Serialize};
use super::{Id, Timestamp, Metadata};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileModificationProposal {
    pub id: Id,
    pub session_id: Id,
    pub file_path: String,
    pub original_content: String,
    pub proposed_content: String,
    pub change_type: ChangeType,
    pub description: String,
    pub confidence: f32,
    pub approval_status: ApprovalStatus,
    pub created_at: Timestamp,
    pub approved_at: Option<Timestamp>,
    #[serde(default)]
    pub metadata: Metadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    Edit,
    Create,
    Delete,
    Rename,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
}

impl FileModificationProposal {
    pub fn new(
        session_id: Id,
        file_path: String,
        original_content: String,
        proposed_content: String,
        change_type: ChangeType,
        description: String,
        confidence: f32,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id,
            file_path,
            original_content,
            proposed_content,
            change_type,
            description,
            confidence,
            approval_status: ApprovalStatus::Pending,
            created_at: chrono::Utc::now().to_rfc3339(),
            approved_at: None,
            metadata: Metadata::new(),
        }
    }

    pub fn approve(&mut self) {
        self.approval_status = ApprovalStatus::Approved;
        self.approved_at = Some(chrono::Utc::now().to_rfc3339());
    }

    pub fn reject(&mut self) {
        self.approval_status = ApprovalStatus::Rejected;
        self.approved_at = Some(chrono::Utc::now().to_rfc3339());
    }

    pub fn expire(&mut self) {
        self.approval_status = ApprovalStatus::Expired;
    }

    pub fn is_pending(&self) -> bool {
        self.approval_status == ApprovalStatus::Pending
    }

    pub fn is_approved(&self) -> bool {
        self.approval_status == ApprovalStatus::Approved
    }
}
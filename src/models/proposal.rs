use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::{Id, Metadata, Identifiable};

/// File modification proposal for user approval workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileModificationProposal {
    pub id: Id,
    pub session_id: Option<Id>,
    pub file_path: String,
    pub modification_type: String,
    pub original_content: String,
    pub proposed_content: String,
    pub change_type: Option<ChangeType>,

    pub approval_status: ApprovalStatus,
    pub line_range: Option<(u32, u32)>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub approved_at: Option<chrono::DateTime<chrono::Utc>>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    pub metadata: Option<Metadata>,
}

/// Types of file modifications (enhanced version)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ModificationType {
    ContentReplace,
    ContentInsert,
    ContentDelete,
    FileRename,
    FileCreate,
    FileDelete,
    FileMove,
}

/// Legacy change type for backwards compatibility
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    Edit,
    Create,
    Delete,
    Rename,
}

/// Enhanced approval status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
    Applied,
    Error,
}

/// Response to a file modification proposal
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalResponse {
    pub proposal_id: Id,
    pub approved: bool,
    pub comment: Option<String>,
    pub responded_at: chrono::DateTime<chrono::Utc>,
    pub responded_by: Option<String>,
}

impl ModificationType {
    /// Check if this modification type is considered high risk
    pub fn is_high_risk(&self) -> bool {
        matches!(self,
            ModificationType::FileDelete |
            ModificationType::FileMove |
            ModificationType::ContentDelete
        )
    }

    /// Get the default timeout for this modification type in seconds
    pub fn default_timeout_seconds(&self) -> u32 {
        match self {
            ModificationType::FileDelete => 60,
            ModificationType::ContentDelete => 45,
            ModificationType::FileMove => 30,
            ModificationType::ContentReplace => 30,
            ModificationType::ContentInsert => 20,
            ModificationType::FileRename => 20,
            ModificationType::FileCreate => 15,
        }
    }
}

impl FileModificationProposal {
    /// Create a new file modification proposal (enhanced version)
    pub fn new(
        file_path: String,
        modification_type: String,
        original_content: String,
        proposed_content: String,
        _description: String,
    ) -> Self {
        let now = chrono::Utc::now();

        Self {
            id: Self::generate_id(),
            session_id: None,
            file_path,
            modification_type,
            original_content,
            proposed_content,
            change_type: None,
            approval_status: ApprovalStatus::Pending,
            line_range: None,
            created_at: now,
            approved_at: None,
            expires_at: Some(now + chrono::Duration::minutes(5)),
            metadata: None,
        }
    }

    /// Legacy constructor for backwards compatibility
    pub fn new_legacy(
        session_id: Id,
        file_path: String,
        original_content: String,
        proposed_content: String,
        change_type: ChangeType,
        _description: String,
        _confidence: f32,
    ) -> Self {
        let now = chrono::Utc::now();
        let modification_type = match change_type {
            ChangeType::Edit => "content_replace",
            ChangeType::Create => "file_create",
            ChangeType::Delete => "file_delete",
            ChangeType::Rename => "file_rename",
        };

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: Some(session_id),
            file_path,
            modification_type: modification_type.to_string(),
            original_content,
            proposed_content,
            change_type: Some(change_type),
            approval_status: ApprovalStatus::Pending,
            line_range: None,
            created_at: now,
            approved_at: None,
            expires_at: Some(now + chrono::Duration::minutes(5)),
            metadata: None,
        }
    }

    /// Create with line range information
    pub fn new_with_line_range(
        file_path: String,
        modification_type: String,
        original_content: String,
        proposed_content: String,
        description: String,
        line_range: (u32, u32),
    ) -> Self {
        let mut proposal = Self::new(
            file_path,
            modification_type,
            original_content,
            proposed_content,
            description,
        );
        proposal.line_range = Some(line_range);
        proposal
    }

    /// Create with custom expiration
    pub fn new_with_expiry(
        file_path: String,
        modification_type: String,
        original_content: String,
        proposed_content: String,
        description: String,
        expires_in: chrono::Duration,
    ) -> Self {
        let mut proposal = Self::new(
            file_path,
            modification_type,
            original_content,
            proposed_content,
            description,
        );
        proposal.expires_at = Some(proposal.created_at + expires_in);
        proposal
    }

    /// Approve this proposal
    pub fn approve(&mut self) {
        self.approval_status = ApprovalStatus::Approved;
        self.approved_at = Some(chrono::Utc::now());
    }

    /// Reject this proposal
    pub fn reject(&mut self) {
        self.approval_status = ApprovalStatus::Rejected;
        self.approved_at = Some(chrono::Utc::now());
    }

    /// Mark this proposal as expired
    pub fn expire(&mut self) {
        self.approval_status = ApprovalStatus::Expired;
        self.approved_at = Some(chrono::Utc::now());
    }

    /// Check if proposal is pending
    pub fn is_pending(&self) -> bool {
        self.approval_status == ApprovalStatus::Pending
    }

    /// Check if proposal is approved
    pub fn is_approved(&self) -> bool {
        self.approval_status == ApprovalStatus::Approved
    }

    /// Check if this proposal has expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            chrono::Utc::now() > expires_at
        } else {
            false
        }
    }

    /// Get the time remaining before expiry
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

    /// Parse modification type from string
    pub fn parse_modification_type(&self) -> Option<ModificationType> {
        match self.modification_type.as_str() {
            "content_replace" => Some(ModificationType::ContentReplace),
            "content_insert" => Some(ModificationType::ContentInsert),
            "content_delete" => Some(ModificationType::ContentDelete),
            "file_rename" => Some(ModificationType::FileRename),
            "file_create" => Some(ModificationType::FileCreate),
            "file_delete" => Some(ModificationType::FileDelete),
            "file_move" => Some(ModificationType::FileMove),
            _ => None,
        }
    }

    /// Check if this is a high-risk modification
    pub fn is_high_risk(&self) -> bool {
        self.parse_modification_type()
            .map(|mt| mt.is_high_risk())
            .unwrap_or(false)
    }

    /// Get the recommended timeout for this proposal
    pub fn recommended_timeout_seconds(&self) -> u32 {
        self.parse_modification_type()
            .map(|mt| mt.default_timeout_seconds())
            .unwrap_or(30)
    }

    /// Set metadata for this proposal
    pub fn with_metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Add a metadata entry
    pub fn add_metadata(&mut self, key: String, value: serde_json::Value) {
        if self.metadata.is_none() {
            self.metadata = Some(HashMap::new());
        }
        if let Some(ref mut metadata) = self.metadata {
            metadata.insert(key, value);
        }
    }

    /// Get a metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&serde_json::Value> {
        self.metadata.as_ref().and_then(|m| m.get(key))
    }
}

impl Identifiable for FileModificationProposal {}

impl ProposalResponse {
    pub fn approve(proposal_id: Id, comment: Option<String>) -> Self {
        Self {
            proposal_id,
            approved: true,
            comment,
            responded_at: chrono::Utc::now(),
            responded_by: None,
        }
    }

    /// Creae a new rejection response
    pub fn reject(proposal_id: Id, comment: Option<String>) -> Self {
        Self {
            proposal_id,
            approved: false,
            comment,
            responded_at: chrono::Utc::now(),
            responded_by: None,
        }
    }

    pub fn with_user(mut self, user: String) -> Self {
        self.responded_by = Some(user);
        self
    }
}
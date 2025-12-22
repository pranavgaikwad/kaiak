use serde::{Deserialize, Serialize};
use super::{Id, Timestamp, Status, Incident, Metadata};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixGenerationRequest {
    pub id: Id,
    pub session_id: Id,
    pub incidents: Vec<Incident>,
    pub workspace_path: String,
    pub migration_context: Option<serde_json::Value>,
    pub preferences: Option<serde_json::Value>,
    pub status: Status,
    pub created_at: Timestamp,
    pub updated_at: Option<Timestamp>,
    #[serde(default)]
    pub metadata: Metadata,
}

impl FixGenerationRequest {
    pub fn new(
        session_id: Id,
        incidents: Vec<Incident>,
        workspace_path: String,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id,
            incidents,
            workspace_path,
            migration_context: None,
            preferences: None,
            status: Status::Pending,
            created_at: now,
            updated_at: None,
            metadata: Metadata::new(),
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.incidents.is_empty()
            && !self.workspace_path.is_empty()
            && std::path::Path::new(&self.workspace_path).is_absolute()
    }

    pub fn update_status(&mut self, status: Status) {
        self.status = status;
        self.updated_at = Some(chrono::Utc::now().to_rfc3339());
    }
}
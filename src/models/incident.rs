use serde::{Deserialize, Serialize};
use super::{Id, Metadata, Severity};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Incident {
    pub id: Id,
    pub rule_id: String,
    pub file_path: String,
    pub line_number: u32,
    pub severity: Severity,
    pub description: String,
    pub message: String,
    pub category: String,
    #[serde(default)]
    pub metadata: Metadata,
}

impl Incident {
    pub fn new(
        rule_id: String,
        file_path: String,
        line_number: u32,
        severity: Severity,
        description: String,
        message: String,
        category: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            rule_id,
            file_path,
            line_number,
            severity,
            description,
            message,
            category,
            metadata: Metadata::new(),
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.description.is_empty()
            && !self.file_path.is_empty()
            && self.line_number > 0
    }
}
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod incident;
pub mod request;
pub mod session;
pub mod messages;
pub mod proposal;
pub mod interaction;

pub use incident::*;
pub use request::*;
pub use session::*;
pub use messages::*;
pub use proposal::*;
pub use interaction::*;

/// Common identifier type used across all models
pub type Id = String;

/// Timestamp in ISO 8601 format
pub type Timestamp = String;

/// Generic metadata container for extensible data
pub type Metadata = HashMap<String, serde_json::Value>;

/// Common status enumeration for various entities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Pending,
    InProgress,
    Processing,
    Completed,
    Failed,
    Cancelled,
    Terminated,
    Ready,
    Error,
}

/// Severity levels for incidents and errors
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// Helper trait for generating unique identifiers
pub trait Identifiable {
    fn generate_id() -> Id {
        uuid::Uuid::new_v4().to_string()
    }
}

/// Helper trait for timestamping
pub trait Timestampable {
    fn current_timestamp() -> Timestamp {
        chrono::Utc::now().to_rfc3339()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_serialization() {
        let status = Status::InProgress;
        let serialized = serde_json::to_string(&status).unwrap();
        assert_eq!(serialized, "\"in_progress\"");
    }

    #[test]
    fn test_severity_serialization() {
        let severity = Severity::Warning;
        let serialized = serde_json::to_string(&severity).unwrap();
        assert_eq!(serialized, "\"warning\"");
    }
}
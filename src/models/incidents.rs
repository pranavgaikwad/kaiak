use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Validate)]
pub struct MigrationIncident {
    pub id: String,
    pub uri: String,
    pub message: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<IncidentSeverity>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum IncidentSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl Default for IncidentSeverity {
    fn default() -> Self {
        IncidentSeverity::Warning
    }
}

impl std::fmt::Display for IncidentSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IncidentSeverity::Info => write!(f, "info"),
            IncidentSeverity::Warning => write!(f, "warning"),
            IncidentSeverity::Error => write!(f, "error"),
            IncidentSeverity::Critical => write!(f, "critical"),
        }
    }
}

impl std::fmt::Display for MigrationIncident {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let severity = self.severity.as_ref().map(|s| s.to_string()).unwrap_or_else(|| "unknown".to_string());
        write!(f, "[{}] {} - {}", severity, self.id, self.message)
    }
}

impl MigrationIncident {
    pub fn new(
        id: String,
        uri: String,
        message: String,
        description: String,
    ) -> Self {
        Self {
            id,
            uri,
            message,
            description,
            effort: None,
            severity: None,
        }
    }

    /// Create a new incident with all optional fields
    pub fn with_details(
        id: String,
        uri: String,
        message: String,
        description: String,
        effort: Option<String>,
        severity: Option<IncidentSeverity>,
    ) -> Self {
        Self {
            id,
            uri,
            message,
            description,
            effort,
            severity,
        }
    }
}

// Compatibility with existing codebase - alias for the old Incident type
pub type Incident = MigrationIncident;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incident_creation() {
        let incident = MigrationIncident::new(
            "test-id".to_string(),
            "file:///path/to/file.java".to_string(),
            "Use of deprecated API".to_string(),
            "This API has been deprecated".to_string(),
        );

        assert_eq!(incident.id, "test-id");
        assert_eq!(incident.uri, "file:///path/to/file.java");
        assert_eq!(incident.severity, None);
        assert_eq!(incident.effort, None);
    }

    #[test]
    fn test_incident_with_details() {
        let incident = MigrationIncident::with_details(
            "test-id".to_string(),
            "file:///path/to/file.java".to_string(),
            "Use of deprecated API".to_string(),
            "This API has been deprecated".to_string(),
            Some("trivial".to_string()),
            Some(IncidentSeverity::Warning),
        );

        assert_eq!(incident.id, "test-id");
        assert_eq!(incident.severity, Some(IncidentSeverity::Warning));
        assert_eq!(incident.effort, Some("trivial".to_string()));
    }

    #[test]
    fn test_severity_serialization() {
        let severity = IncidentSeverity::Warning;
        let serialized = serde_json::to_string(&severity).unwrap();
        assert_eq!(serialized, "\"warning\"");

        let deserialized: IncidentSeverity = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, IncidentSeverity::Warning);
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(IncidentSeverity::Critical.to_string(), "critical");
        assert_eq!(IncidentSeverity::Error.to_string(), "error");
    }

    #[test]
    fn test_incident_display() {
        let incident = MigrationIncident::with_details(
            "id1".to_string(),
            "file:///path/to/file.java".to_string(),
            "Test message".to_string(),
            "Description".to_string(),
            None,
            Some(IncidentSeverity::Warning),
        );

        assert_eq!(incident.to_string(), "[warning] id1 - Test message");
    }

    #[test]
    fn test_incident_display_no_severity() {
        let incident = MigrationIncident::new(
            "id1".to_string(),
            "file:///path/to/file.java".to_string(),
            "Test message".to_string(),
            "Description".to_string(),
        );

        assert_eq!(incident.to_string(), "[unknown] id1 - Test message");
    }
}
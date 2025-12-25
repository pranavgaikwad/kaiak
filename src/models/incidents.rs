use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Validate)]
pub struct MigrationIncident {
    #[validate(length(min = 1, max = 255, message = "Incident ID must be between 1 and 255 characters"))]
    pub id: String,
    #[validate(length(min = 1, max = 255, message = "Rule ID must be between 1 and 255 characters"))]
    pub rule_id: String,
    #[validate(length(min = 1, max = 1000, message = "Message must be between 1 and 1000 characters"))]
    pub message: String,
    #[validate(length(max = 5000, message = "Description must not exceed 5000 characters"))]
    pub description: String,
    #[validate(length(max = 100, message = "Effort must not exceed 100 characters"))]
    pub effort: String,
    pub severity: IncidentSeverity,
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
        write!(f, "[{}] {} - {}", self.severity, self.rule_id, self.message)
    }
}

impl MigrationIncident {
    pub fn new(
        id: String,
        rule_id: String,
        message: String,
        description: String,
        effort: String,
        severity: IncidentSeverity,
    ) -> Self {
        Self {
            id,
            rule_id,
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
            "deprecated-api".to_string(),
            "Use of deprecated API".to_string(),
            "This API has been deprecated".to_string(),
            "trivial".to_string(),
            IncidentSeverity::Warning,
        );

        assert_eq!(incident.id, "test-id");
        assert_eq!(incident.severity, IncidentSeverity::Warning);
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
        let incident = MigrationIncident::new(
            "id1".to_string(),
            "rule1".to_string(),
            "Test message".to_string(),
            "Description".to_string(),
            "trivial".to_string(),
            IncidentSeverity::Warning,
        );

        assert_eq!(incident.to_string(), "[warning] rule1 - Test message");
    }
}
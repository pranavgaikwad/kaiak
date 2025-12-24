use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::path::PathBuf;

// Import Goose event types
pub use goose::agents::AgentEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEventNotification {
    pub session_id: String,
    pub request_id: Option<String>,
    pub message_id: String,
    pub timestamp: DateTime<Utc>,
    pub event_type: AgentEventType,
    pub content: AgentEventContent,
    pub metadata: EventMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEventType {
    Progress,
    AiResponse,
    ToolCall,
    Thinking,
    UserInteraction,
    FileModification,
    Error,
    System,
    ModelChange,
    HistoryCompacted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEventContent {
    Progress {
        percentage: u8,
        phase: String,
        description: String,
        current_step: Option<String>,
        total_steps: Option<u32>,
    },
    AiResponse {
        text: String,
        partial: bool,
        confidence: Option<f32>,
        tokens: Option<u32>,
    },
    ToolCall {
        tool_name: String,
        operation: String,
        parameters: serde_json::Value,
        status: ToolCallStatus,
        result: Option<ToolCallResult>,
    },
    Thinking {
        internal_monologue: String,
        reasoning_type: String,
        confidence: Option<f32>,
    },
    UserInteraction {
        interaction_id: String,
        interaction_type: UserInteractionType,
        prompt: String,
        options: Option<Vec<String>>,
        default_response: Option<String>,
        timeout: Option<u32>,
    },
    FileModification {
        proposal_id: String,
        file_path: PathBuf,
        operation: FileOperation,
        diff: Option<String>,
        requires_approval: bool,
    },
    Error {
        error_code: String,
        message: String,
        details: Option<String>,
        recoverable: bool,
        suggested_action: Option<String>,
    },
    System {
        message: String,
        level: SystemEventLevel,
        component: String,
    },
    ModelChange {
        old_model: String,
        new_model: String,
        reason: String,
    },
    HistoryCompacted {
        original_length: u32,
        compacted_length: u32,
        tokens_saved: Option<u32>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallStatus {
    Pending,
    Approved,
    Denied,
    Executing,
    Completed,
    Failed,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub execution_time: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserInteractionType {
    Confirmation,
    Choice,
    TextInput,
    FileApproval,
    ToolPermission,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileOperation {
    Create,
    Modify,
    Delete,
    Rename,
    Copy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SystemEventLevel {
    Debug,
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    pub sequence_number: u64,
    pub correlation_id: Option<String>,
    pub trace_id: Option<String>,
    pub processing_duration: Option<u64>,
}

impl AgentEventNotification {
    pub fn new(
        session_id: String,
        message_id: String,
        event_type: AgentEventType,
        content: AgentEventContent,
    ) -> Self {
        Self {
            session_id,
            request_id: None,
            message_id,
            timestamp: Utc::now(),
            event_type,
            content,
            metadata: EventMetadata {
                sequence_number: 0,
                correlation_id: None,
                trace_id: None,
                processing_duration: None,
            },
        }
    }

    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    pub fn with_metadata(mut self, metadata: EventMetadata) -> Self {
        self.metadata = metadata;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_notification_creation() {
        let event = AgentEventNotification::new(
            "session-123".to_string(),
            "msg-456".to_string(),
            AgentEventType::Progress,
            AgentEventContent::Progress {
                percentage: 50,
                phase: "processing".to_string(),
                description: "Processing incidents".to_string(),
                current_step: Some("Step 1".to_string()),
                total_steps: Some(3),
            },
        );

        assert_eq!(event.session_id, "session-123");
        assert_eq!(event.message_id, "msg-456");
        assert!(event.request_id.is_none());
    }

    #[test]
    fn test_tool_call_status_serialization() {
        let status = ToolCallStatus::Completed;
        let serialized = serde_json::to_string(&status).unwrap();
        assert_eq!(serialized, "\"completed\"");
    }

    #[test]
    fn test_system_event_level_serialization() {
        let level = SystemEventLevel::Warning;
        let serialized = serde_json::to_string(&level).unwrap();
        assert_eq!(serialized, "\"warning\"");
    }
}
use serde::{Deserialize, Serialize};
use super::{Id, Timestamp, Metadata};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamMessage {
    pub id: Id,
    pub session_id: Id,
    pub request_id: Option<Id>,
    pub message_type: MessageType,
    pub timestamp: Timestamp,
    pub content: MessageContent,
    #[serde(default)]
    pub metadata: Metadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum MessageType {
    Progress,
    AiResponse,
    ToolCall,
    Thinking,
    UserInteraction,
    FileModification,
    Error,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MessageContent {
    #[serde(rename = "progress")]
    Progress {
        percentage: u8,
        phase: String,
        description: String,
    },
    #[serde(rename = "ai_response")]
    AiResponse {
        text: String,
        partial: bool,
        confidence: Option<f32>,
    },
    #[serde(rename = "tool_call")]
    ToolCall {
        tool_name: String,
        operation: ToolOperation,
        parameters: serde_json::Value,
        result: Option<ToolResult>,
    },
    #[serde(rename = "thinking")]
    Thinking { text: String },
    #[serde(rename = "user_interaction")]
    UserInteraction {
        interaction_id: Id,
        interaction_type: String,
        prompt: String,
        proposal_id: Option<Id>,
        timeout: Option<u32>,
    },
    #[serde(rename = "file_modification")]
    FileModification {
        proposal_id: Id,
        file_path: String,
        change_type: String,
        description: String,
        original_content: String,
        proposed_content: String,
        confidence: f32,
    },
    #[serde(rename = "error")]
    Error {
        error_code: String,
        message: String,
        details: Option<String>,
        recoverable: bool,
    },
    #[serde(rename = "system")]
    System {
        event: String,
        request_id: Option<Id>,
        status: String,
        summary: Option<serde_json::Value>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolOperation {
    Start,
    Progress,
    Complete,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}

impl StreamMessage {
    pub fn new(
        session_id: Id,
        request_id: Option<Id>,
        message_type: MessageType,
        content: MessageContent,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id,
            request_id,
            message_type,
            timestamp: chrono::Utc::now().to_rfc3339(),
            content,
            metadata: Metadata::new(),
        }
    }
}
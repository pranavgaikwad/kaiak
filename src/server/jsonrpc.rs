use serde::{Deserialize, Serialize};
use serde_json::Value;
use tower_lsp::jsonrpc::{Error, ErrorCode};

/// Custom JSON-RPC error codes for Kaiak-specific operations
pub mod error_codes {
    pub const SESSION_CREATION_FAILED: i32 = -32001;
    pub const WORKSPACE_ACCESS_DENIED: i32 = -32002;
    pub const SESSION_NOT_FOUND: i32 = -32003;
    pub const SESSION_ALREADY_TERMINATED: i32 = -32004;
    pub const SESSION_NOT_READY: i32 = -32005;
    pub const AGENT_INITIALIZATION_FAILED: i32 = -32006;
    pub const REQUEST_NOT_FOUND: i32 = -32007;
    pub const REQUEST_ALREADY_COMPLETED: i32 = -32008;
    pub const INTERACTION_NOT_FOUND: i32 = -32009;
    pub const INTERACTION_ALREADY_RESPONDED: i32 = -32010;
    pub const RESPONSE_VALIDATION_FAILED: i32 = -32011;
    pub const FILE_MODIFICATION_FAILED: i32 = -32012;
    pub const TOOL_EXECUTION_TIMEOUT: i32 = -32013;
    pub const CONFIGURATION_ERROR: i32 = -32014;
    pub const RESOURCE_EXHAUSTED: i32 = -32015;
}

/// Custom notification methods for Kaiak streaming
pub mod methods {
    pub const STREAM_PROGRESS: &str = "kaiak/stream/progress";
    pub const STREAM_AI_RESPONSE: &str = "kaiak/stream/ai_response";
    pub const STREAM_TOOL_CALL: &str = "kaiak/stream/tool_call";
    pub const STREAM_THINKING: &str = "kaiak/stream/thinking";
    pub const STREAM_USER_INTERACTION: &str = "kaiak/stream/user_interaction";
    pub const STREAM_FILE_MODIFICATION: &str = "kaiak/stream/file_modification";
    pub const STREAM_ERROR: &str = "kaiak/stream/error";
    pub const STREAM_SYSTEM: &str = "kaiak/stream/system";

    pub const SESSION_CREATE: &str = "kaiak/session/create";
    pub const SESSION_TERMINATE: &str = "kaiak/session/terminate";
    pub const SESSION_STATUS: &str = "kaiak/session/status";
    pub const FIX_GENERATE: &str = "kaiak/fix/generate";
    pub const FIX_CANCEL: &str = "kaiak/fix/cancel";
    pub const INTERACTION_RESPOND: &str = "kaiak/interaction/respond";
}

/// Helper functions for creating JSON-RPC errors
pub fn create_error(code: i32, message: &str, data: Option<Value>) -> Error {
    Error {
        code: ErrorCode::ServerError(code.into()),
        message: message.to_string().into(),
        data,
    }
}

pub fn session_not_found_error(session_id: &str) -> Error {
    create_error(
        error_codes::SESSION_NOT_FOUND,
        "Session not found",
        Some(serde_json::json!({ "session_id": session_id })),
    )
}

pub fn workspace_access_denied_error(workspace_path: &str) -> Error {
    create_error(
        error_codes::WORKSPACE_ACCESS_DENIED,
        "Workspace access denied",
        Some(serde_json::json!({ "workspace_path": workspace_path })),
    )
}

/// Request/response types for session management
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub workspace_path: String,
    pub session_name: Option<String>,
    pub configuration: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSessionResponse {
    pub session_id: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TerminateSessionRequest {
    pub session_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TerminateSessionResponse {
    pub session_id: String,
    pub status: String,
    pub message_count: u32,
    pub terminated_at: String,
}

/// Request/response types for fix generation
#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateFixRequest {
    pub session_id: String,
    pub incidents: Vec<Value>,
    pub migration_context: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateFixResponse {
    pub request_id: String,
    pub session_id: String,
    pub status: String,
    pub incident_count: usize,
    pub created_at: String,
}

/// Streaming notification types
#[derive(Debug, Serialize, Deserialize)]
pub struct StreamNotification {
    pub session_id: String,
    pub request_id: Option<String>,
    pub message_id: String,
    pub timestamp: String,
    pub content: Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = create_error(-32001, "Test error", None);
        assert_eq!(error.code, ErrorCode::ServerError(-32001));
        assert_eq!(error.message, "Test error");
    }

    #[test]
    fn test_session_not_found_error() {
        let error = session_not_found_error("test-session");
        if let ErrorCode::ServerError(code) = error.code {
            assert_eq!(code, error_codes::SESSION_NOT_FOUND as i64);
        }
    }
}
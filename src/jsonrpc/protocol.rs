//! Core JSON-RPC 2.0 protocol implementation
//!
//! This implements the JSON-RPC 2.0 specification without external dependencies,
//! providing exactly what we need for LSP-style communication.

use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    /// Request ID (can be string, number, or null for notifications)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 Error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 Batch request (array of requests)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcBatch(pub Vec<JsonRpcRequest>);

/// JSON-RPC 2.0 Batch response (array of responses)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcBatchResponse(pub Vec<JsonRpcResponse>);

/// JSON-RPC 2.0 Notification (server-to-client, no response expected)
/// 
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl JsonRpcNotification {
    /// Create a new notification
    pub fn new(method: impl Into<String>, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params,
        }
    }

    /// Create a progress notification (common pattern)
    pub fn progress(token: impl Into<String>, value: serde_json::Value) -> Self {
        Self::new(
            "$/progress",
            Some(serde_json::json!({
                "token": token.into(),
                "value": value,
            })),
        )
    }
}

/// Standard JSON-RPC 2.0 error codes
pub mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
    pub const SERVER_ERROR_START: i32 = -32099;
    pub const SERVER_ERROR_END: i32 = -32000;
}

impl JsonRpcRequest {
    /// Create a new request
    pub fn new(method: String, params: Option<serde_json::Value>, id: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method,
            params,
            id,
        }
    }

    /// Create a notification (request without id)
    pub fn notification(method: String, params: Option<serde_json::Value>) -> Self {
        Self::new(method, params, None)
    }

    /// Check if this is a notification (no response expected)
    pub fn is_notification(&self) -> bool {
        self.id.is_none()
    }

    /// Validate the request structure
    pub fn validate(&self) -> Result<(), JsonRpcError> {
        if self.jsonrpc != "2.0" {
            return Err(JsonRpcError {
                code: error_codes::INVALID_REQUEST,
                message: "Invalid JSON-RPC version".to_string(),
                data: None,
            });
        }

        if self.method.is_empty() {
            return Err(JsonRpcError {
                code: error_codes::INVALID_REQUEST,
                message: "Method name cannot be empty".to_string(),
                data: None,
            });
        }

        if self.method.starts_with("rpc.") {
            return Err(JsonRpcError {
                code: error_codes::INVALID_REQUEST,
                message: "Method names starting with 'rpc.' are reserved".to_string(),
                data: None,
            });
        }

        Ok(())
    }
}

impl JsonRpcResponse {
    /// Create a success response
    pub fn success(result: serde_json::Value, id: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    /// Create an error response
    pub fn error(error: JsonRpcError, id: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(error),
            id,
        }
    }

    /// Create a parse error response
    pub fn parse_error() -> Self {
        Self::error(
            JsonRpcError {
                code: error_codes::PARSE_ERROR,
                message: "Parse error".to_string(),
                data: None,
            },
            None,
        )
    }

    /// Create an invalid request error response
    pub fn invalid_request(id: Option<serde_json::Value>) -> Self {
        Self::error(
            JsonRpcError {
                code: error_codes::INVALID_REQUEST,
                message: "Invalid Request".to_string(),
                data: None,
            },
            id,
        )
    }

    /// Create a method not found error response
    pub fn method_not_found(method: &str, id: Option<serde_json::Value>) -> Self {
        Self::error(
            JsonRpcError {
                code: error_codes::METHOD_NOT_FOUND,
                message: "Method not found".to_string(),
                data: Some(serde_json::json!({ "method": method })),
            },
            id,
        )
    }

    /// Create an invalid params error response
    pub fn invalid_params(message: &str, id: Option<serde_json::Value>) -> Self {
        Self::error(
            JsonRpcError {
                code: error_codes::INVALID_PARAMS,
                message: format!("Invalid params: {}", message),
                data: None,
            },
            id,
        )
    }

    /// Create an internal error response
    pub fn internal_error(message: &str, id: Option<serde_json::Value>) -> Self {
        Self::error(
            JsonRpcError {
                code: error_codes::INTERNAL_ERROR,
                message: format!("Internal error: {}", message),
                data: None,
            },
            id,
        )
    }
}

impl JsonRpcError {
    /// Create a custom application error
    pub fn custom(code: i32, message: String, data: Option<serde_json::Value>) -> Self {
        Self { code, message, data }
    }
}

/// Convert our KaiakError to JSON-RPC error
impl From<crate::KaiakError> for JsonRpcError {
    fn from(error: crate::KaiakError) -> Self {
        let code = match error {
            crate::KaiakError::Configuration { .. } => -32014,
            crate::KaiakError::Session { .. } => -32003,
            crate::KaiakError::SessionNotFound(_) => -32003,
            crate::KaiakError::Agent { .. } => -32010,
            crate::KaiakError::Workspace { .. } => -32011,
            crate::KaiakError::AgentInitialization { .. } => -32012,
            crate::KaiakError::SessionInUse { .. } => -32013,
            crate::KaiakError::ResourceExhausted(_) => -32015,
            crate::KaiakError::Io { .. } => -32016,
            crate::KaiakError::Serialization { .. } => -32017,
            crate::KaiakError::Transport { .. } => -32001,
            crate::KaiakError::InvalidWorkspacePath(_) => -32011,
            crate::KaiakError::Internal(_) => -32603,
            crate::KaiakError::GooseIntegration { .. } => -32010,
            crate::KaiakError::ToolExecution { .. } => -32013,
            crate::KaiakError::InteractionTimeout { .. } => -32014,
            crate::KaiakError::FileOperation { .. } => -32012,
        };

        JsonRpcError {
            code,
            message: error.user_message(),
            data: Some(serde_json::json!({
                "error_type": format!("{:?}", std::mem::discriminant(&error))
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_creation() {
        let req = JsonRpcRequest::new(
            "test_method".to_string(),
            Some(serde_json::json!({"param": "value"})),
            Some(serde_json::json!(1)),
        );

        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.method, "test_method");
        assert!(!req.is_notification());
    }

    #[test]
    fn test_notification_creation() {
        let req = JsonRpcRequest::notification(
            "notify_method".to_string(),
            Some(serde_json::json!({"param": "value"})),
        );

        assert!(req.is_notification());
        assert!(req.id.is_none());
    }

    #[test]
    fn test_request_validation() {
        let mut req = JsonRpcRequest::new(
            "test_method".to_string(),
            None,
            Some(serde_json::json!(1)),
        );

        assert!(req.validate().is_ok());

        // Test invalid version
        req.jsonrpc = "1.0".to_string();
        assert!(req.validate().is_err());

        // Test reserved method name
        req.jsonrpc = "2.0".to_string();
        req.method = "rpc.test".to_string();
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_response_creation() {
        let resp = JsonRpcResponse::success(
            serde_json::json!({"result": "success"}),
            Some(serde_json::json!(1)),
        );

        assert_eq!(resp.jsonrpc, "2.0");
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());

        let err_resp = JsonRpcResponse::method_not_found("unknown", Some(serde_json::json!(1)));
        assert!(err_resp.error.is_some());
        assert!(err_resp.result.is_none());
    }

    #[test]
    fn test_serialization() {
        let req = JsonRpcRequest::new(
            "test".to_string(),
            Some(serde_json::json!({"a": 1})),
            Some(serde_json::json!(42)),
        );

        let json = serde_json::to_string(&req).unwrap();
        let parsed: JsonRpcRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(req.method, parsed.method);
        assert_eq!(req.id, parsed.id);
    }
}
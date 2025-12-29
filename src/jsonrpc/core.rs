//! Core JSON-RPC utilities and traits shared between client and server

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use validator::Validate;

/// Trait for registering Kaiak JSON-RPC methods with consistent patterns
pub trait KaiakJsonRpcHandler {
    /// Register all methods provided by this handler
    fn register_methods(&self, server: &crate::jsonrpc::JsonRpcServer);

    /// Get the handler name for logging and debugging
    fn handler_name(&self) -> &'static str;

    /// Get handler version for compatibility checking
    fn handler_version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }
}

/// Request wrapper for validation and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KaiakRequest<T> {
    /// The actual request payload
    pub inner: T,

    /// Optional session information for request context
    pub session_info: Option<SessionInfo>,

    /// Request metadata for tracking and debugging
    pub metadata: RequestMetadata,
}

impl<T> KaiakRequest<T>
where
    T: Validate,
{
    /// Create a new request with validation
    pub fn new(inner: T, session_info: Option<SessionInfo>) -> Result<Self> {
        // Validate the request
        inner.validate().map_err(|e| anyhow::anyhow!("Request validation failed: {:?}", e))?;

        let metadata = RequestMetadata::new();

        Ok(Self {
            inner,
            session_info,
            metadata,
        })
    }

    /// Create a new request without session info
    pub fn without_session(inner: T) -> Result<Self> {
        Self::new(inner, None)
    }
}

/// General implementation for KaiakRequest without Validate constraint
impl<T> KaiakRequest<T> {
    /// Get the request ID for tracking (general implementation)
    pub fn request_id(&self) -> &str {
        &self.metadata.request_id
    }

    /// Get the creation timestamp (general implementation)
    pub fn created_at(&self) -> DateTime<Utc> {
        self.metadata.created_at
    }
}

/// Response wrapper for consistency and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KaiakResponse<T> {
    /// The actual response payload
    pub inner: T,

    /// Response metadata for tracking and debugging
    pub metadata: ResponseMetadata,
}

impl<T> KaiakResponse<T> {
    /// Create a new response
    pub fn new(inner: T, request_id: String) -> Self {
        let metadata = ResponseMetadata::new(request_id);

        Self { inner, metadata }
    }

    /// Create a response from a request
    pub fn from_request<R>(inner: T, request: &KaiakRequest<R>) -> Self {
        Self::new(inner, request.request_id().to_string())
    }

    /// Convert to JSON-RPC result
    pub fn into_rpc_result(self) -> Result<T, crate::jsonrpc::JsonRpcError> {
        Ok(self.inner)
    }
}

/// Session information for request context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session ID if applicable
    pub session_id: Option<String>,

    /// User or client identifier
    pub client_id: Option<String>,

    /// Additional session metadata
    pub metadata: HashMap<String, Value>,
}

impl SessionInfo {
    /// Create new session info
    pub fn new() -> Self {
        Self {
            session_id: None,
            client_id: None,
            metadata: HashMap::new(),
        }
    }

    /// Create session info with session ID
    pub fn with_session(session_id: String) -> Self {
        Self {
            session_id: Some(session_id),
            client_id: None,
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to session info
    pub fn with_metadata(mut self, key: String, value: Value) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

impl Default for SessionInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Request metadata for tracking and debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMetadata {
    /// Unique request identifier
    pub request_id: String,

    /// Request creation timestamp
    pub created_at: DateTime<Utc>,

    /// Client version that made the request
    pub client_version: Option<String>,

    /// Additional custom metadata
    pub custom: HashMap<String, Value>,
}

impl RequestMetadata {
    /// Create new request metadata with generated ID
    pub fn new() -> Self {
        Self {
            request_id: uuid::Uuid::new_v4().to_string(),
            created_at: Utc::now(),
            client_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            custom: HashMap::new(),
        }
    }

    /// Create request metadata with specific ID
    pub fn with_id(request_id: String) -> Self {
        Self {
            request_id,
            created_at: Utc::now(),
            client_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            custom: HashMap::new(),
        }
    }

    /// Add custom metadata
    pub fn with_custom(mut self, key: String, value: Value) -> Self {
        self.custom.insert(key, value);
        self
    }
}

impl Default for RequestMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Response metadata for tracking and debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    /// Corresponding request ID
    pub request_id: String,

    /// Response creation timestamp
    pub created_at: DateTime<Utc>,

    /// Server version that generated the response
    pub server_version: String,

    /// Processing duration in milliseconds
    pub processing_duration_ms: Option<u64>,

    /// Additional custom metadata
    pub custom: HashMap<String, Value>,
}

impl ResponseMetadata {
    /// Create new response metadata
    pub fn new(request_id: String) -> Self {
        Self {
            request_id,
            created_at: Utc::now(),
            server_version: env!("CARGO_PKG_VERSION").to_string(),
            processing_duration_ms: None,
            custom: HashMap::new(),
        }
    }

    /// Set processing duration
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.processing_duration_ms = Some(duration_ms);
        self
    }

    /// Add custom metadata
    pub fn with_custom(mut self, key: String, value: Value) -> Self {
        self.custom.insert(key, value);
        self
    }
}

/// Helper trait for converting types to JSON-RPC results
pub trait IntoRpcResult<T> {
    /// Convert to JSON-RPC result
    fn into_rpc_result(self) -> Result<T, crate::jsonrpc::JsonRpcError>;
}

impl<T> IntoRpcResult<T> for Result<T> {
    fn into_rpc_result(self) -> Result<T, crate::jsonrpc::JsonRpcError> {
        self.map_err(|e| {
            crate::jsonrpc::JsonRpcError::custom(
                crate::jsonrpc::protocol::error_codes::INTERNAL_ERROR,
                format!("Internal error: {}", e),
                None,
            )
        })
    }
}

impl<T> IntoRpcResult<T> for T {
    fn into_rpc_result(self) -> Result<T, crate::jsonrpc::JsonRpcError> {
        Ok(self)
    }
}

/// Validation helper for JSON-RPC parameters
pub fn validate_params<T>(params: T) -> Result<T, crate::jsonrpc::JsonRpcError>
where
    T: Validate,
{
    params.validate().map_err(|e| {
        crate::jsonrpc::JsonRpcError::custom(
            crate::jsonrpc::protocol::error_codes::INVALID_PARAMS,
            format!("Parameter validation failed: {:?}", e),
            None,
        )
    })?;
    Ok(params)
}

/// Create a standardized JSON-RPC error with consistent formatting
pub fn create_standard_error(code: i32, message: &str, data: Option<Value>) -> crate::jsonrpc::JsonRpcError {
    crate::jsonrpc::JsonRpcError::custom(code, message.to_string(), data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Serialize, Deserialize, Validate)]
    struct TestRequest {
        #[validate(length(min = 1))]
        name: String,
    }

    #[test]
    fn test_kaiak_request_creation() {
        let test_req = TestRequest {
            name: "test".to_string(),
        };

        let request = KaiakRequest::without_session(test_req).unwrap();
        assert!(!request.request_id().is_empty());
        assert_eq!(request.inner.name, "test");
    }

    #[test]
    fn test_kaiak_request_validation_failure() {
        let test_req = TestRequest {
            name: "".to_string(), // Should fail validation
        };

        let result = KaiakRequest::without_session(test_req);
        assert!(result.is_err());
    }

    #[test]
    fn test_kaiak_response_creation() {
        let test_resp = "test response";
        let response = KaiakResponse::new(test_resp, "test-id".to_string());

        assert_eq!(response.inner, "test response");
        assert_eq!(response.metadata.request_id, "test-id");
    }

    #[test]
    fn test_session_info() {
        let session_info = SessionInfo::with_session("session-123".to_string())
            .with_metadata("key".to_string(), Value::String("value".to_string()));

        assert_eq!(session_info.session_id.unwrap(), "session-123");
        assert_eq!(session_info.metadata.get("key").unwrap(), &Value::String("value".to_string()));
    }
}
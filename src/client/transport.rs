//! JSON-RPC client transport for Unix socket communication

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use uuid::Uuid;

/// Client information for debugging and tracing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub version: String,
    pub socket_path: String,
    pub request_id: String,
}

impl ClientInfo {
    pub fn new(socket_path: String) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            socket_path,
            request_id: Uuid::new_v4().to_string(),
        }
    }
}

/// JSON-RPC request wrapper for client-side calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientRequest {
    /// JSON-RPC method name
    pub method: String,

    /// Request parameters as JSON value
    pub params: Value,

    /// Optional request timeout in seconds
    pub timeout: Option<u64>,

    /// Client connection info (for debugging)
    pub client_info: Option<ClientInfo>,
}

impl ClientRequest {
    pub fn new(method: String, params: Value) -> Self {
        Self {
            method,
            params,
            timeout: None,
            client_info: None,
        }
    }

    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout = Some(timeout_seconds);
        self
    }

    pub fn with_client_info(mut self, client_info: ClientInfo) -> Self {
        self.client_info = Some(client_info);
        self
    }
}

/// JSON-RPC 2.0 request structure
#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Value,
    id: String,
}

/// JSON-RPC 2.0 response structure
#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 error structure
#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// JSON-RPC client for Unix socket communication
pub struct JsonRpcClient {
    socket_path: String,
}

impl JsonRpcClient {
    /// Create a new JSON-RPC client
    pub fn new(socket_path: String) -> Self {
        Self { socket_path }
    }

    /// Validate that the socket exists and is accessible
    pub async fn validate_connection(&self) -> Result<bool> {
        let socket_path = Path::new(&self.socket_path);

        if !socket_path.exists() {
            return Ok(false);
        }

        // Try to connect to validate the socket
        match UnixStream::connect(&self.socket_path).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Execute a JSON-RPC procedure call
    pub async fn call(&self, request: ClientRequest) -> Result<Value> {
        // Generate unique request ID
        let request_id = Uuid::new_v4().to_string();

        // Create JSON-RPC 2.0 request
        let jsonrpc_request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: request.method.clone(),
            params: request.params,
            id: request_id.clone(),
        };

        // Connect to socket
        let mut stream = UnixStream::connect(&self.socket_path)
            .await
            .map_err(|e| anyhow!("Failed to connect to socket {}: {}", self.socket_path, e))?;

        // Serialize request to JSON
        let request_json = serde_json::to_string(&jsonrpc_request)
            .map_err(|e| anyhow!("Failed to serialize request: {}", e))?;

        // Send length-prefixed message
        let message_bytes = request_json.as_bytes();
        let length = message_bytes.len() as u32;

        stream.write_all(&length.to_be_bytes()).await
            .map_err(|e| anyhow!("Failed to write message length: {}", e))?;

        stream.write_all(message_bytes).await
            .map_err(|e| anyhow!("Failed to write message: {}", e))?;

        // Read response length
        let mut length_bytes = [0u8; 4];
        stream.read_exact(&mut length_bytes).await
            .map_err(|e| anyhow!("Failed to read response length: {}", e))?;

        let response_length = u32::from_be_bytes(length_bytes) as usize;

        // Read response data
        let mut response_bytes = vec![0u8; response_length];
        stream.read_exact(&mut response_bytes).await
            .map_err(|e| anyhow!("Failed to read response: {}", e))?;

        // Parse JSON response
        let response_json = String::from_utf8(response_bytes)
            .map_err(|e| anyhow!("Invalid UTF-8 in response: {}", e))?;

        let response: JsonRpcResponse = serde_json::from_str(&response_json)
            .map_err(|e| anyhow!("Failed to parse response JSON: {}", e))?;

        // Verify response ID matches request
        if response.id != request_id {
            return Err(anyhow!("Response ID mismatch: expected {}, got {}", request_id, response.id));
        }

        // Handle error response
        if let Some(error) = response.error {
            return Err(anyhow!("JSON-RPC error {}: {}", error.code, error.message));
        }

        // Return result
        response.result
            .ok_or_else(|| anyhow!("Response missing both result and error"))
    }

    /// Execute configure procedure
    pub async fn configure(&self, params: Value) -> Result<Value> {
        let request = ClientRequest::new("kaiak/configure".to_string(), params)
            .with_client_info(ClientInfo::new(self.socket_path.clone()));

        self.call(request).await
    }

    /// Execute generate_fix procedure
    pub async fn generate_fix(&self, params: Value) -> Result<Value> {
        let request = ClientRequest::new("kaiak/generate_fix".to_string(), params)
            .with_timeout(300) // 5 minute timeout for AI operations
            .with_client_info(ClientInfo::new(self.socket_path.clone()));

        self.call(request).await
    }

    /// Execute delete_session procedure
    pub async fn delete_session(&self, params: Value) -> Result<Value> {
        let request = ClientRequest::new("kaiak/delete_session".to_string(), params)
            .with_client_info(ClientInfo::new(self.socket_path.clone()));

        self.call(request).await
    }

    /// Get socket path
    pub fn socket_path(&self) -> &str {
        &self.socket_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_procedure_request_creation() {
        let params = serde_json::json!({"test": "value"});
        let request = ClientRequest::new("test/method".to_string(), params.clone());

        assert_eq!(request.method, "test/method");
        assert_eq!(request.params, params);
        assert!(request.timeout.is_none());
        assert!(request.client_info.is_none());
    }

    #[test]
    fn test_procedure_request_with_timeout() {
        let params = serde_json::json!({});
        let request = ClientRequest::new("test/method".to_string(), params)
            .with_timeout(30);

        assert_eq!(request.timeout, Some(30));
    }

    #[test]
    fn test_client_info_creation() {
        let socket_path = "/tmp/test.sock".to_string();
        let client_info = ClientInfo::new(socket_path.clone());

        assert_eq!(client_info.socket_path, socket_path);
        assert_eq!(client_info.version, env!("CARGO_PKG_VERSION"));
        assert!(!client_info.request_id.is_empty());
    }

    #[test]
    fn test_jsonrpc_client_creation() {
        let socket_path = "/tmp/test.sock".to_string();
        let client = JsonRpcClient::new(socket_path.clone());

        assert_eq!(client.socket_path(), &socket_path);
    }
}
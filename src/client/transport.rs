//! JSON-RPC client transport for Unix socket communication

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tracing::{debug, trace};
use uuid::Uuid;

use crate::jsonrpc::{JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};

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

/// Client-side request builder for JSON-RPC calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientRequest {
    pub method: String,
    pub params: Value,
    pub timeout: Option<u64>,
    pub client_info: Option<ClientInfo>, // Only exists for debugging purposes
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

    /// Convert to a JsonRpcRequest with the given ID
    fn to_jsonrpc_request(&self, id: String) -> JsonRpcRequest {
        JsonRpcRequest::new(
            self.method.clone(),
            Some(self.params.clone()),
            Some(Value::String(id)),
        )
    }
}

/// JSON-RPC client for Unix socket communication
/// 
/// Uses LSP-style Content-Length framing to match the server protocol.
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

    /// Execute a JSON-RPC procedure call using LSP-style framing
    /// 
    /// Reads all messages from the server until it receives the final response.
    /// Notifications are passed to the provided callback.
    /// 
    /// # Example
    /// ```ignore
    /// // With notification handling
    /// client.call(request, |n| println!("Notification: {:?}", n)).await?;
    /// 
    /// // Without notification handling  
    /// client.call(request, |_| {}).await?;
    /// ```
    pub async fn call<F>(&self, request: ClientRequest, mut on_notification: F) -> Result<Value>
    where
        F: FnMut(JsonRpcNotification),
    {
        let request_id = Uuid::new_v4().to_string();

        let jsonrpc_request = request.to_jsonrpc_request(request_id.clone());

        let stream = UnixStream::connect(&self.socket_path)
            .await
            .map_err(|e| anyhow!("Failed to connect to socket {}: {}", self.socket_path, e))?;

        let (read_half, mut write_half) = stream.into_split();
        let mut reader = BufReader::new(read_half);

        let request_json = serde_json::to_string(&jsonrpc_request)
            .map_err(|e| anyhow!("Failed to serialize request: {}", e))?;

        debug!("Sending request: {}", request_json);

        let message = format!("Content-Length: {}\r\n\r\n{}", request_json.len(), request_json);
        write_half.write_all(message.as_bytes()).await
            .map_err(|e| anyhow!("Failed to write message: {}", e))?;
        write_half.flush().await
            .map_err(|e| anyhow!("Failed to flush: {}", e))?;

        loop {
            let message_json = Self::read_lsp_message(&mut reader).await?;
            debug!("Received message: {}", message_json);

            let msg: Value = serde_json::from_str(&message_json)
                .map_err(|e| anyhow!("Failed to parse message JSON: {}", e))?;

            let is_notification = msg.get("method").is_some() 
                && (msg.get("id").is_none() || msg.get("id") == Some(&Value::Null));

            if is_notification {
                let notification: JsonRpcNotification = serde_json::from_value(msg)
                    .map_err(|e| anyhow!("Failed to parse notification: {}", e))?;
                on_notification(notification);
            } else {
                let response: JsonRpcResponse = serde_json::from_value(msg)
                    .map_err(|e| anyhow!("Failed to parse response: {}", e))?;

                let response_id = response.id
                    .as_ref()
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                    
                if response_id != request_id {
                    return Err(anyhow!("Response ID mismatch: expected {}, got {}", request_id, response_id));
                }

                if let Some(ref error) = response.error {
                    return Err(anyhow!("JSON-RPC error {}: {}", error.code, error.message));
                }

                return response.result
                    .ok_or_else(|| anyhow!("Response missing both result and error"));
            }
        }
    }

    /// Read an LSP-style message with Content-Length header
    async fn read_lsp_message<R: tokio::io::AsyncBufRead + Unpin>(reader: &mut R) -> Result<String> {
        let mut content_length: Option<usize> = None;

        loop {
            let mut line = String::new();
            let bytes_read = reader.read_line(&mut line).await
                .map_err(|e| anyhow!("Failed to read header line: {}", e))?;

            if bytes_read == 0 {
                return Err(anyhow!("Connection closed while reading headers"));
            }

            let line = line.trim_end();
            trace!("Read header: {}", line);

            if line.is_empty() {
                break;
            }

            if let Some(length_str) = line.strip_prefix("Content-Length: ") {
                content_length = Some(length_str.parse()
                    .map_err(|e| anyhow!("Invalid Content-Length: {}", e))?);
            }
        }

        let content_length = content_length
            .ok_or_else(|| anyhow!("Missing Content-Length header"))?;

        let mut buffer = vec![0u8; content_length];
        reader.read_exact(&mut buffer).await
            .map_err(|e| anyhow!("Failed to read message body: {}", e))?;

        String::from_utf8(buffer)
            .map_err(|e| anyhow!("Invalid UTF-8 in response: {}", e))
    }

    /// Execute generate_fix procedure
    pub async fn generate_fix<F>(&self, params: Value, on_notification: F) -> Result<Value>
    where
        F: FnMut(JsonRpcNotification),
    {
        let request = ClientRequest::new("kaiak/generate_fix".to_string(), params)
            .with_timeout(300) // 5 minute timeout for AI operations
            .with_client_info(ClientInfo::new(self.socket_path.clone()));

        self.call(request, on_notification).await
    }

    /// Execute delete_session procedure
    pub async fn delete_session<F>(&self, params: Value, on_notification: F) -> Result<Value>
    where
        F: FnMut(JsonRpcNotification),
    {
        let request = ClientRequest::new("kaiak/delete_session".to_string(), params)
            .with_client_info(ClientInfo::new(self.socket_path.clone()));

        self.call(request, on_notification).await
    }

    /// Get socket path
    pub fn socket_path(&self) -> &str {
        &self.socket_path
    }
}


/// Manages the connection state file (~/.kaiak/connection)
/// Everytime user connects to a server, we store the path to
/// the socket file as connection state so all subsequent requests
/// can be made to the same server until disconnect is called
pub struct ConnectionState;

impl ConnectionState {
    pub fn state_file_path() -> Result<PathBuf> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| anyhow!("Unable to determine home directory"))?;
        
        let kaiak_dir = home_dir.join(".kaiak");
        if !kaiak_dir.exists() {
            std::fs::create_dir_all(&kaiak_dir)?;
        }
        
        Ok(kaiak_dir.join("connection"))
    }

    /// Save the current connection (socket path)
    pub fn save(socket_path: &str) -> Result<()> {
        let state_file = Self::state_file_path()?;
        std::fs::write(&state_file, socket_path)?;
        Ok(())
    }

    pub fn load() -> Result<Option<String>> {
        let state_file = Self::state_file_path()?;
        
        if !state_file.exists() {
            return Ok(None);
        }
        
        let socket_path = std::fs::read_to_string(&state_file)?;
        let socket_path = socket_path.trim().to_string();
        
        if socket_path.is_empty() {
            return Ok(None);
        }
        
        Ok(Some(socket_path))
    }

    pub fn clear() -> Result<()> {
        let state_file = Self::state_file_path()?;
        
        if state_file.exists() {
            std::fs::remove_file(&state_file)?;
        }
        
        Ok(())
    }

    pub fn is_connected() -> Result<bool> {
        Ok(Self::load()?.is_some())
    }

    pub fn get_client() -> Result<JsonRpcClient> {
        let socket_path = Self::load()?
            .ok_or_else(|| anyhow!("Not connected to any server. Use 'kaiak connect <socket_path>' first."))?;
        
        Ok(JsonRpcClient::new(socket_path))
    }
}

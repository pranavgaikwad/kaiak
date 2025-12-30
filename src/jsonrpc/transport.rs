//! Transport layer for JSON-RPC communication
//!
//! Implements LSP-style message framing with Content-Length headers
//! and support for different transport types (stdio, IPC, HTTP).

use crate::jsonrpc::protocol::{JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, trace};

/// Transport trait for different communication methods
#[async_trait]
pub trait Transport: Send {
    /// Read a JSON-RPC request from the transport
    async fn read_request(&mut self) -> Result<JsonRpcRequest>;

    /// Write a JSON-RPC response to the transport
    async fn write_response(&mut self, response: JsonRpcResponse) -> Result<()>;

    /// Write a JSON-RPC notification to the transport (server-to-client)
    async fn write_notification(&mut self, notification: JsonRpcNotification) -> Result<()>;

    /// Close the transport connection
    async fn close(&mut self) -> Result<()>;

    /// Get transport description for logging
    fn description(&self) -> &'static str;
}

/// Trait for the write-half of a transport (for sharing with notification senders)
#[async_trait]
pub trait TransportWriter: Send + Sync {
    /// Write a JSON-RPC notification
    async fn write_notification(&mut self, notification: JsonRpcNotification) -> Result<()>;
    
    /// Write a JSON-RPC response
    async fn write_response(&mut self, response: JsonRpcResponse) -> Result<()>;
}

/// Stdio transport using LSP-style Content-Length headers
pub struct StdioTransport {
    reader: BufReader<tokio::io::Stdin>,
    writer: tokio::io::Stdout,
}

impl StdioTransport {
    /// Create a new stdio transport
    pub fn new() -> Self {
        Self {
            reader: BufReader::new(tokio::io::stdin()),
            writer: tokio::io::stdout(),
        }
    }

    /// Read LSP-style message with Content-Length header
    async fn read_lsp_message(&mut self) -> Result<String> {
        let mut content_length = None;

        // Read headers
        loop {
            let mut line = String::new();
            let bytes_read = self.reader.read_line(&mut line).await?;

            if bytes_read == 0 {
                return Err(anyhow!("Connection closed"));
            }

            // Remove trailing \r\n or \n
            let line = line.trim_end();

            // Empty line indicates end of headers
            if line.is_empty() {
                break;
            }

            // Parse Content-Length header
            if let Some(length_str) = line.strip_prefix("Content-Length: ") {
                content_length = Some(length_str.parse::<usize>()?);
            }

            // Ignore other headers (Content-Type, etc.)
            trace!("Received header: {}", line);
        }

        let content_length = content_length
            .ok_or_else(|| anyhow!("Missing Content-Length header"))?;

        // Read the JSON content
        let mut buffer = vec![0u8; content_length];
        self.reader.read_exact(&mut buffer).await?;

        let content = String::from_utf8(buffer)?;
        debug!("Received message: {} bytes", content_length);
        trace!("Message content: {}", content);

        Ok(content)
    }

    /// Write LSP-style message with Content-Length header
    async fn write_lsp_message(&mut self, content: &str) -> Result<()> {
        let content_bytes = content.as_bytes();
        let content_length = content_bytes.len();

        // Write headers
        self.writer
            .write_all(format!("Content-Length: {}\r\n\r\n", content_length).as_bytes())
            .await?;

        // Write content
        self.writer.write_all(content_bytes).await?;
        self.writer.flush().await?;

        debug!("Sent message: {} bytes", content_length);
        trace!("Message content: {}", content);

        Ok(())
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn read_request(&mut self) -> Result<JsonRpcRequest> {
        let content = self.read_lsp_message().await?;
        let request: JsonRpcRequest = serde_json::from_str(&content)?;
        request.validate().map_err(|e| anyhow!("Invalid request: {}", e.message))?;
        Ok(request)
    }

    async fn write_response(&mut self, response: JsonRpcResponse) -> Result<()> {
        let content = serde_json::to_string(&response)?;
        self.write_lsp_message(&content).await
    }

    async fn write_notification(&mut self, notification: JsonRpcNotification) -> Result<()> {
        let content = serde_json::to_string(&notification)?;
        self.write_lsp_message(&content).await
    }

    async fn close(&mut self) -> Result<()> {
        self.writer.flush().await?;
        debug!("Stdio transport closed");
        Ok(())
    }

    fn description(&self) -> &'static str {
        "JSON-RPC over stdin/stdout (LSP-style)"
    }
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}

/// Unix domain socket (IPC) transport for a single connection
pub struct IpcTransport {
    reader: BufReader<tokio::net::unix::OwnedReadHalf>,
    writer: tokio::net::unix::OwnedWriteHalf,
}

impl IpcTransport {
    /// Create a new IPC transport by connecting to an existing socket (client-side)
    pub async fn connect<P: AsRef<Path>>(path: P) -> Result<Self> {
        let stream = tokio::net::UnixStream::connect(path.as_ref()).await?;
        let (read_half, write_half) = stream.into_split();

        Ok(Self {
            reader: BufReader::new(read_half),
            writer: write_half,
        })
    }

    /// Create a new IPC transport from an existing Unix stream (from accepted connection)
    pub fn from_stream(stream: tokio::net::UnixStream) -> Self {
        let (read_half, write_half) = stream.into_split();

        Self {
            reader: BufReader::new(read_half),
            writer: write_half,
        }
    }

    /// Read LSP-style message over Unix socket
    async fn read_lsp_message(&mut self) -> Result<String> {
        let mut content_length = None;

        // Read headers
        loop {
            let mut line = String::new();
            let bytes_read = self.reader.read_line(&mut line).await?;

            if bytes_read == 0 {
                return Err(anyhow!("Connection closed"));
            }

            let line = line.trim_end();

            if line.is_empty() {
                break;
            }

            if let Some(length_str) = line.strip_prefix("Content-Length: ") {
                content_length = Some(length_str.parse::<usize>()?);
            }
        }

        let content_length = content_length
            .ok_or_else(|| anyhow!("Missing Content-Length header"))?;

        // Read the JSON content
        let mut buffer = vec![0u8; content_length];
        self.reader.read_exact(&mut buffer).await?;

        let content = String::from_utf8(buffer)?;
        debug!("Received IPC message: {} bytes", content_length);

        Ok(content)
    }

    /// Write LSP-style message over Unix socket
    async fn write_lsp_message(&mut self, content: &str) -> Result<()> {
        let content_bytes = content.as_bytes();
        let content_length = content_bytes.len();

        // Write headers
        self.writer
            .write_all(format!("Content-Length: {}\r\n\r\n", content_length).as_bytes())
            .await?;

        // Write content
        self.writer.write_all(content_bytes).await?;

        debug!("Sent IPC message: {} bytes", content_length);
        Ok(())
    }
}

#[async_trait]
impl Transport for IpcTransport {
    async fn read_request(&mut self) -> Result<JsonRpcRequest> {
        let content = self.read_lsp_message().await?;
        let request: JsonRpcRequest = serde_json::from_str(&content)?;
        request.validate().map_err(|e| anyhow!("Invalid request: {}", e.message))?;
        Ok(request)
    }

    async fn write_response(&mut self, response: JsonRpcResponse) -> Result<()> {
        let content = serde_json::to_string(&response)?;
        self.write_lsp_message(&content).await
    }

    async fn write_notification(&mut self, notification: JsonRpcNotification) -> Result<()> {
        let content = serde_json::to_string(&notification)?;
        self.write_lsp_message(&content).await
    }

    async fn close(&mut self) -> Result<()> {
        self.writer.shutdown().await?;
        debug!("IPC transport closed");
        Ok(())
    }

    fn description(&self) -> &'static str {
        "JSON-RPC over Unix domain socket (LSP-style)"
    }
}

/// Unix domain socket server transport that listens for connections
/// 
/// This wraps a UnixListener and accepts connections one at a time.
/// Each accepted connection is handled sequentially.
pub struct IpcServerTransport {
    listener: tokio::net::UnixListener,
    socket_path: String,
    current_connection: Option<IpcTransport>,
}

impl IpcServerTransport {
    /// Bind to a Unix socket path and start listening
    pub async fn bind<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_ref = path.as_ref();
        let socket_path = path_ref.to_string_lossy().to_string();
        
        // Remove existing socket file if it exists
        if path_ref.exists() {
            std::fs::remove_file(path_ref)
                .map_err(|e| anyhow!("Failed to remove existing socket file: {}", e))?;
        }
        
        let listener = tokio::net::UnixListener::bind(path_ref)
            .map_err(|e| anyhow!("Failed to bind to socket {}: {}", socket_path, e))?;
        
        debug!("IPC server listening on: {}", socket_path);
        
        Ok(Self {
            listener,
            socket_path,
            current_connection: None,
        })
    }
    
    /// Accept a new connection (blocks until a client connects)
    async fn accept_connection(&mut self) -> Result<()> {
        debug!("Waiting for client connection on {}", self.socket_path);
        
        let (stream, _addr) = self.listener.accept().await
            .map_err(|e| anyhow!("Failed to accept connection: {}", e))?;
        
        debug!("Client connected to {}", self.socket_path);
        self.current_connection = Some(IpcTransport::from_stream(stream));
        
        Ok(())
    }
    
    /// Ensure we have an active connection, accepting one if needed
    async fn ensure_connection(&mut self) -> Result<&mut IpcTransport> {
        if self.current_connection.is_none() {
            self.accept_connection().await?;
        }
        
        Ok(self.current_connection.as_mut().unwrap())
    }
}

#[async_trait]
impl Transport for IpcServerTransport {
    async fn read_request(&mut self) -> Result<JsonRpcRequest> {
        loop {
            let transport = self.ensure_connection().await?;
            
            match transport.read_request().await {
                Ok(request) => return Ok(request),
                Err(e) => {
                    // Connection closed or error - drop this connection and wait for a new one
                    debug!("Connection error (will accept new connection): {}", e);
                    self.current_connection = None;
                    // Continue loop to accept next connection
                }
            }
        }
    }

    async fn write_response(&mut self, response: JsonRpcResponse) -> Result<()> {
        let transport = self.current_connection.as_mut()
            .ok_or_else(|| anyhow!("No active connection"))?;
        transport.write_response(response).await
    }

    async fn write_notification(&mut self, notification: JsonRpcNotification) -> Result<()> {
        let transport = self.current_connection.as_mut()
            .ok_or_else(|| anyhow!("No active connection"))?;
        transport.write_notification(notification).await
    }

    async fn close(&mut self) -> Result<()> {
        if let Some(ref mut transport) = self.current_connection {
            transport.close().await?;
        }
        
        // Clean up socket file
        if Path::new(&self.socket_path).exists() {
            std::fs::remove_file(&self.socket_path)?;
        }
        
        debug!("IPC server transport closed: {}", self.socket_path);
        Ok(())
    }

    fn description(&self) -> &'static str {
        "JSON-RPC server over Unix domain socket (LSP-style)"
    }
}

/// Transport configuration
#[derive(Debug, Clone)]
pub enum TransportConfig {
    /// Standard input/output with LSP message framing
    Stdio,
    /// Unix domain socket with specified path
    UnixSocket { path: String },
}

impl TransportConfig {
    /// Create a server-side transport from configuration
    /// 
    /// For Unix sockets, this binds and listens (server mode).
    /// For stdio, this creates a standard input/output transport.
    pub async fn create_transport(&self) -> Result<Box<dyn Transport>> {
        match self {
            TransportConfig::Stdio => {
                Ok(Box::new(StdioTransport::new()))
            }
            TransportConfig::UnixSocket { path } => {
                // Server mode: bind and listen
                let transport = IpcServerTransport::bind(path).await?;
                Ok(Box::new(transport))
            }
        }
    }
    
    /// Create a client-side transport from configuration
    /// 
    /// For Unix sockets, this connects to an existing server.
    pub async fn create_client_transport(&self) -> Result<Box<dyn Transport>> {
        match self {
            TransportConfig::Stdio => {
                Ok(Box::new(StdioTransport::new()))
            }
            TransportConfig::UnixSocket { path } => {
                // Client mode: connect to existing server
                let transport = IpcTransport::connect(path).await?;
                Ok(Box::new(transport))
            }
        }
    }

    /// Create transport configuration from InitConfig
    pub fn from_init_config(init_config: &crate::models::configuration::InitConfig) -> Result<Self> {
        match init_config.transport.as_str() {
            "stdio" => Ok(TransportConfig::Stdio),
            "socket" => {
                let path = init_config.socket_path
                    .clone()
                    .ok_or_else(|| anyhow!("Socket path is required when using socket transport"))?;
                Ok(TransportConfig::UnixSocket { path })
            }
            other => Err(anyhow!("Unsupported transport type: {}", other)),
        }
    }

    /// Get transport description
    pub fn description(&self) -> String {
        match self {
            TransportConfig::Stdio => "stdin/stdout".to_string(),
            TransportConfig::UnixSocket { path } => format!("Unix socket ({})", path),
        }
    }
}

/// Helper functions for working with LSP message format
pub mod lsp_format {
    use super::*;

    /// Parse LSP message headers from a string
    pub fn parse_headers(headers: &str) -> Result<usize> {
        for line in headers.lines() {
            let line = line.trim();
            if let Some(length_str) = line.strip_prefix("Content-Length: ") {
                return Ok(length_str.parse()?);
            }
        }
        Err(anyhow!("Content-Length header not found"))
    }

    /// Format message with LSP headers
    pub fn format_message(content: &str) -> String {
        format!("Content-Length: {}\r\n\r\n{}", content.len(), content)
    }

    /// Validate LSP message format
    pub fn validate_message(raw_message: &str) -> Result<&str> {
        let header_end = raw_message
            .find("\r\n\r\n")
            .ok_or_else(|| anyhow!("Invalid LSP message format: missing header separator"))?;

        let headers = &raw_message[..header_end];
        let content = &raw_message[header_end + 4..];

        let expected_length = parse_headers(headers)?;
        let actual_length = content.len();

        if expected_length != actual_length {
            return Err(anyhow!(
                "Content length mismatch: expected {}, got {}",
                expected_length,
                actual_length
            ));
        }

        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsp_format() {
        let content = r#"{"jsonrpc":"2.0","method":"test","id":1}"#;
        let formatted = lsp_format::format_message(content);

        assert!(formatted.contains("Content-Length: 40"));
        assert!(formatted.ends_with(content));
    }

    #[test]
    fn test_header_parsing() {
        let headers = "Content-Length: 42\r\nContent-Type: application/json";
        let length = lsp_format::parse_headers(headers).unwrap();
        assert_eq!(length, 42);
    }

    #[test]
    fn test_message_validation() {
        let message = "Content-Length: 40\r\n\r\n{\"jsonrpc\":\"2.0\",\"method\":\"test\",\"id\":1}";
        let content = lsp_format::validate_message(message).unwrap();
        assert_eq!(content, r#"{"jsonrpc":"2.0","method":"test","id":1}"#);
    }

    #[test]
    fn test_transport_config() {
        let config = TransportConfig::Stdio;
        assert_eq!(config.description(), "stdin/stdout");

        let config = TransportConfig::UnixSocket {
            path: "/tmp/test.sock".to_string(),
        };
        assert!(config.description().contains("/tmp/test.sock"));
    }
}
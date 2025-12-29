//! Kaiak JSON-RPC server implementation using custom JSON-RPC 2.0

use anyhow::Result;
use std::sync::Arc;
use tracing::{error, info, debug};

use crate::{
    jsonrpc::{
        create_kaiak_server,
        transport::TransportConfig as JsonRpcTransportConfig,
    },
    models::configuration::ServerConfig,
};

/// Transport configuration for the server
#[derive(Debug, Clone)]
pub enum TransportConfig {
    /// Standard input/output transport
    Stdio,
    /// Unix domain socket transport
    UnixSocket { path: String },
}

impl From<TransportConfig> for JsonRpcTransportConfig {
    fn from(config: TransportConfig) -> Self {
        match config {
            TransportConfig::Stdio => JsonRpcTransportConfig::Stdio,
            TransportConfig::UnixSocket { path } => JsonRpcTransportConfig::UnixSocket { path },
        }
    }
}

/// Create and start the Kaiak JSON-RPC server with the specified configuration
pub async fn start_server(
    server_config: Arc<ServerConfig>,
    transport_config: Option<TransportConfig>,
) -> Result<()> {
    info!("Starting Kaiak JSON-RPC server");

    // Use transport from configuration if not explicitly provided
    let transport = if let Some(transport) = transport_config {
        transport.into()
    } else {
        JsonRpcTransportConfig::from_init_config(&server_config.init_config)?
    };

    // Override the transport in server config for consistency
    let mut config_copy = (*server_config).clone();
    match &transport {
        JsonRpcTransportConfig::Stdio => {
            config_copy.init_config.transport = "stdio".to_string();
            config_copy.init_config.socket_path = None;
        },
        JsonRpcTransportConfig::UnixSocket { path } => {
            config_copy.init_config.transport = "socket".to_string();
            config_copy.init_config.socket_path = Some(path.clone());
        },
        JsonRpcTransportConfig::Http { .. } => {
            // HTTP transport not supported yet
            anyhow::bail!("HTTP transport not implemented yet");
        },
    }
    let server_config = Arc::new(config_copy);

    // Create and start JSON-RPC server
    let session_manager = Arc::new(crate::agent::GooseAgentManager::new());
    let mut kaiak_server = create_kaiak_server(server_config.clone(), session_manager).await?;

    info!("Starting Kaiak JSON-RPC server with {} transport", transport.description());
    kaiak_server.start().await?;

    // Keep the server running indefinitely
    // In a real implementation, you might want to handle shutdown signals
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        if !kaiak_server.is_running().await {
            error!("Server has stopped unexpectedly");
            break;
        }
    }

    info!("Kaiak JSON-RPC server stopped");
    Ok(())
}

/// Start server with stdio transport (convenience function)
pub async fn start_stdio_server(server_config: Arc<ServerConfig>) -> Result<()> {
    start_server(server_config, Some(TransportConfig::Stdio)).await
}

/// Start server with Unix socket transport (convenience function)
pub async fn start_unix_socket_server(
    server_config: Arc<ServerConfig>,
    socket_path: String,
) -> Result<()> {
    start_server(server_config, Some(TransportConfig::UnixSocket { path: socket_path })).await
}

/// Create a default server configuration for testing and development
pub fn create_default_server_config() -> ServerConfig {
    ServerConfig::default()
}

/// Validate server configuration before starting
pub fn validate_server_config(config: &ServerConfig) -> Result<()> {
    config.validate()?;

    // Additional validation specific to server startup
    match config.init_config.transport.as_str() {
        "stdio" => {
            debug!("Using stdio transport - no additional validation needed");
        },
        "socket" => {
            if config.init_config.socket_path.is_none() {
                anyhow::bail!("Socket path is required when using socket transport");
            }

            let socket_path = config.init_config.socket_path.as_ref().unwrap();
            if socket_path.is_empty() {
                anyhow::bail!("Socket path cannot be empty");
            }

            debug!("Using Unix socket transport: {}", socket_path);
        },
        other => {
            anyhow::bail!("Unsupported transport type: {}", other);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::configuration::{InitConfig, BaseConfig};

    fn create_test_server_config() -> ServerConfig {
        ServerConfig {
            init_config: InitConfig {
                transport: "stdio".to_string(),
                socket_path: None,
                log_level: "info".to_string(),
                max_concurrent_sessions: 10,
            },
            base_config: BaseConfig::default(),
        }
    }

    #[test]
    fn test_transport_config_conversion() {
        let stdio_config = TransportConfig::Stdio;
        let json_rpc_config: JsonRpcTransportConfig = stdio_config.into();
        assert!(matches!(json_rpc_config, JsonRpcTransportConfig::Stdio));

        let socket_config = TransportConfig::UnixSocket {
            path: "/tmp/test.sock".to_string(),
        };
        let json_rpc_config: JsonRpcTransportConfig = socket_config.into();

        match json_rpc_config {
            JsonRpcTransportConfig::UnixSocket { path } => {
                assert_eq!(path, "/tmp/test.sock");
            },
            _ => assert!(false, "Expected UnixSocket transport"),
        }
    }

    #[test]
    fn test_validate_server_config() {
        let config = create_test_server_config();
        assert!(validate_server_config(&config).is_ok());

        let mut invalid_config = config.clone();
        invalid_config.init_config.transport = "invalid".to_string();
        assert!(validate_server_config(&invalid_config).is_err());

        let mut socket_config = config.clone();
        socket_config.init_config.transport = "socket".to_string();
        socket_config.init_config.socket_path = None;
        assert!(validate_server_config(&socket_config).is_err());

        socket_config.init_config.socket_path = Some("/tmp/test.sock".to_string());
        assert!(validate_server_config(&socket_config).is_ok());
    }

    #[test]
    fn test_create_default_server_config() {
        let config = create_default_server_config();
        assert!(validate_server_config(&config).is_ok());
    }
}
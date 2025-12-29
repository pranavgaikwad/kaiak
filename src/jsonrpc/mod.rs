//! Custom JSON-RPC 2.0 implementation for Kaiak
//!
//! This module provides a focused JSON-RPC 2.0 implementation designed specifically
//! for Kaiak's needs, supporting LSP-style message framing over various transports.
//!
//! Features:
//! - LSP-style Content-Length headers for reliable message framing
//! - Support for stdio, IPC (Unix sockets), and future HTTP transports
//! - Clean integration with Kaiak's handler system
//! - Async/await native design with tokio
//! - No external JSON-RPC dependencies

// Core JSON-RPC implementation
pub mod protocol;
pub mod transport;
pub mod server;

pub mod methods;
pub mod core;

// Re-export main types for convenient use
pub use protocol::{JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, JsonRpcError};
pub use transport::{Transport, TransportConfig, StdioTransport, IpcTransport};
pub use server::{
    JsonRpcServer, ServerBuilder, MethodHandler, 
    StreamingMethodHandler, NotificationSender, NotificationReceiver,
};

// Legacy re-exports for existing code compatibility
pub use methods::{GENERATE_FIX, DELETE_SESSION};
pub use core::{KaiakRequest, KaiakResponse, ResponseMetadata};

/// Version of the JSON-RPC protocol used by Kaiak
pub const JSONRPC_VERSION: &str = "2.0";

/// Default timeout for client requests (in seconds)
pub const DEFAULT_REQUEST_TIMEOUT: u64 = 300;

/// Default maximum number of concurrent connections for server
pub const DEFAULT_MAX_CONNECTIONS: usize = 10;

/// Helper function to create a server with Kaiak handlers
pub async fn create_kaiak_server(
    server_config: std::sync::Arc<crate::models::configuration::ServerConfig>,
    agent_manager: std::sync::Arc<crate::agent::GooseAgentManager>,
) -> anyhow::Result<JsonRpcServer> {
    use crate::jsonrpc::transport::TransportConfig;

    // Determine transport from config
    let transport_config = match server_config.init_config.transport.as_str() {
        "stdio" => TransportConfig::Stdio,
        "socket" => {
            let socket_path = server_config.init_config.socket_path
                .clone()
                .unwrap_or_else(|| "/tmp/kaiak.sock".to_string());
            TransportConfig::UnixSocket { path: socket_path }
        }
        _ => {
            tracing::warn!("Unknown transport '{}', defaulting to stdio", server_config.init_config.transport);
            TransportConfig::Stdio
        }
    };

    // Create server with appropriate transport
    let server = JsonRpcServer::new(transport_config).await?;

    // Register Kaiak methods
    register_kaiak_methods(&server, agent_manager, std::sync::Arc::new(server_config.base_config.clone())).await?;

    Ok(server)
}

/// Register all Kaiak JSON-RPC methods with the server
pub async fn register_kaiak_methods(
    server: &JsonRpcServer,
    agent_manager: std::sync::Arc<crate::agent::GooseAgentManager>,
    base_config: std::sync::Arc<crate::models::configuration::BaseConfig>,
) -> anyhow::Result<()> {
    use crate::handlers::{
        generate_fix::{GenerateFixRequest, GenerateFixHandler},
        delete_session::{DeleteSessionRequest, DeleteSessionHandler},
    };
    
    // Register generate_fix method (streaming - sends notifications during execution)
    {
        let agent_manager = agent_manager.clone();
        server.register_streaming_method(
            GENERATE_FIX.to_string(),
            move |params, notifier| {
                let agent_manager = agent_manager.clone();
                let base_config = base_config.clone();
                async move {
                    let params_value = params.unwrap_or(serde_json::Value::Null);
                    let kaiak_request: KaiakRequest<GenerateFixRequest> =
                        serde_json::from_value(params_value)
                        .map_err(|e| crate::jsonrpc::JsonRpcError::custom(
                            crate::jsonrpc::protocol::error_codes::INVALID_PARAMS,
                            format!("Failed to parse parameters: {}", e),
                            None,
                        ))?;

                    let handler = GenerateFixHandler::new(agent_manager, base_config.clone());
                    let request_inner = kaiak_request.inner.clone();
                    let response = handler.handle_generate_fix(request_inner, notifier).await
                        .map_err(|e| crate::jsonrpc::JsonRpcError::from(e))?;

                    let kaiak_response = KaiakResponse::from_request(response, &kaiak_request);
                    serde_json::to_value(kaiak_response)
                        .map_err(|e| crate::jsonrpc::JsonRpcError::custom(
                            crate::jsonrpc::protocol::error_codes::INTERNAL_ERROR,
                            format!("Failed to serialize response: {}", e),
                            None,
                        ))
                }
            },
        ).await?;
    }

    // Register delete_session method (non-streaming)
    {
        let agent_manager = agent_manager.clone();
        server.register_async_method(
            DELETE_SESSION.to_string(),
            move |params| {
                let agent_manager = agent_manager.clone();
                async move {
                    let params_value = params.unwrap_or(serde_json::Value::Null);
                    let kaiak_request: KaiakRequest<DeleteSessionRequest> =
                        serde_json::from_value(params_value)
                        .map_err(|e| crate::jsonrpc::JsonRpcError::custom(
                            crate::jsonrpc::protocol::error_codes::INVALID_PARAMS,
                            format!("Failed to parse parameters: {}", e),
                            None,
                        ))?;

                    let handler = DeleteSessionHandler::new(agent_manager);
                    let request_inner = kaiak_request.inner.clone();
                    let response = handler.handle_delete_session(request_inner).await
                        .map_err(|e| crate::jsonrpc::JsonRpcError::from(e))?;

                    let kaiak_response = KaiakResponse::from_request(response, &kaiak_request);
                    serde_json::to_value(kaiak_response)
                        .map_err(|e| crate::jsonrpc::JsonRpcError::custom(
                            crate::jsonrpc::protocol::error_codes::INTERNAL_ERROR,
                            format!("Failed to serialize response: {}", e),
                            None,
                        ))
                }
            },
        ).await?;
    }

    tracing::info!("Registered {} Kaiak JSON-RPC methods", 2);
    Ok(())
}
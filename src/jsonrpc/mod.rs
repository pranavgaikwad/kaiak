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

pub use protocol::{JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, JsonRpcError};
pub use transport::{Transport, TransportConfig, StdioTransport, IpcTransport};
pub use server::{
    JsonRpcServer, ServerBuilder, MethodHandler, 
    StreamingMethodHandler, NotificationSender, NotificationReceiver,
};

pub use methods::{GENERATE_FIX, DELETE_SESSION};
pub use core::{KaiakRequest, KaiakResponse, ResponseMetadata};

pub const JSONRPC_VERSION: &str = "2.0";
pub const DEFAULT_REQUEST_TIMEOUT: u64 = 300;
pub const DEFAULT_MAX_CONNECTIONS: usize = 10;

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

    let server = JsonRpcServer::new(transport_config).await?;

    register_kaiak_methods(&server, agent_manager, std::sync::Arc::new(server_config.base_config.clone())).await?;

    Ok(server)
}

/// Register all Kaiak JSON-RPC methods with the server
/// 
/// Methods accept raw request types directly (no wrapper required).
/// For example, `generate_fix` accepts:
/// ```json
/// {
///   "session_id": "...",
///   "incidents": [...],
///   "agent_config": {...}
/// }
/// ```
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
                    
                    // Parse directly as GenerateFixRequest (no wrapper)
                    let request: GenerateFixRequest = serde_json::from_value(params_value.clone())
                        .map_err(|e| {
                            create_parse_error::<GenerateFixRequest>(&e, &params_value)
                        })?;

                    let handler = GenerateFixHandler::new(agent_manager, base_config.clone());
                    let response = handler.handle_generate_fix(request, notifier).await
                        .map_err(|e| crate::jsonrpc::JsonRpcError::from(e))?;

                    // Return raw response (no wrapper)
                    serde_json::to_value(response)
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
                    
                    // Parse directly as DeleteSessionRequest (no wrapper)
                    let request: DeleteSessionRequest = serde_json::from_value(params_value.clone())
                        .map_err(|e| {
                            create_parse_error::<DeleteSessionRequest>(&e, &params_value)
                        })?;

                    let handler = DeleteSessionHandler::new(agent_manager);
                    let response = handler.handle_delete_session(request).await
                        .map_err(|e| crate::jsonrpc::JsonRpcError::from(e))?;

                    // Return raw response (no wrapper)
                    serde_json::to_value(response)
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

fn create_parse_error<T>(error: &serde_json::Error, params: &serde_json::Value) -> JsonRpcError {
    let type_name = std::any::type_name::<T>()
        .rsplit("::")
        .next()
        .unwrap_or("Request");
    
    let received_fields: Vec<&str> = match params {
        serde_json::Value::Object(map) => map.keys().map(|s| s.as_str()).collect(),
        _ => vec![],
    };
    
    let hint = if received_fields.is_empty() {
        "No parameters provided".to_string()
    } else {
        format!("Received fields: {}", received_fields.join(", "))
    };
    
    JsonRpcError::custom(
        protocol::error_codes::INVALID_PARAMS,
        format!("Invalid {}: {}. {}", type_name, error, hint),
        Some(serde_json::json!({
            "parse_error": error.to_string(),
            "received": params,
        })),
    )
}
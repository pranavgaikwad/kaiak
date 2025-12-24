use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::{Error, Result as JsonRpcResult};
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tracing::{error, info, debug};

use crate::handlers::{ConfigureHandler, GenerateFixHandler, DeleteSessionHandler};
use crate::server::jsonrpc::{
    methods, error_codes, create_error,
    ConfigureRequest, ConfigureResponse,
    GenerateFixRequest, GenerateFixResponse,
    DeleteSessionRequest, DeleteSessionResponse,
    StreamNotification,
};

/// Main Kaiak LSP server that handles JSON-RPC requests
pub struct KaiakServer {
    client: Client,
    configure_handler: Arc<RwLock<Option<ConfigureHandler>>>,
    generate_fix_handler: Arc<RwLock<Option<GenerateFixHandler>>>,
    delete_session_handler: Arc<RwLock<Option<DeleteSessionHandler>>>,
}

impl KaiakServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            configure_handler: Arc::new(RwLock::new(None)),
            generate_fix_handler: Arc::new(RwLock::new(None)),
            delete_session_handler: Arc::new(RwLock::new(None)),
        }
    }

    /// Initialize handlers after server creation
    async fn ensure_handlers_initialized(&self) -> Result<()> {
        // Placeholder implementation - actual handlers will be created in user story phases
        // For now, we just mark them as "initialized" to satisfy the routing requirements
        debug!("Handler initialization placeholder - actual implementation in user story phases");
        Ok(())
    }

    /// Send a streaming notification to the client
    async fn send_stream_notification(&self, notification: StreamNotification) {
        // For tower-lsp, we'll send custom notifications as "window/showMessage" for now
        // In a real implementation, this would use custom notification types
        let params = tower_lsp::lsp_types::ShowMessageParams {
            typ: tower_lsp::lsp_types::MessageType::INFO,
            message: format!("Stream: {}", serde_json::to_string(&notification).unwrap_or_else(|_| "notification".to_string())),
        };

        self.client.send_notification::<tower_lsp::lsp_types::notification::ShowMessage>(params).await;
    }

    /// Handle configure request
    async fn handle_configure(&self, params: ConfigureRequest) -> JsonRpcResult<ConfigureResponse> {
        if let Err(e) = self.ensure_handlers_initialized().await {
            error!("Failed to initialize handlers: {}", e);
            return Err(create_error(
                error_codes::CONFIGURATION_ERROR,
                "Handler initialization failed",
                None,
            ));
        }

        // Placeholder implementation - actual handler will be implemented in User Story 1
        info!("Configure request received (placeholder implementation)");

        Err(create_error(
            error_codes::CONFIGURATION_ERROR,
            "Configure handler not yet implemented - will be created in User Story 1",
            None,
        ))
    }

    /// Handle delete session request
    async fn handle_delete_session(&self, params: DeleteSessionRequest) -> JsonRpcResult<DeleteSessionResponse> {
        if let Err(e) = self.ensure_handlers_initialized().await {
            error!("Failed to initialize handlers: {}", e);
            return Err(create_error(
                error_codes::SESSION_NOT_FOUND,
                "Handler initialization failed",
                None,
            ));
        }

        // Placeholder implementation - actual handler will be implemented in User Story 2
        info!("Delete session request received (placeholder implementation)");

        Err(create_error(
            error_codes::SESSION_NOT_FOUND,
            "Delete session handler not yet implemented - will be created in User Story 2",
            None,
        ))
    }

    /// Handle fix generation request with streaming
    async fn handle_generate_fix(&self, params: GenerateFixRequest) -> JsonRpcResult<GenerateFixResponse> {
        if let Err(e) = self.ensure_handlers_initialized().await {
            error!("Failed to initialize handlers: {}", e);
            return Err(create_error(
                error_codes::AGENT_INITIALIZATION_FAILED,
                "Handler initialization failed",
                None,
            ));
        }

        // Placeholder implementation - actual handler will be implemented in User Story 3
        info!("Generate fix request received (placeholder implementation)");

        Err(create_error(
            error_codes::AGENT_INITIALIZATION_FAILED,
            "Generate fix handler not yet implemented - will be created in User Story 3",
            None,
        ))
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for KaiakServer {
    async fn initialize(&self, _params: InitializeParams) -> JsonRpcResult<InitializeResult> {
        info!("Kaiak LSP server initializing");

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                // We don't implement traditional LSP features, only custom JSON-RPC methods
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "kaiak".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            ..Default::default()
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        info!("Kaiak LSP server initialized successfully");

        // Initialize handlers in the background
        if let Err(e) = self.ensure_handlers_initialized().await {
            error!("Failed to initialize handlers: {}", e);
        }
    }

    async fn shutdown(&self) -> JsonRpcResult<()> {
        info!("Kaiak LSP server shutting down");
        Ok(())
    }

    async fn did_open(&self, _params: DidOpenTextDocumentParams) {
        // Not used for our use case
    }

    async fn did_change(&self, _params: DidChangeTextDocumentParams) {
        // Not used for our use case
    }

    async fn did_save(&self, _params: DidSaveTextDocumentParams) {
        // Not used for our use case
    }

    async fn did_close(&self, _params: DidCloseTextDocumentParams) {
        // Not used for our use case
    }

    // Custom request handlers for the three-endpoint API
    async fn execute_command(&self, params: ExecuteCommandParams) -> JsonRpcResult<Option<Value>> {
        match params.command.as_str() {
            methods::CONFIGURE => {
                if let Some(arg) = params.arguments.get(0) {
                    match serde_json::from_value::<ConfigureRequest>(arg.clone()) {
                        Ok(req) => self.handle_configure(req).await.map(|resp| {
                            Some(serde_json::to_value(resp).unwrap_or(Value::Null))
                        }),
                        Err(e) => Err(create_error(
                            error_codes::RESPONSE_VALIDATION_FAILED,
                            &format!("Invalid request format: {}", e),
                            None,
                        )),
                    }
                } else {
                    Err(create_error(
                        error_codes::RESPONSE_VALIDATION_FAILED,
                        "Missing request parameters",
                        None,
                    ))
                }
            }
            methods::GENERATE_FIX => {
                if let Some(arg) = params.arguments.get(0) {
                    match serde_json::from_value::<GenerateFixRequest>(arg.clone()) {
                        Ok(req) => self.handle_generate_fix(req).await.map(|resp| {
                            Some(serde_json::to_value(resp).unwrap_or(Value::Null))
                        }),
                        Err(e) => Err(create_error(
                            error_codes::RESPONSE_VALIDATION_FAILED,
                            &format!("Invalid request format: {}", e),
                            None,
                        )),
                    }
                } else {
                    Err(create_error(
                        error_codes::RESPONSE_VALIDATION_FAILED,
                        "Missing request parameters",
                        None,
                    ))
                }
            }
            methods::DELETE_SESSION => {
                if let Some(arg) = params.arguments.get(0) {
                    match serde_json::from_value::<DeleteSessionRequest>(arg.clone()) {
                        Ok(req) => self.handle_delete_session(req).await.map(|resp| {
                            Some(serde_json::to_value(resp).unwrap_or(Value::Null))
                        }),
                        Err(e) => Err(create_error(
                            error_codes::RESPONSE_VALIDATION_FAILED,
                            &format!("Invalid request format: {}", e),
                            None,
                        )),
                    }
                } else {
                    Err(create_error(
                        error_codes::RESPONSE_VALIDATION_FAILED,
                        "Missing request parameters",
                        None,
                    ))
                }
            }
            _ => Err(Error {
                code: tower_lsp::jsonrpc::ErrorCode::MethodNotFound,
                message: format!("Unknown command: {} - only kaiak/configure, kaiak/generate_fix, and kaiak/delete_session are supported", params.command).into(),
                data: None,
            }),
        }
    }
}

/// Create and start the Kaiak LSP server with the specified transport
pub async fn start_server(transport_config: crate::server::TransportConfig) -> Result<()> {
    info!("Starting Kaiak LSP server");

    match transport_config {
        crate::server::TransportConfig::Stdio => {
            info!("Starting server with stdio transport");

            let stdin = tokio::io::stdin();
            let stdout = tokio::io::stdout();

            let (service, socket) = LspService::new(|client| KaiakServer::new(client));
            Server::new(stdin, stdout, socket).serve(service).await;

            info!("Kaiak LSP server stopped");
            Ok(())
        }
        crate::server::TransportConfig::UnixSocket { path } => {
            #[cfg(unix)]
            {
                use tokio::net::UnixListener;

                info!("Starting server with Unix socket: {}", path);

                // Remove existing socket file if it exists
                let _ = std::fs::remove_file(&path);

                let listener = UnixListener::bind(&path)?;
                info!("Unix socket server listening at: {}", path);

                loop {
                    let (stream, _) = listener.accept().await?;
                    let (read, write) = stream.into_split();

                    let (service, socket) = LspService::new(|client| KaiakServer::new(client));

                    tokio::spawn(async move {
                        Server::new(read, write, socket).serve(service).await;
                    });
                }
            }

            #[cfg(not(unix))]
            {
                anyhow::bail!("Unix sockets are not supported on this platform");
            }
        }
    }
}
use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::{Error, Result as JsonRpcResult};
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tracing::{error, info, debug};

use crate::handlers::{FixGenerationHandler, LifecycleHandler};
use crate::models::{FixGenerationRequest, Incident};
use crate::server::jsonrpc::{
    methods, error_codes, create_error,
    CreateSessionRequest, CreateSessionResponse,
    TerminateSessionRequest, TerminateSessionResponse,
    GenerateFixRequest, GenerateFixResponse,
    StreamNotification,
    session_not_found_error,
};

/// Main Kaiak LSP server that handles JSON-RPC requests
pub struct KaiakServer {
    client: Client,
    fix_handler: Arc<RwLock<Option<FixGenerationHandler>>>,
    lifecycle_handler: Arc<RwLock<Option<LifecycleHandler>>>,
}

impl KaiakServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            fix_handler: Arc::new(RwLock::new(None)),
            lifecycle_handler: Arc::new(RwLock::new(None)),
        }
    }

    /// Initialize handlers after server creation
    async fn ensure_handlers_initialized(&self) -> Result<()> {
        // Initialize fix generation handler
        {
            let mut fix_handler = self.fix_handler.write().await;
            if fix_handler.is_none() {
                *fix_handler = Some(FixGenerationHandler::new().await?);
                debug!("Fix generation handler initialized");
            }
        }

        // Initialize lifecycle handler
        {
            let mut lifecycle_handler = self.lifecycle_handler.write().await;
            if lifecycle_handler.is_none() {
                *lifecycle_handler = Some(LifecycleHandler::new().await?);
                debug!("Lifecycle handler initialized");
            }
        }

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

    /// Handle create session request
    async fn handle_create_session(&self, params: CreateSessionRequest) -> JsonRpcResult<CreateSessionResponse> {
        if let Err(e) = self.ensure_handlers_initialized().await {
            error!("Failed to initialize handlers: {}", e);
            return Err(create_error(
                error_codes::SESSION_CREATION_FAILED,
                "Handler initialization failed",
                None,
            ));
        }

        let lifecycle_handler = self.lifecycle_handler.read().await;
        let handler = lifecycle_handler.as_ref().unwrap();

        match handler.create_session(params.workspace_path.clone(), params.session_name.clone()).await {
            Ok(ai_session) => {
                info!("Session created: {}", ai_session.id);
                Ok(CreateSessionResponse {
                    session_id: ai_session.id,
                    status: "ready".to_string(),
                    created_at: chrono::Utc::now().to_rfc3339(),
                })
            }
            Err(e) => {
                error!("Failed to create session for workspace {}: {}", params.workspace_path, e);
                Err(create_error(
                    error_codes::SESSION_CREATION_FAILED,
                    &format!("Session creation failed: {}", e),
                    None,
                ))
            }
        }
    }

    /// Handle terminate session request
    async fn handle_terminate_session(&self, params: TerminateSessionRequest) -> JsonRpcResult<TerminateSessionResponse> {
        let lifecycle_handler = self.lifecycle_handler.read().await;
        let handler = match lifecycle_handler.as_ref() {
            Some(h) => h,
            None => return Err(session_not_found_error(&params.session_id)),
        };

        match handler.terminate_session(&params.session_id).await {
            Ok(()) => {
                info!("Session terminated: {}", params.session_id);
                Ok(TerminateSessionResponse {
                    session_id: params.session_id,
                    status: "terminated".to_string(),
                    message_count: 0, // TODO: Track actual message count
                    terminated_at: chrono::Utc::now().to_rfc3339(),
                })
            }
            Err(e) => {
                error!("Failed to terminate session {}: {}", params.session_id, e);
                Err(session_not_found_error(&params.session_id))
            }
        }
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

        let fix_handler = self.fix_handler.read().await;
        let handler = fix_handler.as_ref().unwrap();

        // Convert JSON incidents to Incident structs
        let incidents: Result<Vec<Incident>, _> = params.incidents.iter()
            .map(|inc| serde_json::from_value(inc.clone()))
            .collect();

        let incidents = match incidents {
            Ok(incidents) => incidents,
            Err(e) => {
                error!("Failed to parse incidents: {}", e);
                return Err(create_error(
                    error_codes::RESPONSE_VALIDATION_FAILED,
                    &format!("Invalid incident format: {}", e),
                    None,
                ));
            }
        };

        // Capture incident count before move
        let incident_count = incidents.len();

        // Create fix generation request
        let fix_request = FixGenerationRequest::new(
            params.session_id.clone(),
            incidents,
            "unknown".to_string(), // TODO: Extract from migration_context
        );

        match handler.handle_request(&fix_request).await {
            Ok((request_id, mut receiver)) => {
                // Spawn a task to handle streaming messages
                let client = self.client.clone();
                let session_id = params.session_id.clone();
                let request_id_clone = request_id.clone();

                tokio::spawn(async move {
                    while let Some(message) = receiver.recv().await {
                        let notification = StreamNotification {
                            session_id: session_id.clone(),
                            request_id: Some(request_id_clone.clone()),
                            message_id: uuid::Uuid::new_v4().to_string(),
                            timestamp: chrono::Utc::now().to_rfc3339(),
                            content: serde_json::to_value(&message).unwrap_or(Value::Null),
                        };

                        // Send as a standard LSP notification for now
                        let params = tower_lsp::lsp_types::ShowMessageParams {
                            typ: tower_lsp::lsp_types::MessageType::INFO,
                            message: format!("Stream: {}", serde_json::to_string(&notification).unwrap_or_else(|_| "notification".to_string())),
                        };

                        client.send_notification::<tower_lsp::lsp_types::notification::ShowMessage>(params).await;
                    }
                });

                info!("Fix generation request started: {}", request_id);
                Ok(GenerateFixResponse {
                    request_id,
                    session_id: params.session_id,
                    status: "processing".to_string(),
                    incident_count,
                    created_at: chrono::Utc::now().to_rfc3339(),
                })
            }
            Err(e) => {
                error!("Failed to process fix generation request: {}", e);
                Err(create_error(
                    error_codes::AGENT_INITIALIZATION_FAILED,
                    &format!("Fix generation failed: {}", e),
                    None,
                ))
            }
        }
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

    // Custom request handlers would be implemented via execute_command or custom requests
    async fn execute_command(&self, params: ExecuteCommandParams) -> JsonRpcResult<Option<Value>> {
        match params.command.as_str() {
            methods::SESSION_CREATE => {
                if let Some(arg) = params.arguments.get(0) {
                    match serde_json::from_value::<CreateSessionRequest>(arg.clone()) {
                        Ok(req) => self.handle_create_session(req).await.map(|resp| {
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
            methods::SESSION_TERMINATE => {
                if let Some(arg) = params.arguments.get(0) {
                    match serde_json::from_value::<TerminateSessionRequest>(arg.clone()) {
                        Ok(req) => self.handle_terminate_session(req).await.map(|resp| {
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
            methods::FIX_GENERATE => {
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
            _ => Err(Error {
                code: tower_lsp::jsonrpc::ErrorCode::MethodNotFound,
                message: format!("Unknown command: {}", params.command).into(),
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
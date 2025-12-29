//! JSON-RPC server implementation with method dispatch
//!
//! Provides a clean server that can handle JSON-RPC requests over various transports
//! and dispatch them to registered method handlers.

use crate::jsonrpc::{
    protocol::{JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, JsonRpcError},
    transport::{Transport, TransportConfig},
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::future::BoxFuture;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, info, instrument, trace, warn};

/// Sender for streaming notifications from handlers
pub type NotificationSender = mpsc::UnboundedSender<JsonRpcNotification>;

/// Receiver for notifications (used internally by the server)
pub type NotificationReceiver = mpsc::UnboundedReceiver<JsonRpcNotification>;

/// Takes JSON parameters and returns a JSON result
pub type MethodHandler = Arc<
    dyn Fn(Option<serde_json::Value>) -> BoxFuture<'static, Result<serde_json::Value, JsonRpcError>>
        + Send
        + Sync,
>;

/// Streaming method handler function signature
/// Takes JSON parameters and a notification sender for streaming updates
pub type StreamingMethodHandler = Arc<
    dyn Fn(Option<serde_json::Value>, NotificationSender) -> BoxFuture<'static, Result<serde_json::Value, JsonRpcError>>
        + Send
        + Sync,
>;

/// Internal handler storage - can be either legacy or streaming
#[derive(Clone)]
enum HandlerType {
    NonStreaming(MethodHandler),
    Streaming(StreamingMethodHandler),
}

/// JSON-RPC server with notification streaming support
pub struct JsonRpcServer {
    transport: Box<dyn Transport>,
    methods: Arc<Mutex<HashMap<String, HandlerType>>>,
    running: Arc<Mutex<bool>>,
    /// Sender for notifications - clone and pass to handlers
    notification_tx: NotificationSender,
    /// Receiver for notifications - consumed by the server loop
    notification_rx: Arc<Mutex<NotificationReceiver>>,
}

impl JsonRpcServer {
    /// Create a new JSON-RPC server with the specified transport
    pub async fn new(transport_config: TransportConfig) -> Result<Self> {
        let transport = transport_config.create_transport().await?;
        let (notification_tx, notification_rx) = mpsc::unbounded_channel();

        Ok(Self {
            transport,
            methods: Arc::new(Mutex::new(HashMap::new())),
            running: Arc::new(Mutex::new(false)),
            notification_tx,
            notification_rx: Arc::new(Mutex::new(notification_rx)),
        })
    }

    /// Get a clone of the notification sender
    /// 
    /// Pass this to handlers that need to stream notifications back to the client.
    pub fn notification_sender(&self) -> NotificationSender {
        self.notification_tx.clone()
    }

    /// Register a method handler (legacy, without notification support)
    pub async fn register_method<F, Fut>(
        &self,
        method_name: String,
        handler: F,
    ) -> Result<()>
    where
        F: Fn(Option<serde_json::Value>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<serde_json::Value, JsonRpcError>> + Send + 'static,
    {
        let wrapped_handler: MethodHandler = Arc::new(move |params| {
            Box::pin(handler(params))
        });

        let mut methods = self.methods.lock().await;
        methods.insert(method_name.clone(), HandlerType::NonStreaming(wrapped_handler));

        debug!("Registered method: {}", method_name);
        Ok(())
    }

    /// Register an async method handler with error conversion (legacy, without notification support)
    pub async fn register_async_method<F, Fut, E>(
        &self,
        method_name: String,
        handler: F,
    ) -> Result<()>
    where
        F: Fn(Option<serde_json::Value>) -> Fut + Send + Sync + 'static + Clone,
        Fut: std::future::Future<Output = Result<serde_json::Value, E>> + Send + 'static,
        E: Into<JsonRpcError> + Send + 'static,
    {
        let wrapped_handler: MethodHandler = Arc::new(move |params| {
            let handler_clone = handler.clone();
            Box::pin(async move {
                match handler_clone(params).await {
                    Ok(result) => Ok(result),
                    Err(e) => Err(e.into()),
                }
            })
        });

        let mut methods = self.methods.lock().await;
        methods.insert(method_name.clone(), HandlerType::NonStreaming(wrapped_handler));

        debug!("Registered async method: {}", method_name);
        Ok(())
    }

    /// Register a streaming method handler that can send notifications during execution
    pub async fn register_streaming_method<F, Fut, E>(
        &self,
        method_name: String,
        handler: F,
    ) -> Result<()>
    where
        F: Fn(Option<serde_json::Value>, NotificationSender) -> Fut + Send + Sync + 'static + Clone,
        Fut: std::future::Future<Output = Result<serde_json::Value, E>> + Send + 'static,
        E: Into<JsonRpcError> + Send + 'static,
    {
        let wrapped_handler: StreamingMethodHandler = Arc::new(move |params, notifier| {
            let handler_clone = handler.clone();
            Box::pin(async move {
                match handler_clone(params, notifier).await {
                    Ok(result) => Ok(result),
                    Err(e) => Err(e.into()),
                }
            })
        });

        let mut methods = self.methods.lock().await;
        methods.insert(method_name.clone(), HandlerType::Streaming(wrapped_handler));

        debug!("Registered streaming method: {}", method_name);
        Ok(())
    }

    /// Check if the server is running
    pub async fn is_running(&self) -> bool {
        let running = self.running.lock().await;
        *running
    }

    /// Start the server and process requests
    #[instrument(skip(self))]
    pub async fn start(&mut self) -> Result<()> {
        {
            let mut running = self.running.lock().await;
            if *running {
                return Err(anyhow!("Server is already running"));
            }
            *running = true;
        }

        info!("Starting JSON-RPC server with {} transport", self.transport.description());

        while self.is_running().await {
            // Flush any pending notifications before processing next request
            self.flush_notifications().await;
            
            match self.handle_single_request().await {
                Ok(()) => {
                    // Flush notifications that may have been queued during request handling
                    self.flush_notifications().await;
                }
                Err(e) => {
                    error!("Error handling request: {}", e);
                    // Continue processing other requests even if one fails
                }
            }
        }

        info!("JSON-RPC server stopped");
        Ok(())
    }

    /// Flush all pending notifications to the transport
    async fn flush_notifications(&mut self) {
        let mut rx = self.notification_rx.lock().await;
        
        // Drain all pending notifications
        while let Ok(notification) = rx.try_recv() {
            trace!("Sending notification: {}", notification.method);
            if let Err(e) = self.transport.write_notification(notification).await {
                error!("Failed to send notification: {}", e);
            }
        }
    }

    /// Send a notification immediately (for use outside request handlers)
    pub async fn send_notification(&mut self, notification: JsonRpcNotification) -> Result<()> {
        self.transport.write_notification(notification).await
    }

    /// Stop the server
    pub async fn stop(&mut self) -> Result<()> {
        {
            let mut running = self.running.lock().await;
            *running = false;
        }

        self.transport.close().await?;
        info!("JSON-RPC server stopped");
        Ok(())
    }

    /// Handle a single JSON-RPC request
    #[instrument(skip(self))]
    async fn handle_single_request(&mut self) -> Result<()> {
        // Read request from transport
        let request = match self.transport.read_request().await {
            Ok(req) => req,
            Err(e) => {
                error!("Failed to read request: {}", e);
                // Send parse error response
                let response = JsonRpcResponse::parse_error();
                if let Err(write_err) = self.transport.write_response(response).await {
                    error!("Failed to send error response: {}", write_err);
                }
                return Ok(());
            }
        };

        debug!("Received request: method={}, id={:?}", request.method, request.id);

        // Process the request
        let response = self.process_request(request).await;

        // Send response (if not a notification)
        if let Some(response) = response {
            if let Err(e) = self.transport.write_response(response).await {
                error!("Failed to send response: {}", e);
            }
        }

        Ok(())
    }

    /// Process a JSON-RPC request and return a response (if needed)
    #[instrument(skip(self))]
    async fn process_request(&self, request: JsonRpcRequest) -> Option<JsonRpcResponse> {
        // Notifications don't get responses
        let request_id = request.id.clone();
        let is_notification = request.is_notification();

        // Validate request
        if let Err(error) = request.validate() {
            if !is_notification {
                return Some(JsonRpcResponse::error(error, request_id));
            } else {
                // For notifications, we still log the error but don't respond
                warn!("Invalid notification: {}", error.message);
                return None;
            }
        }

        // Look up method handler
        let methods = self.methods.lock().await;
        let handler_type = match methods.get(&request.method) {
            Some(handler) => handler.clone(),
            None => {
                drop(methods); // Release lock early
                if !is_notification {
                    return Some(JsonRpcResponse::method_not_found(&request.method, request_id));
                } else {
                    warn!("Method not found for notification: {}", request.method);
                    return None;
                }
            }
        };
        drop(methods); // Release lock

        // Execute the method handler based on type
        let result = match handler_type {
            HandlerType::NonStreaming(handler) => handler(request.params).await,
            HandlerType::Streaming(handler) => {
                // Pass a clone of the notification sender to the streaming handler
                let notifier = self.notification_tx.clone();
                handler(request.params, notifier).await
            }
        };

        match result {
            Ok(result) => {
                if !is_notification {
                    Some(JsonRpcResponse::success(result, request_id))
                } else {
                    None
                }
            }
            Err(error) => {
                if !is_notification {
                    Some(JsonRpcResponse::error(error, request_id))
                } else {
                    error!("Error in notification handler for {}: {}", request.method, error.message);
                    None
                }
            }
        }
    }

    /// Get the list of registered methods
    pub async fn get_registered_methods(&self) -> Vec<String> {
        let methods = self.methods.lock().await;
        methods.keys().cloned().collect()
    }

    /// Get transport description
    pub fn transport_description(&self) -> &'static str {
        self.transport.description()
    }
}

/// Builder for JSON-RPC server
pub struct ServerBuilder {
    transport_config: Option<TransportConfig>,
    methods: HashMap<String, HandlerType>,
}

impl ServerBuilder {
    /// Create a new server builder
    pub fn new() -> Self {
        Self {
            transport_config: None,
            methods: HashMap::new(),
        }
    }

    /// Set the transport configuration
    pub fn with_transport(mut self, transport_config: TransportConfig) -> Self {
        self.transport_config = Some(transport_config);
        self
    }

    /// Set stdio transport
    pub fn with_stdio(self) -> Self {
        self.with_transport(TransportConfig::Stdio)
    }

    /// Set Unix socket transport
    pub fn with_unix_socket(self, path: String) -> Self {
        self.with_transport(TransportConfig::UnixSocket { path })
    }

    /// Register a method during building
    pub fn register_method<F, Fut>(
        mut self,
        method_name: String,
        handler: F,
    ) -> Self
    where
        F: Fn(Option<serde_json::Value>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<serde_json::Value, JsonRpcError>> + Send + 'static,
    {
        let wrapped_handler: MethodHandler = Arc::new(move |params| {
            Box::pin(handler(params))
        });

        self.methods.insert(method_name, HandlerType::NonStreaming(wrapped_handler));
        self
    }

    /// Register a streaming method during building
    pub fn register_streaming_method<F, Fut, E>(
        mut self,
        method_name: String,
        handler: F,
    ) -> Self
    where
        F: Fn(Option<serde_json::Value>, NotificationSender) -> Fut + Send + Sync + 'static + Clone,
        Fut: std::future::Future<Output = Result<serde_json::Value, E>> + Send + 'static,
        E: Into<JsonRpcError> + Send + 'static,
    {
        let wrapped_handler: StreamingMethodHandler = Arc::new(move |params, notifier| {
            let handler_clone = handler.clone();
            Box::pin(async move {
                match handler_clone(params, notifier).await {
                    Ok(result) => Ok(result),
                    Err(e) => Err(e.into()),
                }
            })
        });

        self.methods.insert(method_name, HandlerType::Streaming(wrapped_handler));
        self
    }

    /// Build the server
    pub async fn build(self) -> Result<JsonRpcServer> {
        let transport_config = self.transport_config
            .ok_or_else(|| anyhow!("Transport configuration not specified"))?;

        let server = JsonRpcServer::new(transport_config).await?;

        // Register pre-configured methods
        for (method_name, handler) in self.methods {
            let mut methods = server.methods.lock().await;
            methods.insert(method_name, handler);
        }

        Ok(server)
    }
}

impl Default for ServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for types that can be converted to JSON-RPC method handlers
#[async_trait]
pub trait IntoMethodHandler {
    /// Convert into a method handler
    async fn into_handler(self) -> MethodHandler;
}

/// Helper function to create a simple method handler
pub fn create_method_handler<F, Fut>(handler: F) -> MethodHandler
where
    F: Fn(Option<serde_json::Value>) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<serde_json::Value, JsonRpcError>> + Send + 'static,
{
    Arc::new(move |params| Box::pin(handler(params)))
}

/// Helper function to create a method handler with automatic error conversion
pub fn create_async_method_handler<F, Fut, E>(handler: F) -> MethodHandler
where
    F: Fn(Option<serde_json::Value>) -> Fut + Send + Sync + 'static + Clone,
    Fut: std::future::Future<Output = Result<serde_json::Value, E>> + Send + 'static,
    E: Into<JsonRpcError> + Send + 'static,
{
    Arc::new(move |params| {
        let handler_clone = handler.clone();
        Box::pin(async move {
            match handler_clone(params).await {
                Ok(result) => Ok(result),
                Err(e) => Err(e.into()),
            }
        })
    })
}

/// Helper function to create a streaming method handler
pub fn create_streaming_method_handler<F, Fut, E>(handler: F) -> StreamingMethodHandler
where
    F: Fn(Option<serde_json::Value>, NotificationSender) -> Fut + Send + Sync + 'static + Clone,
    Fut: std::future::Future<Output = Result<serde_json::Value, E>> + Send + 'static,
    E: Into<JsonRpcError> + Send + 'static,
{
    Arc::new(move |params, notifier| {
        let handler_clone = handler.clone();
        Box::pin(async move {
            match handler_clone(params, notifier).await {
                Ok(result) => Ok(result),
                Err(e) => Err(e.into()),
            }
        })
    })
}

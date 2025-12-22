use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncWrite};
use tower_lsp::jsonrpc;
use tracing::{debug, error, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransportConfig {
    Stdio,
    UnixSocket { path: String },
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self::Stdio
    }
}

pub struct Transport {
    config: TransportConfig,
}

impl Transport {
    pub fn new(config: TransportConfig) -> Self {
        Self { config }
    }

    /// Initialize the transport layer based on configuration
    pub async fn start<T>(&self, server: T) -> Result<()>
    where
        T: tower_lsp::LanguageServer + Send + Sync + 'static,
    {
        match &self.config {
            TransportConfig::Stdio => {
                info!("Starting server with stdio transport");
                self.start_stdio(server).await
            }
            TransportConfig::UnixSocket { path } => {
                info!("Starting server with Unix socket: {}", path);
                self.start_unix_socket(server, path).await
            }
        }
    }

    async fn start_stdio<T>(&self, server: T) -> Result<()>
    where
        T: tower_lsp::LanguageServer + Send + Sync + 'static,
    {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        debug!("Initializing LSP server with stdio transport");
        tower_lsp::Server::new(stdin, stdout, server)
            .serve()
            .await;

        Ok(())
    }

    #[cfg(unix)]
    async fn start_unix_socket<T>(&self, server: T, path: &str) -> Result<()>
    where
        T: tower_lsp::LanguageServer + Send + Sync + 'static,
    {
        use tokio::net::{UnixListener, UnixStream};

        // Remove existing socket file if it exists
        let _ = std::fs::remove_file(path);

        let listener = UnixListener::bind(path)?;
        info!("Unix socket server listening on: {}", path);

        while let Ok((stream, _)) = listener.accept().await {
            debug!("New client connected");
            let (read, write) = stream.into_split();

            // Clone server for this connection
            let server_clone = server.clone();

            tokio::spawn(async move {
                if let Err(e) = tower_lsp::Server::new(read, write, server_clone)
                    .serve()
                    .await {
                    error!("Client connection error: {}", e);
                }
            });
        }

        Ok(())
    }

    #[cfg(not(unix))]
    async fn start_unix_socket<T>(&self, _server: T, _path: &str) -> Result<()>
    where
        T: tower_lsp::LanguageServer + Send + Sync + 'static,
    {
        anyhow::bail!("Unix sockets are not supported on this platform");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_config_default() {
        let config = TransportConfig::default();
        matches!(config, TransportConfig::Stdio);
    }

    #[test]
    fn test_transport_creation() {
        let transport = Transport::new(TransportConfig::Stdio);
        matches!(transport.config, TransportConfig::Stdio);
    }
}
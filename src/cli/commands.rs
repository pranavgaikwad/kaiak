//! CLI command implementations.

use anyhow::Result;
use std::path::PathBuf;
use tracing::{error, info};

use crate::client::ConnectionState;
use crate::models::configuration::ServerConfig;
use crate::server::{start_server, TransportConfig};

use super::utils::{load_request_params, load_server_config, print_notification};

/// Start the Kaiak JSON-RPC server
pub async fn serve(
    transport_type: String,
    socket_path: Option<String>,
    config_path: Option<PathBuf>,
    config_json: Option<String>,
) -> Result<()> {
    info!("Starting Kaiak server with {} transport", transport_type);
    info!("Loading configuration...");

    let server_config = load_server_config(config_path, config_json)?;
    server_config.validate()?;

    let transport_config = match transport_type.as_str() {
        "stdio" => TransportConfig::Stdio,
        "socket" => TransportConfig::UnixSocket {
            path: socket_path
                .or(server_config.init_config.socket_path.clone())
                .unwrap_or_else(|| "/tmp/kaiak.sock".to_string()),
        },
        _ => anyhow::bail!("Invalid transport type: {}", transport_type),
    };

    info!(
        "Initializing Kaiak JSON-RPC server with transport: {:?}",
        transport_config
    );

    info!("Server starting...");
    start_server(std::sync::Arc::new(server_config), Some(transport_config)).await?;

    info!("Kaiak server stopped");
    Ok(())
}

/// Connect to a Kaiak server via Unix socket
pub async fn connect(socket_path: String) -> Result<()> {
    use crate::client::JsonRpcClient;

    info!("Connecting to Kaiak server at: {}", socket_path);

    let path = std::path::Path::new(&socket_path);
    if !path.exists() {
        anyhow::bail!("Socket path does not exist: {}", socket_path);
    }

    let client = JsonRpcClient::new(socket_path.clone());
    if !client.validate_connection().await? {
        anyhow::bail!("Failed to connect to server at: {}", socket_path);
    }

    ConnectionState::save(&socket_path)?;

    println!("✓ Connected to Kaiak server at: {}", socket_path);
    println!("  Connection saved. Use 'kaiak status' to check connection.");
    println!("  Use 'kaiak disconnect' to disconnect.");

    Ok(())
}

/// Disconnect from the current Kaiak server
pub async fn disconnect() -> Result<()> {
    if !ConnectionState::is_connected()? {
        println!("Not currently connected to any server.");
        return Ok(());
    }

    let socket_path = ConnectionState::load()?.unwrap_or_default();
    ConnectionState::clear()?;

    println!("✓ Disconnected from: {}", socket_path);

    Ok(())
}

/// Generate fix for migration incidents
pub async fn generate_fix(
    params_file: Option<PathBuf>,
    params_json: Option<String>,
) -> Result<()> {
    let params = load_request_params(params_file, params_json, "generate_fix")?;
    let client = ConnectionState::get_client()?;

    info!("Sending generate_fix request to: {}", client.socket_path());

    let result = client
        .generate_fix(params, |notification| {
            print_notification(&notification);
        })
        .await?;

    let output = serde_json::to_string_pretty(&result)?;
    println!("{}", output);

    Ok(())
}

/// Delete an agent session
pub async fn delete_session(session_id: String) -> Result<()> {
    uuid::Uuid::parse_str(&session_id)
        .map_err(|_| anyhow::anyhow!("Invalid session ID: must be a valid UUID"))?;

    let params = serde_json::json!({
        "inner": {
            "session_id": session_id
        }
    });

    let client = ConnectionState::get_client()?;

    info!(
        "Deleting session {} via: {}",
        session_id,
        client.socket_path()
    );

    let result = client.delete_session(params, |_| {}).await?;

    let output = serde_json::to_string_pretty(&result)?;
    println!("{}", output);

    Ok(())
}

/// Initialize default configuration
pub async fn init(force: bool) -> Result<()> {
    let config_path = ServerConfig::config_path()?;

    if config_path.exists() && !force {
        anyhow::bail!(
            "Configuration file already exists at {:?}. Use --force to overwrite.",
            config_path
        );
    }

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let config = ServerConfig::default();
    let toml_content = toml::to_string_pretty(&config)?;
    std::fs::write(&config_path, toml_content)?;

    println!("✓ Configuration initialized at {:?}", config_path);
    Ok(())
}

/// Manage configuration (show, validate, edit)
pub async fn config(show: bool, validate: bool, edit: bool) -> Result<()> {
    if show {
        let config = ServerConfig::load()?;
        let toml_content = toml::to_string_pretty(&config)?;
        println!("{}", toml_content);
    }

    if validate {
        match ServerConfig::load() {
            Ok(config) => match config.validate() {
                Ok(()) => println!("✓ Configuration is valid"),
                Err(e) => error!("Configuration validation failed: {}", e),
            },
            Err(e) => error!("Failed to load configuration: {}", e),
        }
    }

    if edit {
        let config_path = ServerConfig::config_path()?;
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
        std::process::Command::new(editor)
            .arg(&config_path)
            .status()?;
    }

    Ok(())
}

/// Show version information
pub async fn version() -> Result<()> {
    println!("Kaiak {}", env!("CARGO_PKG_VERSION"));
    println!("Built with Rust {}", rustc_version::version()?);
    Ok(())
}

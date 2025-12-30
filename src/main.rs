use anyhow::Result;
use clap::{Parser, Subcommand};
use kaiak::client::{ConnectionState, JsonRpcClient, JsonRpcNotification};
use kaiak::logging::init_logging;
use kaiak::models::configuration::{ServerConfig, ConfigurationHierarchy};
use kaiak::server::{start_server, TransportConfig};
use std::path::PathBuf;
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "kaiak")]
#[command(about = "Standalone server integrating Goose AI agent for code migration workflows")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, global = true)]
    log_level: Option<String>,

    /// Configuration file path
    #[arg(long, global = true)]
    config: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the JSON-RPC server
    Serve {
        /// Transport method: "stdio" or "socket"
        #[arg(long, short = 't', default_value = "stdio")]
        transport: String,

        /// Unix socket path (used when transport = "socket")
        #[arg(long, short = 's')]
        socket_path: Option<String>,

        /// Path to a custom configuration file (TOML format)
        #[arg(long, short = 'c', conflicts_with = "config_json")]
        config_path: Option<PathBuf>,

        /// Inline JSON configuration (overrides file-based config)
        #[arg(long, short = 'j', conflicts_with = "config_path")]
        config_json: Option<String>,
    },

    /// Connect to a Kaiak server via Unix socket
    /// stdio is not supported in connect command
    Connect {
        socket_path: String,
    }, 

    /// Disconnect from the current Kaiak server
    Disconnect,

    /// Generate fix for migration incidents (requires active connection)
    GenerateFix {
        /// Path to JSON file containing request parameters
        #[arg(long, short = 'p', conflicts_with = "params_json")]
        params_file: Option<PathBuf>,

        /// Inline JSON request parameters
        #[arg(long, short = 'j', conflicts_with = "params_file")]
        params_json: Option<String>,
    },

    /// Delete a session (requires active connection)
    DeleteSession {
        session_id: String,
    },

    /// Initialize default configuration at default location
    Init {
        #[arg(long)]
        force: bool,
    },

    /// Manage configuration
    Config {
        #[arg(long)]
        show: bool,

        #[arg(long)]
        validate: bool,

        #[arg(long)]
        edit: bool,
    },

    /// Show version information
    Version,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging early
    init_logging()?;

    match cli.command {
        // Server commands
        Commands::Serve {
            transport,
            socket_path,
            config_path,
            config_json,
        } => {
            info!("Starting Kaiak server with {} transport", transport);
            serve_command(transport, socket_path, config_path, config_json).await
        }

        // Client commands
        Commands::Connect { socket_path } => connect_command(socket_path).await,
        Commands::Disconnect => disconnect_command().await,
        Commands::GenerateFix { params_file, params_json } => {
            generate_fix_command(params_file, params_json).await
        }
        Commands::DeleteSession { session_id } => {
            delete_session_command(session_id).await
        }

        // Configuration commands
        Commands::Init { force } => init_command(force).await,
        Commands::Config { show, validate, edit } => {
            config_command(show, validate, edit).await
        }
        Commands::Version => version_command().await,
    }
}

async fn serve_command(
    transport_type: String,
    socket_path: Option<String>,
    config_path: Option<PathBuf>,
    config_json: Option<String>,
) -> Result<()> {
    info!("Loading configuration...");

    // Build configuration using hierarchy:
    // Priority: CLI JSON > CLI file path > user config (~/.kaiak/server.conf) > defaults
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

    info!("Initializing Kaiak JSON-RPC server with transport: {:?}", transport_config);

    // Start the JSON-RPC server
    info!("Server starting...");
    start_server(std::sync::Arc::new(server_config), Some(transport_config)).await?;

    info!("Kaiak server stopped");
    Ok(())
}


async fn connect_command(socket_path: String) -> Result<()> {
    info!("Connecting to Kaiak server at: {}", socket_path);

    // Validate the socket path exists
    let path = std::path::Path::new(&socket_path);
    if !path.exists() {
        anyhow::bail!("Socket path does not exist: {}", socket_path);
    }

    // Create a client and validate connection
    let client = JsonRpcClient::new(socket_path.clone());
    if !client.validate_connection().await? {
        anyhow::bail!("Failed to connect to server at: {}", socket_path);
    }

    // Save the connection
    ConnectionState::save(&socket_path)?;

    println!("✓ Connected to Kaiak server at: {}", socket_path);
    println!("  Connection saved. Use 'kaiak status' to check connection.");
    println!("  Use 'kaiak disconnect' to disconnect.");

    Ok(())
}

async fn disconnect_command() -> Result<()> {
    if !ConnectionState::is_connected()? {
        println!("Not currently connected to any server.");
        return Ok(());
    }

    let socket_path = ConnectionState::load()?.unwrap_or_default();
    ConnectionState::clear()?;

    println!("✓ Disconnected from: {}", socket_path);

    Ok(())
}

async fn generate_fix_command(
    params_file: Option<PathBuf>,
    params_json: Option<String>,
) -> Result<()> {
    // Get params from file or inline JSON
    let params = load_request_params(params_file, params_json, "generate_fix")?;

    // Get client from connection state
    let client = ConnectionState::get_client()?;

    info!("Sending generate_fix request to: {}", client.socket_path());

    // Execute the request with notification streaming
    let result = client.generate_fix(params, |notification| {
        print_notification(&notification);
    }).await?;

    // Pretty print the final result
    let output = serde_json::to_string_pretty(&result)?;
    println!("{}", output);

    Ok(())
}

fn print_notification(notification: &JsonRpcNotification) {
    // Format based on the notification method
    match notification.method.as_str() {
        "kaiak/generateFix/progress" => {
            if let Some(params) = &notification.params {
                let stage = params.get("stage")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let progress = params.get("progress")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                
                println!("[{:>3}%] {}", progress, stage);
                
                // Print additional data if present
                if let Some(data) = params.get("data") {
                    if !data.is_null() {
                        if let Ok(formatted) = serde_json::to_string_pretty(data) {
                            for line in formatted.lines() {
                                println!("       {}", line);
                            }
                        }
                    }
                }
            }
        }
        "$/progress" => {
            // LSP-style progress notification
            if let Some(params) = &notification.params {
                if let Some(value) = params.get("value") {
                    let message = value.get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let percentage = value.get("percentage")
                        .and_then(|v| v.as_u64());
                    
                    if let Some(pct) = percentage {
                        println!("[{:>3}%] {}", pct, message);
                    } else {
                        println!("[...] {}", message);
                    }
                }
            }
        }
        _ => {
            // Generic notification - print method and params
            println!("[{}]", notification.method);
            if let Some(params) = &notification.params {
                if let Ok(formatted) = serde_json::to_string_pretty(params) {
                    for line in formatted.lines() {
                        println!("  {}", line);
                    }
                }
            }
        }
    }
}

async fn delete_session_command(session_id: String) -> Result<()> {
    // Validate session_id is a valid UUID
    uuid::Uuid::parse_str(&session_id)
        .map_err(|_| anyhow::anyhow!("Invalid session ID: must be a valid UUID"))?;

    // Build the request params
    let params = serde_json::json!({
        "inner": {
            "session_id": session_id
        }
    });

    // Get client from connection state
    let client = ConnectionState::get_client()?;

    info!("Deleting session {} via: {}", session_id, client.socket_path());

    // Execute the request (no notifications expected for delete_session)
    let result = client.delete_session(params, |_| {}).await?;

    // Pretty print the result
    let output = serde_json::to_string_pretty(&result)?;
    println!("{}", output);

    Ok(())
}

/// Load request parameters from file or inline JSON
fn load_request_params(
    params_file: Option<PathBuf>,
    params_json: Option<String>,
    command_name: &str,
) -> Result<serde_json::Value> {
    match (params_file, params_json) {
        (Some(path), None) => {
            // Load from file
            if !path.exists() {
                anyhow::bail!("Parameters file not found: {}", path.display());
            }
            let content = std::fs::read_to_string(&path)?;
            let params: serde_json::Value = serde_json::from_str(&content)
                .map_err(|e| anyhow::anyhow!("Failed to parse JSON from {}: {}", path.display(), e))?;
            Ok(params)
        }
        (None, Some(json_str)) => {
            // Parse inline JSON
            let params: serde_json::Value = serde_json::from_str(&json_str)
                .map_err(|e| anyhow::anyhow!("Failed to parse inline JSON: {}", e))?;
            Ok(params)
        }
        (None, None) => {
            anyhow::bail!(
                "No parameters provided for {}. Use --params-file or --params-json.",
                command_name
            );
        }
        (Some(_), Some(_)) => {
            // This shouldn't happen due to clap's conflicts_with, but handle it anyway
            anyhow::bail!("Cannot use both --params-file and --params-json");
        }
    }
}


fn load_server_config(
    config_path: Option<PathBuf>,
    config_json: Option<String>,
) -> Result<ServerConfig> {
    // If inline JSON is provided, parse and use it as CLI override
    let cli_override = if let Some(json_str) = config_json {
        info!("Using inline JSON configuration");
        let config: ServerConfig = serde_json::from_str(&json_str)
            .map_err(|e| anyhow::anyhow!("Failed to parse config JSON: {}", e))?;
        Some(config)
    } else {
        None
    };

    // Determine which config file path to use
    let user_config_path = if let Some(path) = config_path {
        info!("Using custom config file: {}", path.display());
        if !path.exists() {
            anyhow::bail!("Config file not found: {}", path.display());
        }
        Some(path)
    } else {
        // Use default user config path
        match ConfigurationHierarchy::default_user_config_path() {
            Ok(path) if path.exists() => {
                info!("Using default config file: {}", path.display());
                Some(path)
            }
            Ok(path) => {
                info!("No config file found at {}, using defaults", path.display());
                None
            }
            Err(e) => {
                info!("Could not determine default config path: {}, using defaults", e);
                None
            }
        }
    };

    // Load configuration with hierarchy
    let mut hierarchy = ConfigurationHierarchy::load_with_precedence(
        cli_override.as_ref(),
        user_config_path,
        None, // No system-wide default config path for now
    )?;

    // Apply environment variable overrides
    hierarchy.apply_env_overrides()?;

    // Log configuration sources for debugging
    for source in &hierarchy.sources {
        info!("Config source [priority {}]: {}", source.priority, source.name);
    }

    hierarchy.validate()?;
    Ok(hierarchy.resolved)
}

async fn init_command(force: bool) -> Result<()> {
    let config_path = ServerConfig::config_path()?;

    if config_path.exists() && !force {
        anyhow::bail!(
            "Configuration file already exists at {:?}. Use --force to overwrite.",
            config_path
        );
    }

    // Create config directory if it doesn't exist
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let config = ServerConfig::default();
    let toml_content = toml::to_string_pretty(&config)?;
    std::fs::write(&config_path, toml_content)?;

    info!("Configuration initialized at {:?}", config_path);
    Ok(())
}

async fn config_command(show: bool, validate: bool, edit: bool) -> Result<()> {
    if show {
        let config = ServerConfig::load()?;
        let toml_content = toml::to_string_pretty(&config)?;
        println!("{}", toml_content);
    }

    if validate {
        match ServerConfig::load() {
            Ok(config) => match config.validate() {
                Ok(()) => info!("Configuration is valid"),
                Err(e) => error!("Configuration validation failed: {}", e),
            },
            Err(e) => error!("Failed to load configuration: {}", e),
        }
    }

    if edit {
        let config_path = ServerConfig::config_path()?;
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
        std::process::Command::new(editor).arg(&config_path).status()?;
    }

    Ok(())
}

async fn version_command() -> Result<()> {
    println!("Kaiak {}", env!("CARGO_PKG_VERSION"));
    println!("Built with Rust {}", rustc_version::version()?);
    Ok(())
}

// Note: rustc_version and toml dependencies are already added to Cargo.toml
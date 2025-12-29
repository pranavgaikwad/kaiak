use anyhow::Result;
use clap::{Parser, Subcommand};
use kaiak::logging::init_logging;
use kaiak::models::configuration::{ServerConfig, ConfigurationHierarchy};
use kaiak::server::{start_server, TransportConfig};
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "kaiak")]
#[command(about = "Standalone server integrating Goose AI agent for code migration workflows")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Set log level (trace, debug, info, warn, error)
    #[arg(long, global = true)]
    log_level: Option<String>,

    /// Configuration file path
    #[arg(long, global = true)]
    config: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the Kaiak server
    Serve {
        /// Transport method (stdio, socket)
        #[arg(long, default_value = "stdio")]
        transport: String,

        /// Unix socket path (when using socket transport)
        #[arg(long)]
        socket_path: Option<String>,

        /// Workspace root path
        #[arg(long)]
        workspace: Option<String>,
    },

    /// Initialize default configuration
    Init {
        /// Force overwrite existing configuration
        #[arg(long)]
        force: bool,
    },

    /// Validate configuration
    Config {
        /// Show current configuration
        #[arg(long)]
        show: bool,

        /// Validate configuration file
        #[arg(long)]
        validate: bool,

        /// Edit configuration file
        #[arg(long)]
        edit: bool,
    },

    /// Run health checks
    Doctor,

    /// Show version information
    Version,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging early
    init_logging()?;

    match cli.command {
        Commands::Serve {
            transport,
            socket_path,
            workspace: _workspace,
        } => {
            info!("Starting Kaiak server with {} transport", transport);
            serve_command(transport, socket_path).await
        }
        Commands::Init { force } => init_command(force).await,
        Commands::Config { show, validate, edit } => {
            config_command(show, validate, edit).await
        }
        Commands::Doctor => doctor_command().await,
        Commands::Version => version_command().await,
    }
}

async fn serve_command(transport_type: String, socket_path: Option<String>) -> Result<()> {
    info!("Loading configuration...");
    let settings = ServerSettings::load()?;
    settings.validate()?;

    let transport_config = match transport_type.as_str() {
        "stdio" => TransportConfig::Stdio,
        "socket" => TransportConfig::UnixSocket {
            path: socket_path
                .or(settings.server.socket_path)
                .unwrap_or_else(|| "/tmp/kaiak.sock".to_string()),
        },
        _ => anyhow::bail!("Invalid transport type: {}", transport_type),
    };

    info!("Initializing Kaiak LSP server with transport: {:?}", transport_config);

    // Start the integrated LSP server
    info!("Server starting...");
    start_server(transport_config).await?;

    info!("Kaiak server stopped");
    Ok(())
}

async fn init_command(force: bool) -> Result<()> {
    let config_path = ServerSettings::config_path();

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

    let settings = ServerSettings::default();
    let toml_content = toml::to_string_pretty(&settings)?;
    std::fs::write(&config_path, toml_content)?;

    info!("Configuration initialized at {:?}", config_path);
    Ok(())
}

async fn config_command(show: bool, validate: bool, edit: bool) -> Result<()> {
    if show {
        let settings = ServerSettings::load()?;
        let toml_content = toml::to_string_pretty(&settings)?;
        println!("{}", toml_content);
    }

    if validate {
        match ServerSettings::load() {
            Ok(settings) => match settings.validate() {
                Ok(()) => info!("Configuration is valid"),
                Err(e) => error!("Configuration validation failed: {}", e),
            },
            Err(e) => error!("Failed to load configuration: {}", e),
        }
    }

    if edit {
        let config_path = ServerSettings::config_path();
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
        std::process::Command::new(editor).arg(&config_path).status()?;
    }

    Ok(())
}

async fn doctor_command() -> Result<()> {
    info!("Running Kaiak health checks...");

    // Check configuration
    print!("Configuration: ");
    match ServerSettings::load() {
        Ok(settings) => match settings.validate() {
            Ok(()) => println!("✓ Valid"),
            Err(e) => println!("✗ Invalid - {}", e),
        },
        Err(e) => println!("✗ Failed to load - {}", e),
    }

    // Check dependencies
    print!("Dependencies: ");
    // TODO: Check for Goose availability and other dependencies
    println!("✓ Available (placeholder)");

    // Check network connectivity (for AI providers)
    print!("AI Provider connectivity: ");
    // TODO: Test connectivity to configured AI providers
    println!("? Skipped (placeholder)");

    // Check workspace permissions
    print!("File system permissions: ");
    // TODO: Check read/write permissions for common workspace locations
    println!("? Skipped (placeholder)");

    info!("Health check completed");
    Ok(())
}

async fn version_command() -> Result<()> {
    println!("Kaiak {}", env!("CARGO_PKG_VERSION"));
    println!("Built with Rust {}", rustc_version::version()?);
    Ok(())
}

// Note: rustc_version and toml dependencies are already added to Cargo.toml
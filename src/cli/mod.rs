//! CLI interface for Kaiak.
//!
//! This module provides the command-line interface for both server and client operations.

mod commands;
mod utils;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

pub use commands::*;
pub use utils::*;

#[derive(Parser)]
#[command(name = "kaiak")]
#[command(about = "Standalone server integrating Goose AI agent for code migration workflows")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Set log level (trace, debug, info, warn, error)
    #[arg(long, global = true)]
    pub log_level: Option<String>,

    /// Configuration file path
    #[arg(long, global = true)]
    pub config: Option<String>,
}

/// Available CLI commands
#[derive(Subcommand)]
pub enum Commands {
    Serve {
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
    Connect {
        socket_path: String,
    },

    /// Disconnect from the current Kaiak server
    Disconnect,

    /// Generate fix for migration incidents (requires active connection)
    GenerateFix {
        #[arg(long, short = 'p', conflicts_with = "params_json")]
        params_file: Option<PathBuf>,

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

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }

    /// Run the CLI command
    pub async fn run(self) -> Result<()> {
        match self.command {
            Commands::Serve {
                transport,
                socket_path,
                config_path,
                config_json,
            } => serve(transport, socket_path, config_path, config_json).await,

            Commands::Connect { socket_path } => connect(socket_path).await,
            Commands::Disconnect => disconnect().await,

            Commands::GenerateFix {
                params_file,
                params_json,
            } => generate_fix(params_file, params_json).await,

            Commands::DeleteSession { session_id } => delete_session(session_id).await,

            Commands::Init { force } => init(force).await,
            Commands::Config {
                show,
                validate,
                edit,
            } => config(show, validate, edit).await,

            Commands::Version => version().await,
        }
    }
}

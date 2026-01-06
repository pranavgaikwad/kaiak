//! CLI utility functions.

use anyhow::Result;
use std::path::PathBuf;
use tracing::info;

use crate::client::JsonRpcNotification;
use crate::models::configuration::{ConfigurationHierarchy, ServerConfig};

/// Load request parameters from file or inline JSON
pub fn load_request_params(
    params_file: Option<PathBuf>,
    params_json: Option<String>,
    command_name: &str,
) -> Result<serde_json::Value> {
    match (params_file, params_json) {
        (Some(path), None) => {
            if !path.exists() {
                anyhow::bail!("Parameters file not found: {}", path.display());
            }
            let content = std::fs::read_to_string(&path)?;
            let params: serde_json::Value = serde_json::from_str(&content)
                .map_err(|e| anyhow::anyhow!("Failed to parse JSON from {}: {}", path.display(), e))?;
            Ok(params)
        }
        (None, Some(json_str)) => {
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
            anyhow::bail!("Cannot use both --params-file and --params-json");
        }
    }
}

/// Load server configuration with hierarchy (CLI > file > defaults)
pub fn load_server_config(
    config_path: Option<PathBuf>,
    config_json: Option<String>,
) -> Result<ServerConfig> {
    let cli_override = if let Some(json_str) = config_json {
        info!("Using inline JSON configuration");
        let config: ServerConfig = serde_json::from_str(&json_str)
            .map_err(|e| anyhow::anyhow!("Failed to parse config JSON: {}", e))?;
        Some(config)
    } else {
        None
    };

    let user_config_path = if let Some(path) = config_path {
        info!("Using custom config file: {}", path.display());
        if !path.exists() {
            anyhow::bail!("Config file not found: {}", path.display());
        }
        Some(path)
    } else {
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
                info!(
                    "Could not determine default config path: {}, using defaults",
                    e
                );
                None
            }
        }
    };

    let mut hierarchy = ConfigurationHierarchy::load_with_precedence(
        cli_override.as_ref(),
        user_config_path,
        None,
    )?;

    hierarchy.apply_env_overrides()?;

    for source in &hierarchy.sources {
        info!("Config source [priority {}]: {}", source.priority, source.name);
    }

    hierarchy.validate()?;
    Ok(hierarchy.resolved)
}

/// Print a JSON-RPC notification to stdout
pub fn print_notification(notification: &JsonRpcNotification) {
    match notification.method.as_str() {
        "kaiak/generate_fix/data" => {
            if let Some(params) = &notification.params {
                let kind = params
                    .get("kind")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                if let Some(payload) = params.get("payload") {
                    if let Ok(payload_str) = serde_json::to_string(payload) {
                        println!("[{}] {}", kind, payload_str);
                    }
                }
            }
        }
        _ => {
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

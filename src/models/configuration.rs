use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use validator::Validate;

// Import actual Goose types
pub use goose::agents::{ExtensionConfig, SessionConfig as GooseSessionConfig};
pub use goose::config::permission::PermissionLevel;
pub use goose::session::SessionType;

/// Unified server configuration for Kaiak server initialization and runtime settings
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ServerConfig {
    /// This is set at runtime and cannot be changed until server restart
    #[validate(nested)]
    pub init_config: InitConfig,

    /// This is a base set of configuration that server starts with via configuration files
    /// However, the BaseConfig can be overriden via generate_fix request for a specific session
    #[validate(nested)]
    pub base_config: BaseConfig,
}

/// Immutable server initialization configuration
/// These settings are set once at server startup and cannot be changed during runtime
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct InitConfig {
    /// Transport method: "stdio" or "socket"
    #[validate(custom(function = "validate_transport_type"))]
    pub transport: String,

    /// Unix socket path (required when transport = "socket")
    pub socket_path: Option<String>,

    /// Logging level: trace, debug, info, warn, error
    #[validate(custom(function = "validate_log_level"))]
    pub log_level: String,

    /// Maximum concurrent agent sessions
    #[validate(range(min = 1, max = 100))]
    pub max_concurrent_sessions: u32,
}

/// Runtime server base configuration that can be overridden per session
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct BaseConfig {
    pub model: ModelConfig,
    // We maintain a map of tool names to their permission levels
    // TODO (pgaikwad): Deep dive into smart permission settings
    pub tool_permissions: HashMap<String, PermissionLevel>,
}

/// Per-session agent configuration sent by clients for individual agent sessions in the generate_fix request
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AgentConfig {
    #[validate(custom(function = "validate_workspace_path"))]
    pub workspace: PathBuf,
    pub session: GooseSessionConfig,
    /// override_base_config completely overrides server's base_config
    #[validate(nested)]
    pub override_base_config: Option<BaseConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ModelConfig {
    pub provider: String,
    pub model: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            init_config: InitConfig::default(),
            base_config: BaseConfig::default(),
        }
    }
}

impl Default for InitConfig {
    fn default() -> Self {
        Self {
            transport: "stdio".to_string(),
            socket_path: None,
            log_level: "info".to_string(),
            max_concurrent_sessions: 10,
        }
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            workspace: PathBuf::from("."),
            session: GooseSessionConfig {
                id: uuid::Uuid::new_v4().to_string(),
                schedule_id: None,
                max_turns: Some(1000),
                retry_config: None,
            },
            override_base_config: Some(BaseConfig::default()),
        }
    }
}

impl Default for BaseConfig {
    fn default() -> Self {
        Self {
            model: ModelConfig::default(),
            // TODO (pgaikwad): revisit this
            tool_permissions: HashMap::new(),
        }
    }
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            model: "gpt-4o".to_string(),
            temperature: Some(0.01),
            max_tokens: None,
        }
    }
}

impl AgentConfig {
    /// Validate and optionally generate a new session ID if none provided
    pub fn ensure_valid_session_id(&mut self) -> Result<(), String> {
        // If session ID is empty, generate a new one
        if self.session.id.is_empty() {
            self.session.id = uuid::Uuid::new_v4().to_string();
            return Ok(());
        }

        // Validate the provided session ID is a valid UUID
        match uuid::Uuid::parse_str(&self.session.id) {
            Ok(_) => Ok(()),
            Err(_) => Err(format!(
                "Invalid UUID format for session ID: {}",
                self.session.id
            )),
        }
    }

    /// Create a new configuration with a specific session ID
    pub fn with_session_id(session_id: String) -> Result<Self, String> {
        // Validate session ID is a proper UUID
        uuid::Uuid::parse_str(&session_id)
            .map_err(|_| format!("Invalid UUID format for session ID: {}", session_id))?;

        let mut config = Self::default();
        config.session.id = session_id;
        Ok(config)
    }
}

/// Configuration hierarchy manager that merges multiple configuration sources
/// Handles precedence: CLI args > user config > default config > hardcoded defaults
#[derive(Debug, Clone)]
pub struct ConfigurationHierarchy {
    /// Final resolved configuration
    pub resolved: ServerConfig,

    /// Sources used in resolution (for debugging)
    pub sources: Vec<ConfigSource>,
}

/// Information about a configuration source for debugging and audit trails
#[derive(Debug, Clone)]
pub struct ConfigSource {
    pub name: String,                 // e.g., "CLI arguments", "~/.kaiak/server.conf"
    pub priority: u8,                 // Higher number = higher priority
    pub fields_provided: Vec<String>, // Which fields this source provided
}

impl ConfigurationHierarchy {
    /// Load configuration with precedence: CLI > user config > default config > hardcoded
    pub fn load_with_precedence(
        cli_overrides: Option<&ServerConfig>,
        user_config_path: Option<PathBuf>,
        default_config_path: Option<PathBuf>,
    ) -> Result<Self> {
        let mut sources = Vec::new();
        let mut resolved = ServerConfig::default();

        // Start with hardcoded defaults (lowest priority)
        sources.push(ConfigSource {
            name: "Hardcoded defaults".to_string(),
            priority: 1,
            fields_provided: vec!["all".to_string()],
        });

        // Load default config file if exists
        if let Some(default_path) = default_config_path {
            if default_path.exists() {
                match Self::load_config_file(&default_path) {
                    Ok(config) => {
                        resolved = Self::merge_configs(resolved, config); 
                        sources.push(ConfigSource {
                            name: format!("Default config: {}", default_path.display()),
                            priority: 2,
                            fields_provided: vec!["loaded from file".to_string()],
                        });
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to load default config {}: {}",
                            default_path.display(),
                            e
                        );
                    }
                }
            }
        }

        // Load user config file if exists
        if let Some(user_path) = user_config_path {
            if user_path.exists() {
                match Self::load_config_file(&user_path) {
                    Ok(config) => {
                        resolved = Self::merge_configs(resolved, config);
                        sources.push(ConfigSource {
                            name: format!("User config: {}", user_path.display()),
                            priority: 3,
                            fields_provided: vec!["loaded from file".to_string()],
                        });
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load user config {}: {}", user_path.display(), e);
                    }
                }
            }
        }

        // Apply CLI overrides (highest priority)
        if let Some(cli_config) = cli_overrides {
            resolved = Self::merge_configs(resolved, cli_config.clone());
            sources.push(ConfigSource {
                name: "CLI arguments".to_string(),
                priority: 4,
                fields_provided: vec!["CLI overrides".to_string()],
            });
        }

        Ok(ConfigurationHierarchy { resolved, sources })
    }

    /// Apply environment variable overrides
    pub fn apply_env_overrides(&mut self) -> Result<()> {
        let mut env_overrides = Vec::new();

        if let Ok(val) = std::env::var("KAIAK_LOG_LEVEL") {
            self.resolved.init_config.log_level = val.clone();
            env_overrides.push(format!("KAIAK_LOG_LEVEL={}", val));
        }
        if !env_overrides.is_empty() {
            self.sources.push(ConfigSource {
                name: "Environment variables".to_string(),
                priority: 5,
                fields_provided: env_overrides,
            });
        }

        Ok(())
    }

    /// Validate final configuration
    pub fn validate(&self) -> Result<()> {
        self.resolved
            .validate()
            .map_err(|e| anyhow::anyhow!("Configuration validation failed: {}", e))
    }

    /// Load configuration from a TOML file
    fn load_config_file(path: &PathBuf) -> Result<ServerConfig> {
        let content = std::fs::read_to_string(path)?;
        let config: ServerConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Merge two configurations (second takes precedence)
    /// Right now, we just replace with the override_config
    fn merge_configs(_base: ServerConfig, override_config: ServerConfig) -> ServerConfig {
        override_config
    }

    /// Get the default user config path: ~/.kaiak/server.conf
    pub fn default_user_config_path() -> Result<PathBuf> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Unable to determine home directory"))?;

        let kaiak_dir = home_dir.join(".kaiak");

        // Create .kaiak directory if it doesn't exist
        if !kaiak_dir.exists() {
            std::fs::create_dir_all(&kaiak_dir)?;
        }

        Ok(kaiak_dir.join("server.conf"))
    }
}

impl ServerConfig {
    /// Validate the complete configuration
    pub fn validate(&self) -> Result<()> {
        // Validate nested structures using validator trait
        Validate::validate(self).map_err(|e| anyhow::anyhow!("Validation failed: {:?}", e))?;

        // Additional business logic validation
        if self.init_config.max_concurrent_sessions == 0 {
            anyhow::bail!("Max concurrent sessions must be greater than 0");
        }

        // Validate socket path when using socket transport
        if self.init_config.transport == "socket" && self.init_config.socket_path.is_none() {
            anyhow::bail!("Socket path is required when transport is 'socket'");
        }

        Ok(())
    }
}

// Validation functions for InitConfig
fn validate_transport_type(transport: &str) -> Result<(), validator::ValidationError> {
    match transport {
        "stdio" | "socket" => Ok(()),
        _ => Err(validator::ValidationError::new(
            "Transport must be 'stdio' or 'socket'",
        )),
    }
}

fn validate_log_level(level: &str) -> Result<(), validator::ValidationError> {
    match level {
        "trace" | "debug" | "info" | "warn" | "error" => Ok(()),
        _ => Err(validator::ValidationError::new("Invalid log level")),
    }
}

/// Custom validation function for workspace path
fn validate_workspace_path(path: &PathBuf) -> Result<(), validator::ValidationError> {
    // Check if path is not empty
    if path.as_os_str().is_empty() {
        return Err(validator::ValidationError::new("workspace_path_empty"));
    }

    // Check if path contains valid UTF-8
    if path.to_str().is_none() {
        return Err(validator::ValidationError::new(
            "workspace_path_invalid_utf8",
        ));
    }

    // Check for reasonable path length (avoid extremely long paths)
    if let Some(path_str) = path.to_str() {
        if path_str.len() > 4096 {
            return Err(validator::ValidationError::new("workspace_path_too_long"));
        }
    }

    Ok(())
}

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use validator::Validate;

// Import actual Goose types
pub use goose::agents::{SessionConfig as GooseSessionConfig, RetryConfig};
pub use goose::session::SessionType;

// Model configuration will be handled through Goose's provider system
// We'll use serde_json::Value as a flexible type for now
pub type GooseModelConfig = serde_json::Value;

/// Per-session agent configuration sent by clients via kaiak/configure endpoint
///
/// This is DIFFERENT from config::settings::ServerSettings which controls the server itself.
/// AgentConfiguration is provided by IDE clients for each individual agent session.
///
/// Example: Client sends this in a kaiak/configure request to customize the agent for a specific project.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AgentConfiguration {
    #[validate(nested)]
    pub workspace: WorkspaceConfig,
    pub model: GooseModelConfig,  // Re-use Goose's model configuration
    #[validate(nested)]
    pub tools: ToolConfig,
    pub session: GooseSessionConfig,  // Re-use Goose's session configuration - validated by Goose
    #[validate(nested)]
    pub permissions: PermissionConfig,
}

/// Per-session workspace configuration (sent by IDE client)
///
/// NOTE: This is DIFFERENT from config::settings::DefaultWorkspaceConfig
/// - DefaultWorkspaceConfig = Server-wide defaults for all sessions
/// - WorkspaceConfig = Client-specified workspace for this specific session
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct WorkspaceConfig {
    #[validate(custom(function = "validate_workspace_path"))]
    pub working_dir: PathBuf,
    #[validate(length(min = 1, message = "At least one include pattern is required"))]
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ToolConfig {
    pub enabled_extensions: Vec<String>,
    #[validate(nested)]
    pub custom_tools: Vec<CustomToolConfig>,
    pub planning_mode: bool,
    #[validate(range(min = 1, max = 10000, message = "max_tool_calls must be between 1 and 10000"))]
    pub max_tool_calls: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PermissionConfig {
    #[validate(length(min = 1, message = "At least one tool permission must be specified"))]
    pub tool_permissions: HashMap<String, ToolPermission>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolPermission {
    Allow,       // Always allow this tool
    Deny,        // Always deny this tool
    Approve,     // Require user approval for this tool
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CustomToolConfig {
    #[validate(length(min = 1, max = 100, message = "Tool name must be between 1 and 100 characters"))]
    pub name: String,
    pub extension_type: ExtensionType,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionType {
    Stdio,
    Sse,
    Platform,
    Frontend,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            include_patterns: vec!["**/*".to_string()],
            exclude_patterns: vec![
                ".git/**".to_string(),
                "target/**".to_string(),
                "node_modules/**".to_string(),
            ],
        }
    }
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            enabled_extensions: vec![
                "developer".to_string(),
                "todo".to_string(),
                "extensionmanager".to_string(),
            ],
            custom_tools: vec![],
            planning_mode: false,
            max_tool_calls: Some(10),
        }
    }
}

impl Default for PermissionConfig {
    fn default() -> Self {
        let mut tool_permissions = HashMap::new();
        tool_permissions.insert("read_file".to_string(), ToolPermission::Allow);
        tool_permissions.insert("write_file".to_string(), ToolPermission::Approve);
        tool_permissions.insert("shell_command".to_string(), ToolPermission::Deny);
        tool_permissions.insert("web_search".to_string(), ToolPermission::Allow);

        Self {
            tool_permissions,
        }
    }
}

impl Default for AgentConfiguration {
    fn default() -> Self {
        Self {
            workspace: WorkspaceConfig::default(),
            model: serde_json::json!({
                "provider": "databricks",
                "model": "databricks-meta-llama-3-1-405b-instruct",
                "temperature": 0.1,
                "max_tokens": 4096
            }),
            tools: ToolConfig::default(),
            session: GooseSessionConfig {
                id: uuid::Uuid::new_v4().to_string(),
                schedule_id: None,
                max_turns: Some(1000),
                retry_config: None,
            },
            permissions: PermissionConfig::default(),
        }
    }
}

impl AgentConfiguration {
    /// Validate and optionally generate a new session ID if none provided
    /// T029: Session validation for client-generated UUIDs
    pub fn ensure_valid_session_id(&mut self) -> Result<(), String> {
        // If session ID is empty, generate a new one
        if self.session.id.is_empty() {
            self.session.id = uuid::Uuid::new_v4().to_string();
            return Ok(());
        }

        // Validate the provided session ID is a valid UUID
        match uuid::Uuid::parse_str(&self.session.id) {
            Ok(_) => Ok(()),
            Err(_) => Err(format!("Invalid UUID format for session ID: {}", self.session.id)),
        }
    }

    /// Create a new configuration with a specific session ID
    /// T029: Utility for creating configurations with validated session IDs
    pub fn with_session_id(session_id: String) -> Result<Self, String> {
        // Validate session ID is a proper UUID
        uuid::Uuid::parse_str(&session_id)
            .map_err(|_| format!("Invalid UUID format for session ID: {}", session_id))?;

        let mut config = Self::default();
        config.session.id = session_id;
        Ok(config)
    }

    /// Get the session ID, ensuring it's valid
    /// T029: Safe access to session ID with validation
    pub fn validated_session_id(&self) -> Result<&str, String> {
        if self.session.id.is_empty() {
            return Err("Session ID is empty".to_string());
        }

        // Validate UUID format
        uuid::Uuid::parse_str(&self.session.id)
            .map_err(|_| format!("Invalid UUID format for session ID: {}", self.session.id))?;

        Ok(&self.session.id)
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
        return Err(validator::ValidationError::new("workspace_path_invalid_utf8"));
    }

    // Check for reasonable path length (avoid extremely long paths)
    if let Some(path_str) = path.to_str() {
        if path_str.len() > 4096 {
            return Err(validator::ValidationError::new("workspace_path_too_long"));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_workspace_config() {
        let config = WorkspaceConfig::default();
        assert!(!config.include_patterns.is_empty());
        assert!(!config.exclude_patterns.is_empty());
        assert!(config.exclude_patterns.contains(&".git/**".to_string()));
    }

    #[test]
    fn test_permission_serialization() {
        let permission = ToolPermission::Approve;
        let serialized = serde_json::to_string(&permission).unwrap();
        assert_eq!(serialized, "\"approve\"");
    }

    #[test]
    fn test_extension_type_serialization() {
        let ext_type = ExtensionType::Stdio;
        let serialized = serde_json::to_string(&ext_type).unwrap();
        assert_eq!(serialized, "\"stdio\"");
    }

    #[test]
    fn test_agent_configuration_default() {
        let config = AgentConfiguration::default();
        assert_eq!(config.tools.enabled_extensions.len(), 3);
        assert!(!config.session.id.is_empty());
    }
}
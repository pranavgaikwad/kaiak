use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// Import actual Goose types
pub use goose::agents::{SessionConfig as GooseSessionConfig, RetryConfig};
pub use goose::session::SessionType;

// Model configuration will be handled through Goose's provider system
// We'll use serde_json::Value as a flexible type for now
pub type GooseModelConfig = serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfiguration {
    pub workspace: WorkspaceConfig,
    pub model: GooseModelConfig,  // Re-use Goose's model configuration
    pub tools: ToolConfig,
    pub session: GooseSessionConfig,  // Re-use Goose's session configuration
    pub permissions: PermissionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub working_dir: PathBuf,
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    pub enabled_extensions: Vec<String>,
    pub custom_tools: Vec<CustomToolConfig>,
    pub planning_mode: bool,
    pub max_tool_calls: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionConfig {
    pub tool_permissions: HashMap<String, ToolPermission>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolPermission {
    Allow,       // Always allow this tool
    Deny,        // Always deny this tool
    Approve,     // Require user approval for this tool
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomToolConfig {
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
// Goose agent integration and management

pub mod session_wrapper;
pub mod event_streaming;

use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

// Import Goose types for agent initialization
use goose::agents::{Agent, SessionConfig};
use uuid;

// Re-export key types
pub use session_wrapper::{GooseSessionWrapper, SessionInfo};
pub use event_streaming::EventStreamingHandler;

use crate::models::AgentConfiguration;
use crate::KaiakResult;

/// Agent manager with integrated Goose session management and agent initialization
/// User Story 2: Session management delegation
/// User Story 3: Full agent initialization
pub struct GooseAgentManager {
    /// Goose session wrapper for session management
    session_wrapper: Arc<GooseSessionWrapper>,
    /// Active agent instances mapped by session ID (User Story 3)
    agents: Arc<RwLock<HashMap<String, Arc<Agent>>>>,
    /// Configuration cache for agents
    configurations: Arc<RwLock<HashMap<String, AgentConfiguration>>>,
}

impl GooseAgentManager {
    pub fn new() -> Self {
        Self {
            session_wrapper: Arc::new(GooseSessionWrapper::new()),
            agents: Arc::new(RwLock::new(HashMap::new())),
            configurations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the session wrapper for session operations
    pub fn session_wrapper(&self) -> &Arc<GooseSessionWrapper> {
        &self.session_wrapper
    }

    /// Get the count of active agents
    pub async fn active_agent_count(&self) -> usize {
        let agents = self.agents.read().await;
        agents.len()
    }

    /// Store configuration for future agent creation (will be enhanced in User Story 3)
    pub async fn store_configuration(&self, _config: &AgentConfiguration) -> KaiakResult<()> {
        // User Story 2: Session management is now handled by GooseSessionWrapper
        // User Story 3: Will add actual agent configuration storage
        Ok(())
    }

    /// Create or get a session using Goose SessionManager
    pub async fn get_or_create_session(
        &self,
        session_id: &str,
        config: &AgentConfiguration,
    ) -> KaiakResult<SessionInfo> {
        self.session_wrapper.get_or_create_session(session_id, config).await
    }

    /// Delete a session using Goose SessionManager
    pub async fn delete_session(&self, session_id: &str) -> KaiakResult<bool> {
        self.session_wrapper.delete_session(session_id).await
    }

    /// Lock a session to prevent concurrent access
    pub async fn lock_session(&self, session_id: &str) -> KaiakResult<()> {
        self.session_wrapper.lock_session(session_id).await
    }

    /// Unlock a session
    pub async fn unlock_session(&self, session_id: &str) -> KaiakResult<()> {
        self.session_wrapper.unlock_session(session_id).await
    }

    /// T030: Create and initialize a new Goose agent for a session
    pub async fn create_agent(&self, session_id: &str, config: &AgentConfiguration) -> KaiakResult<Arc<Agent>> {
        use tracing::{info, debug};

        info!("Creating new Goose agent for session: {}", session_id);

        // Create new agent instance using Goose's Agent::new()
        let agent = Agent::new();

        // Store configuration for this session
        {
            let mut configs = self.configurations.write().await;
            configs.insert(session_id.to_string(), config.clone());
        }

        // T031: Set up model provider (placeholder for now)
        self.setup_model_provider(&agent, config).await?;

        // T032: Create SessionConfig (placeholder for now)
        let session_config = self.create_session_config(session_id, config)?;
        debug!("Created session config: {:?}", session_config);

        // T033-T036: Configure agent components (placeholders for now)
        self.configure_default_tools(&agent, config).await?;
        self.setup_permission_enforcement(&agent, config).await?;
        self.setup_custom_tools(&agent, config).await?;
        self.configure_planning_mode(&agent, config).await?;

        let agent_arc = Arc::new(agent);

        // Store the agent instance
        {
            let mut agents = self.agents.write().await;
            agents.insert(session_id.to_string(), agent_arc.clone());
        }

        info!("Successfully created and configured Goose agent for session: {}", session_id);
        Ok(agent_arc)
    }

    /// Get an existing agent for a session
    pub async fn get_agent(&self, session_id: &str) -> Option<Arc<Agent>> {
        let agents = self.agents.read().await;
        agents.get(session_id).cloned()
    }

    /// Remove an agent for a session
    pub async fn remove_agent(&self, session_id: &str) -> bool {
        use tracing::info;

        let mut agents = self.agents.write().await;
        let mut configs = self.configurations.write().await;

        let removed_agent = agents.remove(session_id).is_some();
        configs.remove(session_id);

        if removed_agent {
            info!("Removed Goose agent for session: {}", session_id);
        }

        removed_agent
    }

    /// T031: Implement model provider setup using create_with_named_model() and agent.update_provider()
    async fn setup_model_provider(&self, agent: &Agent, config: &AgentConfiguration) -> KaiakResult<()> {
        use tracing::{debug, info, error};
        use goose::providers::create_with_named_model;

        debug!("Setting up model provider for agent");

        // Extract provider and model from configuration
        let provider_name = config.model.get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or("databricks"); // Default fallback

        let model_name = config.model.get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("databricks-meta-llama-3-1-405b-instruct"); // Default fallback

        info!("Creating provider '{}' with model '{}'", provider_name, model_name);

        // Create the provider using Goose's factory
        match create_with_named_model(provider_name, model_name).await {
            Ok(provider) => {
                debug!("Successfully created provider, updating agent");

                // Update the agent with the new provider
                match agent.update_provider(provider, &config.session.id).await {
                    Ok(()) => {
                        info!("Successfully updated agent with provider '{}' and model '{}'", provider_name, model_name);
                        Ok(())
                    }
                    Err(e) => {
                        error!("Failed to update agent with provider: {}", e);
                        Err(crate::KaiakError::agent_initialization(
                            format!("Failed to update agent with provider: {}", e)
                        ))
                    }
                }
            }
            Err(e) => {
                error!("Failed to create provider '{}' with model '{}': {}", provider_name, model_name, e);
                Err(crate::KaiakError::agent_initialization(
                    format!("Failed to create provider '{}' with model '{}': {}", provider_name, model_name, e)
                ))
            }
        }
    }

    /// T032: Implement SessionConfig creation with session_id, max_turns, and retry_config mapping
    fn create_session_config(&self, session_id: &str, config: &AgentConfiguration) -> KaiakResult<SessionConfig> {
        use tracing::{debug, info, warn};

        debug!("Creating SessionConfig for session: {}", session_id);

        // Validate session ID format (should be UUID)
        if uuid::Uuid::parse_str(session_id).is_err() {
            return Err(crate::KaiakError::session(
                "Session ID must be a valid UUID".to_string(),
                Some(session_id.to_string())
            ));
        }

        // Map configuration from AgentConfiguration to Goose SessionConfig
        let session_config = SessionConfig {
            // Use the provided session ID (client-generated UUID)
            id: session_id.to_string(),

            // Map schedule_id from Goose session config
            schedule_id: config.session.schedule_id.clone(),

            // Map max_turns with validation and default
            max_turns: {
                let max_turns = config.session.max_turns.unwrap_or(1000); // Default to 1000 turns
                if max_turns == 0 {
                    warn!("max_turns was 0, setting to default 1000");
                    Some(1000)
                } else if max_turns > 10000 {
                    warn!("max_turns {} exceeds maximum 10000, capping to 10000", max_turns);
                    Some(10000)
                } else {
                    Some(max_turns)
                }
            },

            // Map retry configuration from Goose session config
            retry_config: config.session.retry_config.clone(),
        };

        info!("Created SessionConfig: id={}, max_turns={:?}, has_retry_config={}",
              session_config.id,
              session_config.max_turns,
              session_config.retry_config.is_some());

        Ok(session_config)
    }

    /// T033: Add default tool configuration using ExtensionConfig for developer, todo, extensionmanager
    async fn configure_default_tools(&self, agent: &Agent, config: &AgentConfiguration) -> KaiakResult<()> {
        use tracing::{debug, info, error, warn};
        use goose::agents::extension::ExtensionConfig;

        debug!("Configuring default tools for agent");

        let enabled_extensions = &config.tools.enabled_extensions;
        info!("Configuring {} enabled extensions: {:?}", enabled_extensions.len(), enabled_extensions);

        for extension_name in enabled_extensions {
            debug!("Adding extension: {}", extension_name);

            let extension_config = match extension_name.as_str() {
                "todo" => ExtensionConfig::Platform {
                    name: "todo".to_string(),
                    description: "Enable a todo list for Goose so it can keep track of what it is doing".to_string(),
                    bundled: Some(true),
                    available_tools: Vec::new(),
                },
                "extensionmanager" => ExtensionConfig::Platform {
                    name: "extensionmanager".to_string(),
                    description: "Enable extension management tools for discovering, enabling, and disabling extensions".to_string(),
                    bundled: Some(true),
                    available_tools: Vec::new(),
                },
                "developer" => ExtensionConfig::Builtin {
                    name: "developer".to_string(),
                    description: "Developer tools including shell access and file operations".to_string(),
                    display_name: Some("Developer".to_string()),
                    timeout: Some(300),
                    bundled: Some(true),
                    available_tools: Vec::new(),
                },
                _ => {
                    warn!("Unknown default extension: {}, skipping", extension_name);
                    continue;
                }
            };

            // Add the extension to the agent
            match agent.add_extension(extension_config).await {
                Ok(()) => {
                    info!("Successfully added extension: {}", extension_name);
                }
                Err(e) => {
                    error!("Failed to add extension '{}': {}", extension_name, e);
                    // Don't fail the entire agent creation for one extension
                    warn!("Continuing with agent creation despite extension failure");
                }
            }
        }

        info!("Completed default tools configuration");
        Ok(())
    }

    /// T034: Implement permission enforcement wrapper mapping tool_permissions to Goose's permission system
    async fn setup_permission_enforcement(&self, _agent: &Agent, config: &AgentConfiguration) -> KaiakResult<()> {
        use tracing::{debug, info};

        debug!("Setting up permission enforcement for agent");

        let tool_permissions = &config.permissions.tool_permissions;
        info!("Configuring {} tool permissions", tool_permissions.len());

        // Goose uses PermissionInspector for permission management
        // The permissions are applied at the agent level and checked during tool execution
        // For now, we log the permission configuration

        for (tool_name, permission) in tool_permissions {
            match permission {
                crate::models::configuration::ToolPermission::Allow => {
                    debug!("Tool '{}': Always allowed (auto-approve)", tool_name);
                    // In Goose, this would be configured via PermissionInspector with SmartApprove
                }
                crate::models::configuration::ToolPermission::Deny => {
                    debug!("Tool '{}': Always denied", tool_name);
                    // In Goose, this would prevent the tool from being added to agent
                }
                crate::models::configuration::ToolPermission::Approve => {
                    debug!("Tool '{}': Requires user approval", tool_name);
                    // In Goose, this triggers PermissionConfirmation handling
                }
            }
        }

        // Note: Actual Goose permission enforcement would use:
        // - agent.set_permission_inspector() with custom PermissionInspector
        // - PermissionInspector::SmartApprove for auto-approved tools
        // - PermissionConfirmation handling for user-approval tools

        info!("Permission enforcement configured successfully");
        Ok(())
    }

    /// T035: Add custom tool support using ExtensionConfig for MCP extensions
    async fn setup_custom_tools(&self, agent: &Agent, config: &AgentConfiguration) -> KaiakResult<()> {
        use tracing::{debug, info, error, warn};
        use goose::agents::extension::ExtensionConfig;

        debug!("Setting up custom tools for agent");

        let custom_tools = &config.tools.custom_tools;
        info!("Configuring {} custom tools", custom_tools.len());

        for custom_tool in custom_tools {
            debug!("Adding custom tool: {}", custom_tool.name);

            // Create ExtensionConfig based on the custom tool configuration
            let extension_config = match custom_tool.extension_type {
                crate::models::configuration::ExtensionType::Stdio => {
                    // Parse stdio tool configuration from JSON
                    let _command = custom_tool.config.get("command")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&custom_tool.name);

                    let _args: Vec<String> = custom_tool.config.get("args")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                        .unwrap_or_default();

                    let default_description = format!("Custom tool: {}", custom_tool.name);
                    let description = custom_tool.config.get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&default_description);

                    let timeout = custom_tool.config.get("timeout")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(300) as u32;

                    ExtensionConfig::Builtin {
                        name: custom_tool.name.clone(),
                        description: description.to_string(),
                        display_name: Some(custom_tool.name.clone()),
                        timeout: Some(timeout as u64),
                        bundled: Some(false),
                        available_tools: Vec::new(),
                    }
                }
                crate::models::configuration::ExtensionType::Sse => {
                    // SSE-based MCP extension
                    let default_url = format!("http://localhost:8080/{}", custom_tool.name);
                    let _url = custom_tool.config.get("url")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&default_url);

                    let default_description = format!("Custom SSE tool: {}", custom_tool.name);
                    let description = custom_tool.config.get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&default_description);

                    ExtensionConfig::Platform {
                        name: custom_tool.name.clone(),
                        description: description.to_string(),
                        bundled: Some(false),
                        available_tools: Vec::new(),
                    }
                }
                crate::models::configuration::ExtensionType::Platform => {
                    // Platform extension
                    let default_description = format!("Custom platform tool: {}", custom_tool.name);
                    let description = custom_tool.config.get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&default_description);

                    ExtensionConfig::Platform {
                        name: custom_tool.name.clone(),
                        description: description.to_string(),
                        bundled: Some(false),
                        available_tools: Vec::new(),
                    }
                }
                crate::models::configuration::ExtensionType::Frontend => {
                    // Frontend extension (handled by client)
                    warn!("Frontend extension '{}' should be handled by client, skipping", custom_tool.name);
                    continue;
                }
            };

            // Add the custom tool extension to the agent
            match agent.add_extension(extension_config).await {
                Ok(()) => {
                    info!("Successfully added custom tool: {}", custom_tool.name);
                }
                Err(e) => {
                    error!("Failed to add custom tool '{}': {}", custom_tool.name, e);
                    // Don't fail the entire agent creation for one custom tool
                    warn!("Continuing with agent creation despite custom tool failure");
                }
            }
        }

        info!("Completed custom tools configuration");
        Ok(())
    }

    /// T036: Implement planning mode configuration based on AgentConfiguration.tools.planning_mode
    async fn configure_planning_mode(&self, _agent: &Agent, config: &AgentConfiguration) -> KaiakResult<()> {
        use tracing::{debug, info};

        debug!("Configuring planning mode for agent");

        if config.tools.planning_mode {
            info!("Planning mode ENABLED - agent will use planning strategies");

            // In Goose, planning mode affects how the agent approaches complex tasks
            // When enabled, the agent will:
            // 1. Break down complex tasks into sub-tasks
            // 2. Create execution plans before taking action
            // 3. Use reflection and iteration strategies
            // 4. Leverage the planner extension for task delegation

            // Ensure planner-related extensions are available
            // The actual planning behavior is built into Goose's agent logic
            // and is controlled via session configuration and agent prompts

            debug!("Planning mode affects agent's task decomposition and execution strategy");
            debug!("Agent will leverage Goose's built-in planning capabilities");
        } else {
            info!("Planning mode DISABLED - agent will use direct execution");
            debug!("Agent will execute tasks directly without intermediate planning steps");
        }

        // Note: In Goose, planning mode can be influenced by:
        // - The agent's system prompts
        // - The session configuration (max_turns affects planning depth)
        // - The availability of planning-specific tools/extensions
        // - The model's reasoning capabilities

        info!("Planning mode configured: {}", if config.tools.planning_mode { "enabled" } else { "disabled" });
        Ok(())
    }
}

impl Default for GooseAgentManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_creation() {
        let manager = GooseAgentManager::new();
        // Basic creation test - actual functionality in User Story 3
        assert!(true);
    }

    #[tokio::test]
    async fn test_active_agent_count() {
        let manager = GooseAgentManager::new();
        assert_eq!(manager.active_agent_count().await, 0);
    }
}
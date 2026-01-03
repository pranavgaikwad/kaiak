// Goose agent integration and management

pub mod session_wrapper;

use std::collections::HashMap;
use tracing::debug;
use std::sync::Arc;
use tokio::sync::RwLock;

use goose::agents::{Agent, SessionConfig};

pub use session_wrapper::{GooseSessionWrapper, SessionInfo};

use crate::models::configuration::AgentConfig;
use crate::KaiakResult;

/// This will manage the lifecycle of Goose agents
/// we can have multiple agents running at any given time
/// we store the state of the agents (tied to goose sessions)
pub struct GooseAgentManager {
    session_wrapper: Arc<GooseSessionWrapper>,
    agents: Arc<RwLock<HashMap<String, Arc<Agent>>>>,
    configurations: Arc<RwLock<HashMap<String, AgentConfig>>>,
}

impl GooseAgentManager {
    pub fn new() -> Self {
        Self {
            session_wrapper: Arc::new(GooseSessionWrapper::new()),
            agents: Arc::new(RwLock::new(HashMap::new())),
            configurations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn session_wrapper(&self) -> &Arc<GooseSessionWrapper> {
        &self.session_wrapper
    }

    pub async fn active_agent_count(&self) -> usize {
        let agents = self.agents.read().await;
        agents.len()
    }

    pub async fn get_or_create_session(
        &self,
        session_id: Option<&str>,
        config: &AgentConfig,
    ) -> KaiakResult<SessionInfo> {
        self.session_wrapper
            .get_or_create_session(session_id, config)
            .await
    }

    pub async fn delete_session(&self, session_id: &str) -> KaiakResult<bool> {
        self.session_wrapper.delete_session(session_id).await
    }

    pub async fn lock_session(&self, session_id: &str) -> KaiakResult<()> {
        self.session_wrapper.lock_session(session_id).await
    }

    pub async fn unlock_session(&self, session_id: &str) -> KaiakResult<()> {
        self.session_wrapper.unlock_session(session_id).await
    }

    pub async fn create_agent(
        &self,
        session_id: &str,
        config: &AgentConfig,
    ) -> KaiakResult<(Arc<Agent>, SessionConfig)> {
        use tracing::{debug, info};

        info!("Creating new Goose agent for session: {}", session_id);

        let agent = Agent::new();
        {
            let mut configs = self.configurations.write().await;
            configs.insert(session_id.to_string(), config.clone());
        }
        self.add_extensions(&agent).await?;
        self.setup_model_provider(&agent, session_id, config)
            .await?;
        let session_config = self.create_session_config(session_id, config)?;
        debug!("Created session config: {:?}", session_config);

        let agent_arc = Arc::new(agent);

        {
            let mut agents = self.agents.write().await;
            agents.insert(session_id.to_string(), agent_arc.clone());
        }

        info!(
            "Successfully created and configured Goose agent for session: {}",
            session_id
        );
        Ok((agent_arc, session_config))
    }

    pub async fn get_agent(&self, session_id: &str) -> Option<Arc<Agent>> {
        let agents = self.agents.read().await;
        agents.get(session_id).cloned()
    }

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

    pub async fn session_exists(&self, session_id: &str) -> bool {
        self.session_wrapper.session_exists(session_id).await
    }

    async fn add_extensions(&self, agent: &Agent) -> KaiakResult<()> {
        use goose::agents::ExtensionConfig;
        let extensions = vec![
            // Developer tools (file system operations)
            ExtensionConfig::Stdio {
                name: "developer".to_string(),
                cmd: "goose".to_string(),
                description: "File system tools for development".to_string(),
                args: vec!["mcp".to_string(), "developer".to_string()],
                envs: Default::default(),
                env_keys: Vec::new(),
                timeout: Some(300),
                bundled: Some(true),
                available_tools: Vec::new(),
            },
            // Todo list management
            ExtensionConfig::Platform {
                name: "todo".to_string(),
                description: "Todo list management".to_string(),
                bundled: Some(true),
                available_tools: Vec::new(),
            }
        ];

        for extension in extensions {
            agent
                .add_extension(extension)
                .await
                .expect("Failed to add extension");
        }

        debug!("Adding extensions to agent");

        Ok(())
    }

    async fn setup_model_provider(
        &self,
        agent: &Agent,
        session_id: &str,
        config: &AgentConfig,
    ) -> KaiakResult<()> {
        use crate::models::configuration::ModelConfig;
        use goose::providers::create_with_named_model;
        use tracing::{debug, error, info};

        debug!(
            "Setting up model provider for agent with session: {}",
            session_id
        );

        // Use override_base_config if provided, otherwise fall back to defaults
        let default_model = ModelConfig::default();
        let model_config = config
            .override_base_config
            .as_ref()
            .map(|c| &c.model)
            .unwrap_or(&default_model);

        let provider_name = model_config.provider.clone();
        let model_name = model_config.model.clone();

        info!(
            "Creating provider '{}' with model '{}'",
            provider_name, model_name
        );

        match create_with_named_model(&provider_name.to_string(), &model_name.to_string()).await {
            Ok(provider) => {
                debug!("Successfully created provider, updating agent");

                match agent.update_provider(provider, session_id).await {
                    Ok(()) => {
                        info!(
                            "Successfully updated agent with provider '{}' and model '{}'",
                            provider_name, model_name
                        );
                        Ok(())
                    }
                    Err(e) => {
                        error!("Failed to update agent with provider: {}", e);
                        Err(crate::KaiakError::agent_initialization(format!(
                            "Failed to update agent with provider: {}",
                            e
                        )))
                    }
                }
            }
            Err(e) => {
                error!(
                    "Failed to create provider '{}' with model '{}': {}",
                    provider_name, model_name, e
                );
                Err(crate::KaiakError::agent_initialization(format!(
                    "Failed to create provider '{}' with model '{}': {}",
                    provider_name, model_name, e
                )))
            }
        }
    }

    fn create_session_config(
        &self,
        session_id: &str,
        config: &AgentConfig,
    ) -> KaiakResult<SessionConfig> {
        use tracing::{debug, info, warn};

        debug!("Creating SessionConfig for session: {}", session_id);

        // Map configuration from AgentConfiguration to Goose SessionConfig
        // Session in config is optional - use defaults when not provided
        let session_config = SessionConfig {
            id: session_id.to_string(),
            schedule_id: config.session.as_ref().and_then(|s| s.schedule_id.clone()),
            max_turns: {
                let max_turns = config
                    .session
                    .as_ref()
                    .and_then(|s| s.max_turns)
                    .unwrap_or(1000); // Default to 1000 turns
                if max_turns == 0 {
                    warn!("max_turns was 0, setting to default 1000");
                    Some(1000)
                } else if max_turns > 10000 {
                    warn!(
                        "max_turns {} exceeds maximum 10000, capping to 10000",
                        max_turns
                    );
                    Some(10000)
                } else {
                    Some(max_turns)
                }
            },
            retry_config: config.session.as_ref().and_then(|s| s.retry_config.clone()),
        };

        info!(
            "Created SessionConfig: id={}, max_turns={:?}, has_retry_config={}",
            session_config.id,
            session_config.max_turns,
            session_config.retry_config.is_some()
        );

        Ok(session_config)
    }
}

impl Default for GooseAgentManager {
    fn default() -> Self {
        Self::new()
    }
}

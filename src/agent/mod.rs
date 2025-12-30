// Goose agent integration and management

pub mod session_wrapper;

use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

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
        self.session_wrapper.get_or_create_session(session_id, config).await
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

    pub async fn create_agent(&self, session_id: &str, config: &AgentConfig) -> KaiakResult<(Arc<Agent>, SessionConfig)> {
        use tracing::{info, debug};

        info!("Creating new Goose agent for session: {}", session_id);

        let agent = Agent::new();
        {
            let mut configs = self.configurations.write().await;
            configs.insert(session_id.to_string(), config.clone());
        }
        self.setup_model_provider(&agent, config).await?;
        let session_config = self.create_session_config(session_id, config)?;
        debug!("Created session config: {:?}", session_config);

        let agent_arc = Arc::new(agent);

        {
            let mut agents = self.agents.write().await;
            agents.insert(session_id.to_string(), agent_arc.clone());
        }

        info!("Successfully created and configured Goose agent for session: {}", session_id);
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

    async fn setup_model_provider(&self, agent: &Agent, config: &AgentConfig) -> KaiakResult<()> {
        use tracing::{debug, info, error};
        use goose::providers::create_with_named_model;

        debug!("Setting up model provider for agent");

        let provider_name = config.override_base_config.as_ref().unwrap().model.provider.clone();
        let model_name = config.override_base_config.as_ref().unwrap().model.model.clone();

        info!("Creating provider '{}' with model '{}'", provider_name, model_name);

        match create_with_named_model(&provider_name.to_string(), &model_name.to_string()).await {
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

    fn create_session_config(&self, session_id: &str, config: &AgentConfig) -> KaiakResult<SessionConfig> {
        use tracing::{debug, info, warn};

        debug!("Creating SessionConfig for session: {}", session_id);

        // Map configuration from AgentConfiguration to Goose SessionConfig
        let session_config = SessionConfig {
            id: session_id.to_string(),
            schedule_id: config.session.schedule_id.clone(),
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
            retry_config: config.session.retry_config.clone(),
        };

        info!("Created SessionConfig: id={}, max_turns={:?}, has_retry_config={}",
              session_config.id,
              session_config.max_turns,
              session_config.retry_config.is_some());

        Ok(session_config)
    }
}

impl Default for GooseAgentManager {
    fn default() -> Self {
        Self::new()
    }
}

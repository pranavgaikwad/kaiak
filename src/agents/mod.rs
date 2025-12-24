// Goose agent integration and management

pub mod goose_integration;
pub mod session_wrapper;
pub mod event_streaming;

// Re-export key types and managers
pub use goose_integration::{GooseAgentManager, AgentError};
pub use session_wrapper::{GooseSessionWrapper, SessionError};
pub use event_streaming::{EventStreamingHandler, StreamingError};

// Import Goose types that we'll use throughout the agents module
pub use goose::agents::{Agent, AgentEvent, ExtensionConfig, SessionConfig};
pub use goose::session::{Session, SessionManager, SessionType};
pub use goose::providers;

use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

use crate::models::{AgentSession, SessionStatus, AgentConfiguration};

/// Central manager for all Goose agent operations
/// This struct coordinates between session management, agent lifecycle, and event streaming
#[derive(Debug)]
pub struct GooseAgentManager {
    /// Goose session wrapper for session operations
    session_wrapper: Arc<GooseSessionWrapper>,
    /// Active agent instances keyed by session ID
    active_agents: Arc<RwLock<HashMap<String, Arc<Agent>>>>,
    /// Event streaming handler for real-time notifications
    event_handler: Arc<EventStreamingHandler>,
}

impl GooseAgentManager {
    pub fn new() -> Self {
        Self {
            session_wrapper: Arc::new(GooseSessionWrapper::new()),
            active_agents: Arc::new(RwLock::new(HashMap::new())),
            event_handler: Arc::new(EventStreamingHandler::new()),
        }
    }

    /// Get the session wrapper for session operations
    pub fn session_wrapper(&self) -> &Arc<GooseSessionWrapper> {
        &self.session_wrapper
    }

    /// Get the event streaming handler
    pub fn event_handler(&self) -> &Arc<EventStreamingHandler> {
        &self.event_handler
    }

    /// Create or get an existing agent for the given session
    pub async fn get_or_create_agent(&self, session_id: &str, config: &AgentConfiguration) -> Result<Arc<Agent>, AgentError> {
        let mut agents = self.active_agents.write().await;

        if let Some(agent) = agents.get(session_id) {
            Ok(agent.clone())
        } else {
            let agent = self.create_agent(session_id, config).await?;
            let agent_arc = Arc::new(agent);
            agents.insert(session_id.to_string(), agent_arc.clone());
            Ok(agent_arc)
        }
    }

    /// Create a new agent instance with the given configuration
    async fn create_agent(&self, session_id: &str, config: &AgentConfiguration) -> Result<Agent, AgentError> {
        // This will be implemented in the agent initialization phase
        // For now, return a placeholder error
        Err(AgentError::NotImplemented("Agent creation not yet implemented".to_string()))
    }

    /// Remove an agent from the active agents map
    pub async fn remove_agent(&self, session_id: &str) -> Option<Arc<Agent>> {
        let mut agents = self.active_agents.write().await;
        agents.remove(session_id)
    }

    /// Get the count of active agents
    pub async fn active_agent_count(&self) -> usize {
        let agents = self.active_agents.read().await;
        agents.len()
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
        // Basic creation test - more detailed tests will be added during implementation
        assert!(Arc::strong_count(&manager.session_wrapper) == 1);
    }

    #[tokio::test]
    async fn test_active_agent_count() {
        let manager = GooseAgentManager::new();
        assert_eq!(manager.active_agent_count().await, 0);
    }
}
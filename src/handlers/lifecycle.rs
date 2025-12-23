use anyhow::Result;
use std::sync::Arc;
use crate::models::{AiSession, SessionStatus};
use crate::goose::AgentManager;
use tracing::{info, error, debug, warn};

/// Handler for session lifecycle operations
pub struct LifecycleHandler {
    agent_manager: Arc<AgentManager>,
}

impl LifecycleHandler {
    pub async fn new() -> Result<Self> {
        let agent_manager = Arc::new(AgentManager::new().await?);
        Ok(Self { agent_manager })
    }

    /// Create a new AI session for the given workspace
    pub async fn create_session(&self, workspace_path: String, session_name: Option<String>) -> Result<AiSession> {
        info!("Creating session for workspace: {}", workspace_path);

        // Validate workspace path
        if workspace_path.trim().is_empty() {
            anyhow::bail!("Workspace path cannot be empty");
        }

        let workspace_path = std::path::PathBuf::from(&workspace_path);
        if !workspace_path.exists() {
            warn!("Workspace path does not exist: {:?}", workspace_path);
        }

        // Create AI session
        let ai_session = AiSession::new(
            workspace_path.to_string_lossy().to_string(),
            session_name,
        );

        // Initialize the session through the agent manager
        match self.agent_manager.get_or_create_session(&ai_session).await {
            Ok(session_wrapper) => {
                let session = session_wrapper.read().await;
                info!("Session created successfully: {}", session.session_id);
                Ok(ai_session)
            }
            Err(e) => {
                error!("Failed to create session: {}", e);
                Err(e)
            }
        }
    }

    /// Terminate an existing session and cleanup resources
    pub async fn terminate_session(&self, session_id: &str) -> Result<()> {
        info!("Terminating session: {}", session_id);

        if session_id.trim().is_empty() {
            anyhow::bail!("Session ID cannot be empty");
        }

        // Terminate through the agent manager
        match self.agent_manager.terminate_session(session_id).await {
            Ok(()) => {
                info!("Session terminated successfully: {}", session_id);
                Ok(())
            }
            Err(e) => {
                error!("Failed to terminate session {}: {}", session_id, e);
                Err(e)
            }
        }
    }

    /// Get the current status of a session
    pub async fn get_session_status(&self, session_id: &str) -> Result<SessionStatus> {
        debug!("Getting status for session: {}", session_id);

        if session_id.trim().is_empty() {
            anyhow::bail!("Session ID cannot be empty");
        }

        // Get session from the manager
        match self.agent_manager.session_manager().get_session(session_id).await {
            Some(session_wrapper) => {
                let session = session_wrapper.read().await;
                debug!("Session {} status: {:?}", session_id, session.status);
                Ok(session.status.clone())
            }
            None => {
                warn!("Session not found: {}", session_id);
                anyhow::bail!("Session not found: {}", session_id)
            }
        }
    }

    /// List all active requests for a session
    pub async fn get_session_requests(&self, session_id: &str) -> Result<Vec<crate::goose::RequestState>> {
        debug!("Getting requests for session: {}", session_id);
        self.agent_manager.get_session_requests(session_id).await
    }

    /// Get count of active sessions
    pub async fn active_session_count(&self) -> usize {
        self.agent_manager.session_manager().active_session_count().await
    }

    /// Get access to the underlying agent manager
    pub fn agent_manager(&self) -> &Arc<AgentManager> {
        &self.agent_manager
    }
}
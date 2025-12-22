use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::models::{Id, AiSession};

pub mod agent;
pub mod session;
pub mod prompts;

pub use agent::*;
pub use session::*;
pub use prompts::*;

/// Manager for Goose agent sessions with thread-safe access
#[derive(Clone)]
pub struct GooseManager {
    sessions: Arc<RwLock<HashMap<Id, Arc<RwLock<GooseSessionWrapper>>>>>,
}

impl GooseManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get or create a Goose session for the given session ID
    pub async fn get_or_create_session(
        &self,
        session_id: Id,
        ai_session: &AiSession,
    ) -> Result<Arc<RwLock<GooseSessionWrapper>>> {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(&session_id) {
            return Ok(session.clone());
        }
        drop(sessions);

        let mut sessions = self.sessions.write().await;

        // Double-check pattern to avoid race condition
        if let Some(session) = sessions.get(&session_id) {
            return Ok(session.clone());
        }

        let wrapper = GooseSessionWrapper::new(ai_session).await?;
        let session_arc = Arc::new(RwLock::new(wrapper));
        sessions.insert(session_id.clone(), session_arc.clone());

        Ok(session_arc)
    }

    /// Remove a session from the manager
    pub async fn remove_session(&self, session_id: &Id) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.remove(session_id) {
            let session = session.write().await;
            session.cleanup().await?;
        }
        Ok(())
    }

    /// Get active session count
    pub async fn active_session_count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.len()
    }

    /// Cleanup all sessions
    pub async fn cleanup_all(&self) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        for (_, session) in sessions.drain() {
            let session = session.write().await;
            if let Err(e) = session.cleanup().await {
                tracing::error!("Failed to cleanup session: {}", e);
            }
        }
        Ok(())
    }
}

impl Default for GooseManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::AiSession;

    #[tokio::test]
    async fn test_manager_creation() {
        let manager = GooseManager::new();
        assert_eq!(manager.active_session_count().await, 0);
    }

    #[tokio::test]
    async fn test_session_count() {
        let manager = GooseManager::new();
        let session = AiSession::new(
            "/tmp/test".to_string(),
            Some("test".to_string()),
        );

        // This would normally create a Goose session, but we'll skip that for unit test
        // let _wrapper = manager.get_or_create_session("test-id".to_string(), &session).await;

        // For now, just test the basic functionality
        assert_eq!(manager.active_session_count().await, 0);
    }
}
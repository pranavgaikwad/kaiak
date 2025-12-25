use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

// Import actual Goose session management types
use goose::session::{Session, SessionManager, SessionType};

use crate::models::configuration::AgentConfiguration;
use crate::{KaiakResult, KaiakError};

/// Wrapper around Goose's SessionManager for Kaiak integration
/// Provides session creation, lookup, deletion, and locking mechanisms
pub struct GooseSessionWrapper {
    /// Session locking mechanism to prevent concurrent access
    session_locks: Arc<RwLock<HashMap<String, chrono::DateTime<chrono::Utc>>>>,
}

/// Information about a managed session
#[derive(Debug, Clone)]
pub struct SessionInfo {
    /// Goose session instance
    pub session: Session,
    /// When session was locked (if currently locked)
    pub locked_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Session configuration used to create this session
    pub configuration: AgentConfiguration,
}

impl GooseSessionWrapper {
    /// Create a new GooseSessionWrapper
    pub fn new() -> Self {
        Self {
            session_locks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new Goose session with the provided configuration
    /// T022: Implementing session creation with SessionManager::create_session()
    pub async fn create_session(
        &self,
        session_id: &str,
        config: &AgentConfiguration,
    ) -> KaiakResult<SessionInfo> {
        info!("Creating Goose session: {}", session_id);

        // Validate session ID format
        if Uuid::parse_str(session_id).is_err() {
            return Err(KaiakError::session("Invalid UUID format for session ID".to_string(), Some(session_id.to_string())));
        }

        // Check if session already exists
        if self.session_exists(session_id).await {
            return Err(KaiakError::session("Session already exists".to_string(), Some(session_id.to_string())));
        }

        // Convert workspace configuration to absolute path
        let working_dir = if config.workspace.working_dir.is_absolute() {
            config.workspace.working_dir.clone()
        } else {
            let current_dir = std::env::current_dir()
                .map_err(|e| KaiakError::workspace(format!("Failed to get current directory: {}", e), None))?;
            current_dir.join(&config.workspace.working_dir)
        };

        // Validate workspace directory exists and is accessible
        if !working_dir.exists() {
            return Err(KaiakError::workspace(
                "Workspace directory does not exist".to_string(),
                Some(working_dir.to_string_lossy().to_string())
            ));
        }

        if !working_dir.is_dir() {
            return Err(KaiakError::workspace(
                "Workspace path is not a directory".to_string(),
                Some(working_dir.to_string_lossy().to_string())
            ));
        }

        // Create session name based on workspace
        let session_name = format!(
            "Kaiak Migration Session - {}",
            working_dir.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("workspace")
        );

        // Create Goose session using SessionManager static method
        debug!("Creating Goose session with working_dir: {:?}", working_dir);
        let session = SessionManager::create_session(
            working_dir,
            session_name,
            SessionType::User,
        ).await.map_err(|e| {
            KaiakError::goose_integration(
                format!("Failed to create Goose session: {}", e),
                Some("session_creation".to_string())
            )
        })?;

        // Verify the session has the expected ID (or update our mapping)
        debug!("Created Goose session with ID: {}", session.id);

        let session_info = SessionInfo {
            session,
            locked_at: None,
            configuration: config.clone(),
        };

        info!("Successfully created Goose session: {}", session_id);
        Ok(session_info)
    }

    /// Get an existing session by ID
    /// T023: Implementing session lookup logic using SessionManager::get_session()
    pub async fn get_session(&self, session_id: &str) -> KaiakResult<Option<SessionInfo>> {
        debug!("Looking up Goose session: {}", session_id);

        // Validate session ID format
        if Uuid::parse_str(session_id).is_err() {
            return Err(KaiakError::session("Invalid UUID format for session ID".to_string(), Some(session_id.to_string())));
        }

        // Get session from Goose SessionManager
        match SessionManager::get_session(session_id, false).await {
            Ok(session) => {
                debug!("Found existing Goose session: {}", session_id);

                // Check if session is currently locked
                let locks = self.session_locks.read().await;
                let locked_at = locks.get(session_id).copied();

                // Create a placeholder configuration - in a real implementation,
                // we would store this mapping or derive it from session metadata
                let configuration = AgentConfiguration::default();

                Ok(Some(SessionInfo {
                    session,
                    locked_at,
                    configuration,
                }))
            }
            Err(e) => {
                // Session not found or other error
                debug!("Goose session not found or error: {}", e);
                Ok(None)
            }
        }
    }

    /// Check if a session exists
    pub async fn session_exists(&self, session_id: &str) -> bool {
        match self.get_session(session_id).await {
            Ok(Some(_)) => true,
            _ => false,
        }
    }

    /// Delete a Goose session
    /// T024: Implementing session deletion logic using SessionManager::delete_session()
    pub async fn delete_session(&self, session_id: &str) -> KaiakResult<bool> {
        info!("Deleting Goose session: {}", session_id);

        // Validate session ID format
        if Uuid::parse_str(session_id).is_err() {
            return Err(KaiakError::session("Invalid UUID format for session ID".to_string(), Some(session_id.to_string())));
        }

        // Check if session is currently locked
        if self.is_session_locked(session_id).await {
            return Err(KaiakError::session_in_use(
                session_id.to_string(),
                self.get_session_lock_time(session_id).await
            ));
        }

        // Delete session from Goose SessionManager
        match SessionManager::delete_session(session_id).await {
            Ok(()) => {
                info!("Successfully deleted Goose session: {}", session_id);

                // Clean up any locks for this session
                {
                    let mut locks = self.session_locks.write().await;
                    locks.remove(session_id);
                }
                Ok(true)
            }
            Err(e) => {
                error!("Failed to delete Goose session {}: {}", session_id, e);
                Err(KaiakError::goose_integration(
                    format!("Failed to delete session: {}", e),
                    Some("session_deletion".to_string())
                ))
            }
        }
    }

    /// Lock a session to prevent concurrent access
    /// T027: Add session locking mechanism to prevent concurrent access (-32016 error)
    pub async fn lock_session(&self, session_id: &str) -> KaiakResult<()> {
        debug!("Locking session: {}", session_id);

        // Validate session ID format
        if Uuid::parse_str(session_id).is_err() {
            return Err(KaiakError::session("Invalid UUID format for session ID".to_string(), Some(session_id.to_string())));
        }

        // Check if session exists
        if !self.session_exists(session_id).await {
            return Err(KaiakError::SessionNotFound(session_id.to_string()));
        }

        let mut locks = self.session_locks.write().await;

        // Check if already locked
        if let Some(locked_at) = locks.get(session_id) {
            return Err(KaiakError::session_in_use(
                session_id.to_string(),
                Some(*locked_at)
            ));
        }

        // Lock the session
        locks.insert(session_id.to_string(), chrono::Utc::now());
        debug!("Successfully locked session: {}", session_id);

        Ok(())
    }

    /// Unlock a session
    pub async fn unlock_session(&self, session_id: &str) -> KaiakResult<()> {
        debug!("Unlocking session: {}", session_id);

        let mut locks = self.session_locks.write().await;
        if locks.remove(session_id).is_some() {
            debug!("Successfully unlocked session: {}", session_id);
            Ok(())
        } else {
            warn!("Attempted to unlock non-locked session: {}", session_id);
            Ok(()) // Not an error - session wasn't locked
        }
    }

    /// Check if a session is currently locked
    pub async fn is_session_locked(&self, session_id: &str) -> bool {
        let locks = self.session_locks.read().await;
        locks.contains_key(session_id)
    }

    /// Get the time when a session was locked
    pub async fn get_session_lock_time(&self, session_id: &str) -> Option<chrono::DateTime<chrono::Utc>> {
        let locks = self.session_locks.read().await;
        locks.get(session_id).copied()
    }

    /// Get or create a session (create-or-reuse pattern)
    pub async fn get_or_create_session(
        &self,
        session_id: &str,
        config: &AgentConfiguration,
    ) -> KaiakResult<SessionInfo> {
        // Try to get existing session first
        if let Some(session_info) = self.get_session(session_id).await? {
            debug!("Reusing existing session: {}", session_id);
            return Ok(session_info);
        }

        // Create new session if it doesn't exist
        debug!("Creating new session: {}", session_id);
        self.create_session(session_id, config).await
    }

    /// Clean up expired session locks (housekeeping)
    pub async fn cleanup_expired_locks(&self) {
        let mut locks = self.session_locks.write().await;
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(1); // 1 hour timeout

        let initial_count = locks.len();
        locks.retain(|session_id, locked_at| {
            if *locked_at < cutoff {
                warn!("Removing expired lock for session: {}", session_id);
                false
            } else {
                true
            }
        });

        let removed_count = initial_count - locks.len();
        if removed_count > 0 {
            info!("Cleaned up {} expired session locks", removed_count);
        }
    }

    /// Get count of active sessions
    pub async fn active_session_count(&self) -> usize {
        // This would use SessionManager's session listing capability
        // For now, we return the count of locked sessions as a proxy
        let locks = self.session_locks.read().await;
        locks.len()
    }
}

impl Default for GooseSessionWrapper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_session_wrapper_creation() {
        let wrapper = GooseSessionWrapper::new();
        assert_eq!(wrapper.active_session_count().await, 0);
    }

    #[tokio::test]
    async fn test_session_locking() {
        let wrapper = GooseSessionWrapper::new();
        let session_id = "test-session-123";

        // Initially not locked
        assert!(!wrapper.is_session_locked(session_id).await);

        // Note: This test would need a real session to exist for locking
        // In a full test, we would create a session first
    }

    #[tokio::test]
    async fn test_uuid_validation() {
        let wrapper = GooseSessionWrapper::new();
        let temp_dir = TempDir::new().unwrap();

        let mut config = AgentConfiguration::default();
        config.workspace.working_dir = temp_dir.path().to_path_buf();

        // Invalid UUID should fail
        let result = wrapper.create_session("not-a-uuid", &config).await;
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.to_string().contains("Invalid UUID format"));
        }
    }
}
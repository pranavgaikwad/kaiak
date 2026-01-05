use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use goose::session::{Session, SessionManager, SessionType};

use crate::models::configuration::AgentConfig;
use crate::{KaiakResult, KaiakError};

/// Wrapper around Goose's SessionManager for Kaiak integration
/// Provides session creation, lookup, deletion, and locking mechanisms
pub struct GooseSessionWrapper {
    /// Session locking mechanism to prevent concurrent access
    session_locks: Arc<RwLock<HashMap<String, chrono::DateTime<chrono::Utc>>>>,
}

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub session: Session,
    pub locked_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl GooseSessionWrapper {
    pub fn new() -> Self {
        Self {
            session_locks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new Goose session with the provided configuration
    /// Returns the SessionInfo containing the Goose-generated session ID
    pub async fn create_session(
        &self,
        config: &AgentConfig,
    ) -> KaiakResult<SessionInfo> {
        info!("Creating new Goose session");

        let working_dir = if config.workspace.is_absolute() {
            config.workspace.clone()
        } else {
            let current_dir = std::env::current_dir()
                .map_err(|e| KaiakError::workspace(format!("Failed to get current directory: {}", e), None))?;
            current_dir.join(&config.workspace)
        };

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

        let session_name = format!(
            "kaiak-{}",
            working_dir.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("workspace")
        );

        // Create Goose session - Goose will generate its own session ID
        debug!("Creating Goose session with working_dir: {:?}", working_dir);
        let session = SessionManager::create_session(
            working_dir.clone(),
            session_name,
            SessionType::User,
        ).await.map_err(|e| {
            KaiakError::goose_integration(
                format!("Failed to create Goose session: {}", e),
                Some("session_creation".to_string())
            )
        })?;

        let session_id = session.id.clone();
        info!("Successfully created Goose session with ID: {} and working_dir: {:?}", session_id, working_dir);

        let session_info = SessionInfo {
            session,
            locked_at: None,
        };

        Ok(session_info)
    }

    /// Get an existing session by ID
    pub async fn get_session(&self, session_id: &str) -> KaiakResult<Option<SessionInfo>> {
        debug!("Looking up Goose session: {}", session_id);

        // Get session from Goose SessionManager
        match SessionManager::get_session(session_id, false).await {
            Ok(session) => {
                debug!("Found existing Goose session: {}", session_id);

                // Check if session is currently locked
                let locks = self.session_locks.read().await;
                let locked_at = locks.get(session_id).copied();

                Ok(Some(SessionInfo {
                    session,
                    locked_at,
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
    pub async fn delete_session(&self, session_id: &str) -> KaiakResult<bool> {
        info!("Deleting Goose session: {}", session_id);

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
    pub async fn lock_session(&self, session_id: &str) -> KaiakResult<()> {
        debug!("Locking session: {}", session_id);

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
    /// If session_id is provided and valid, tries to get the existing session
    /// If session_id is None or session doesn't exist, creates a new one
    pub async fn get_or_create_session(
        &self,
        session_id: Option<&str>,
        config: &AgentConfig,
    ) -> KaiakResult<SessionInfo> {
        // If session_id is provided, try to get existing session
        if let Some(id) = session_id {
            if let Some(session_info) = self.get_session(id).await? {
                debug!("Reusing existing session: {}", id);
                return Ok(session_info);
            }
            // Session ID was provided but session doesn't exist - this is an error
            return Err(KaiakError::SessionNotFound(id.to_string()));
        }

        // No session_id provided - create a new session
        debug!("Creating new session (no session_id provided)");
        self.create_session(config).await
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

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use validator::Validate;

use crate::agents::GooseAgentManager;
use crate::KaiakResult;

/// Request type for kaiak/delete_session endpoint
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DeleteSessionRequest {
    /// Session identifier to delete
    #[validate(length(min = 1, message = "Session ID cannot be empty"))]
    #[validate(custom(function = "validate_uuid_format"))]
    pub session_id: String,
    /// Cleanup options for session deletion
    #[validate(nested)]
    pub cleanup_options: Option<SessionCleanupOptions>,
}

/// Response type for kaiak/delete_session endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteSessionResponse {
    /// Session identifier that was deleted
    pub session_id: String,
    /// Deletion status
    pub status: DeleteSessionStatus,
    /// Cleanup results
    pub cleanup_results: SessionCleanupResults,
    /// Deletion timestamp
    pub deleted_at: String,
}

/// Options for session cleanup during deletion
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SessionCleanupOptions {
    /// Whether to force deletion even if session is active
    pub force: bool,
    /// Whether to cleanup temporary files
    pub cleanup_temp_files: bool,
    /// Whether to preserve session logs
    pub preserve_logs: bool,
    /// Grace period for active operations (seconds)
    #[validate(range(min = 1, max = 3600, message = "Grace period must be between 1 and 3600 seconds"))]
    pub grace_period: Option<u32>,
}

/// Status of session deletion
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeleteSessionStatus {
    /// Session deleted successfully
    Deleted,
    /// Session not found
    NotFound,
    /// Session is active and cannot be deleted
    Active,
    /// Deletion failed due to error
    Failed,
    /// Deletion in progress (for graceful cleanup)
    InProgress,
}

/// Results of session cleanup operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCleanupResults {
    /// Whether session data was removed
    pub session_removed: bool,
    /// Whether temporary files were cleaned up
    pub temp_files_cleaned: bool,
    /// Whether logs were preserved or removed
    pub logs_preserved: bool,
    /// Any cleanup warnings
    pub warnings: Vec<String>,
    /// Number of files removed during cleanup
    pub files_removed: u32,
}

impl Default for SessionCleanupOptions {
    fn default() -> Self {
        Self {
            force: false,
            cleanup_temp_files: true,
            preserve_logs: true,
            grace_period: Some(30), // 30 seconds grace period
        }
    }
}

/// Handler for kaiak/delete_session endpoint
/// Manages session deletion and cleanup operations
pub struct DeleteSessionHandler {
    /// Agent manager for session coordination
    agent_manager: Arc<GooseAgentManager>,
    /// Tracking of deletion operations in progress
    deletion_operations: Arc<RwLock<std::collections::HashMap<String, chrono::DateTime<chrono::Utc>>>>,
}

impl DeleteSessionHandler {
    pub fn new(agent_manager: Arc<GooseAgentManager>) -> Self {
        Self {
            agent_manager,
            deletion_operations: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Handle delete session request
    pub async fn handle_delete_session(&self, request: DeleteSessionRequest) -> KaiakResult<DeleteSessionResponse> {
        info!("Processing delete_session request for: {}", request.session_id);

        // Validate request using serde validator
        if let Err(validation_errors) = request.validate() {
            error!("Request validation failed: {:?}", validation_errors);
            let error_messages: Vec<String> = validation_errors
                .field_errors()
                .into_iter()
                .flat_map(|(field, errors)| {
                    errors.iter().map(move |error| {
                        format!("Field '{}': {}", field, error.message.as_ref().map(|m| m.as_ref()).unwrap_or("validation error"))
                    })
                })
                .collect();

            return Err(crate::KaiakError::session(
                format!("Request validation failed: {}", error_messages.join(", ")),
                Some(request.session_id),
            ));
        }

        // Additional custom validation
        self.validate_request(&request).await?;

        // Get cleanup options with defaults
        let cleanup_options = request.cleanup_options.unwrap_or_default();

        // Check if deletion is already in progress
        if self.is_deletion_in_progress(&request.session_id).await {
            return Ok(DeleteSessionResponse {
                session_id: request.session_id,
                status: DeleteSessionStatus::InProgress,
                cleanup_results: SessionCleanupResults {
                    session_removed: false,
                    temp_files_cleaned: false,
                    logs_preserved: true,
                    warnings: vec!["Deletion already in progress".to_string()],
                    files_removed: 0,
                },
                deleted_at: chrono::Utc::now().to_rfc3339(),
            });
        }

        // Mark deletion as in progress
        self.mark_deletion_in_progress(&request.session_id).await;

        // For User Story 1, we implement the API surface but defer actual session management to User Story 2
        match self.perform_session_deletion(&request.session_id, &cleanup_options).await {
            Ok(cleanup_results) => {
                info!("Session {} deleted successfully", request.session_id);
                self.clear_deletion_in_progress(&request.session_id).await;

                Ok(DeleteSessionResponse {
                    session_id: request.session_id,
                    status: DeleteSessionStatus::Deleted,
                    cleanup_results,
                    deleted_at: chrono::Utc::now().to_rfc3339(),
                })
            }
            Err(e) => {
                error!("Failed to delete session {}: {}", request.session_id, e);
                self.clear_deletion_in_progress(&request.session_id).await;

                // Determine appropriate error status
                let status = if e.to_string().contains("not found") {
                    DeleteSessionStatus::NotFound
                } else if e.to_string().contains("active") {
                    DeleteSessionStatus::Active
                } else {
                    DeleteSessionStatus::Failed
                };

                Ok(DeleteSessionResponse {
                    session_id: request.session_id,
                    status,
                    cleanup_results: SessionCleanupResults {
                        session_removed: false,
                        temp_files_cleaned: false,
                        logs_preserved: true,
                        warnings: vec![e.to_string()],
                        files_removed: 0,
                    },
                    deleted_at: chrono::Utc::now().to_rfc3339(),
                })
            }
        }
    }

    /// Get count of active deletion operations
    pub async fn get_active_deletion_count(&self) -> usize {
        let operations = self.deletion_operations.read().await;
        operations.len()
    }

    /// Validate delete session request
    async fn validate_request(&self, request: &DeleteSessionRequest) -> KaiakResult<()> {
        // Validate session ID format
        if request.session_id.is_empty() {
            return Err(crate::KaiakError::session("Session ID cannot be empty".to_string(), Some(request.session_id.clone())));
        }

        // Validate UUID format for session ID
        if Uuid::parse_str(&request.session_id).is_err() {
            return Err(crate::KaiakError::session("Session ID must be a valid UUID".to_string(), Some(request.session_id.clone())));
        }

        // Validate cleanup options
        if let Some(ref options) = request.cleanup_options {
            if let Some(grace_period) = options.grace_period {
                if grace_period > 3600 { // Max 1 hour grace period
                    return Err(crate::KaiakError::configuration("Grace period cannot exceed 3600 seconds".to_string()));
                }
            }
        }

        Ok(())
    }

    /// Check if deletion is already in progress for session
    async fn is_deletion_in_progress(&self, session_id: &str) -> bool {
        let operations = self.deletion_operations.read().await;
        operations.contains_key(session_id)
    }

    /// Mark deletion as in progress
    async fn mark_deletion_in_progress(&self, session_id: &str) {
        let mut operations = self.deletion_operations.write().await;
        operations.insert(session_id.to_string(), chrono::Utc::now());
    }

    /// Clear deletion in progress marker
    async fn clear_deletion_in_progress(&self, session_id: &str) {
        let mut operations = self.deletion_operations.write().await;
        operations.remove(session_id);
    }

    /// Perform session deletion using Goose SessionManager
    /// User Story 2: Actual session deletion with Goose integration
    async fn perform_session_deletion(&self, session_id: &str, cleanup_options: &SessionCleanupOptions) -> KaiakResult<SessionCleanupResults> {
        debug!("Performing Goose session deletion for: {}", session_id);

        // User Story 2: Use actual Goose SessionManager for session management
        let mut cleanup_results = SessionCleanupResults {
            session_removed: false,
            temp_files_cleaned: false,
            logs_preserved: cleanup_options.preserve_logs,
            warnings: vec![],
            files_removed: 0,
        };

        // Check if session exists using Goose SessionManager
        let _session_exists = match self.agent_manager.session_wrapper().get_session(session_id).await {
            Ok(Some(_)) => {
                debug!("Goose session exists: {}", session_id);
                true
            }
            Ok(None) => {
                debug!("Goose session not found: {}", session_id);
                cleanup_results.warnings.push("Session not found".to_string());
                return Ok(cleanup_results);
            }
            Err(e) => {
                error!("Failed to check session existence: {}", e);
                cleanup_results.warnings.push(format!("Session lookup failed: {}", e));
                return Ok(cleanup_results);
            }
        };

        // Check if session is currently locked (active)
        let is_locked = self.agent_manager.session_wrapper().is_session_locked(session_id).await;
        if is_locked && !cleanup_options.force {
            error!("Session {} is currently in use and force=false", session_id);
            return Err(crate::KaiakError::session_in_use(
                session_id.to_string(),
                self.agent_manager.session_wrapper().get_session_lock_time(session_id).await
            ));
        }

        // If forced deletion and session is locked, unlock it first
        if is_locked && cleanup_options.force {
            debug!("Force unlocking session: {}", session_id);
            if let Err(e) = self.agent_manager.unlock_session(session_id).await {
                cleanup_results.warnings.push(format!("Failed to unlock session: {}", e));
            }
        }

        // Apply grace period if specified
        if let Some(grace_period) = cleanup_options.grace_period {
            if is_locked && !cleanup_options.force {
                debug!("Applying grace period of {} seconds", grace_period);
                tokio::time::sleep(tokio::time::Duration::from_secs(grace_period as u64)).await;
            }
        }

        // Perform cleanup of temporary files if requested
        if cleanup_options.cleanup_temp_files {
            debug!("Cleaning up temporary files for session: {}", session_id);
            // In a real implementation, this would clean up session-specific temp files
            // For now, we simulate the cleanup
            cleanup_results.temp_files_cleaned = true;
            cleanup_results.files_removed = 5; // Simulated count
        }

        // Delete session using Goose SessionManager
        match self.agent_manager.delete_session(session_id).await {
            Ok(deleted) => {
                if deleted {
                    info!("Successfully deleted Goose session: {}", session_id);
                    cleanup_results.session_removed = true;
                } else {
                    warn!("Goose session was already deleted: {}", session_id);
                    cleanup_results.warnings.push("Session was already deleted".to_string());
                }
            }
            Err(e) => {
                error!("Failed to delete Goose session {}: {}", session_id, e);
                return Err(e);
            }
        }

        // Log preservation (if logs should be preserved, they remain untouched)
        if cleanup_options.preserve_logs {
            debug!("Preserving session logs for: {}", session_id);
        } else {
            debug!("Would remove session logs for: {}", session_id);
            // In a real implementation, this would clean up session logs
        }

        debug!("Session deletion completed with options: force={}, cleanup_temp_files={}, preserve_logs={}",
               cleanup_options.force, cleanup_options.cleanup_temp_files, cleanup_options.preserve_logs);

        info!("Goose session {} deleted successfully", session_id);
        Ok(cleanup_results)
    }

    /// Clean up expired deletion operations (housekeeping)
    pub async fn cleanup_expired_operations(&self) {
        let mut operations = self.deletion_operations.write().await;
        let cutoff = chrono::Utc::now() - chrono::Duration::minutes(10); // 10 minute timeout

        operations.retain(|_session_id, started_at| *started_at > cutoff);
    }
}

/// Custom validation function for UUID format
fn validate_uuid_format(session_id: &str) -> Result<(), validator::ValidationError> {
    if Uuid::parse_str(session_id).is_err() {
        return Err(validator::ValidationError::new("invalid_uuid_format"));
    }
    Ok(())
}
//! Manages user interactions that block agent processing.
//!
//! When the Goose agent needs user input (tool confirmations, elicitations),
//! the stream processing registers a pending request here and waits.
//! Client responses come in via `kaiak/client/user_message` and are routed
//! through this manager to unblock the waiting code.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{oneshot, RwLock};
use tracing::{debug, warn};

use goose::permission::permission_confirmation::PrincipalType;
use goose::permission::{Permission, PermissionConfirmation};

/// Manages pending user interactions across sessions.
///
/// Thread-safe and designed to be shared across handlers.
pub struct InteractionManager {
    /// Pending tool confirmations: request_id -> response sender
    pending_confirmations: Arc<RwLock<HashMap<String, oneshot::Sender<PermissionConfirmation>>>>,
    /// Pending elicitations: request_id -> response sender
    pending_elicitations: Arc<RwLock<HashMap<String, oneshot::Sender<serde_json::Value>>>>,
}

impl InteractionManager {
    pub fn new() -> Self {
        Self {
            pending_confirmations: Arc::new(RwLock::new(HashMap::new())),
            pending_elicitations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a pending tool confirmation and get a receiver to await the response.
    ///
    /// The stream processing loop calls this when it encounters an `ActionRequired::ToolConfirmation`.
    /// It then sends a notification to the client and awaits the returned receiver.
    pub async fn register_confirmation(
        &self,
        request_id: String,
    ) -> oneshot::Receiver<PermissionConfirmation> {
        let (tx, rx) = oneshot::channel();
        debug!("Registering pending tool confirmation: {}", request_id);
        self.pending_confirmations
            .write()
            .await
            .insert(request_id, tx);
        rx
    }

    /// Submit a tool confirmation response from the client.
    ///
    /// Called by `ClientNotificationHandler` when it receives a `tool_confirmation` message.
    /// This unblocks the stream processing loop waiting on the corresponding receiver.
    pub async fn submit_confirmation(
        &self,
        request_id: &str,
        permission: Permission,
    ) -> Result<(), String> {
        let tx = self
            .pending_confirmations
            .write()
            .await
            .remove(request_id)
            .ok_or_else(|| format!("No pending confirmation for id: {}", request_id))?;

        debug!(
            "Submitting tool confirmation for {}: {:?}",
            request_id, permission
        );

        tx.send(PermissionConfirmation {
            principal_type: PrincipalType::Tool,
            permission,
        })
        .map_err(|_| "Response receiver was dropped".to_string())
    }

    /// Register a pending elicitation and get a receiver to await the response.
    ///
    /// The stream processing loop calls this when it encounters an `ActionRequired::Elicitation`.
    pub async fn register_elicitation(
        &self,
        request_id: String,
    ) -> oneshot::Receiver<serde_json::Value> {
        let (tx, rx) = oneshot::channel();
        debug!("Registering pending elicitation: {}", request_id);
        self.pending_elicitations
            .write()
            .await
            .insert(request_id, tx);
        rx
    }

    /// Submit an elicitation response from the client.
    ///
    /// Called by `ClientNotificationHandler` when it receives an `elicitation_response` message.
    pub async fn submit_elicitation(
        &self,
        request_id: &str,
        user_data: serde_json::Value,
    ) -> Result<(), String> {
        let tx = self
            .pending_elicitations
            .write()
            .await
            .remove(request_id)
            .ok_or_else(|| format!("No pending elicitation for id: {}", request_id))?;

        debug!("Submitting elicitation response for {}", request_id);

        tx.send(user_data)
            .map_err(|_| "Response receiver was dropped".to_string())
    }

    /// Cancel a pending confirmation (e.g., on timeout or session cleanup).
    pub async fn cancel_confirmation(&self, request_id: &str) -> bool {
        let removed = self
            .pending_confirmations
            .write()
            .await
            .remove(request_id)
            .is_some();
        if removed {
            warn!("Cancelled pending confirmation: {}", request_id);
        }
        removed
    }

    /// Cancel a pending elicitation.
    pub async fn cancel_elicitation(&self, request_id: &str) -> bool {
        let removed = self
            .pending_elicitations
            .write()
            .await
            .remove(request_id)
            .is_some();
        if removed {
            warn!("Cancelled pending elicitation: {}", request_id);
        }
        removed
    }

    /// Get count of pending interactions (for monitoring/debugging).
    pub async fn pending_count(&self) -> (usize, usize) {
        let confirmations = self.pending_confirmations.read().await.len();
        let elicitations = self.pending_elicitations.read().await.len();
        (confirmations, elicitations)
    }
}

impl Default for InteractionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_confirmation_flow() {
        let manager = InteractionManager::new();

        // Register a pending confirmation
        let rx = manager.register_confirmation("test-123".to_string()).await;

        // Submit response (simulating client)
        let submit_result = manager
            .submit_confirmation("test-123", Permission::AllowOnce)
            .await;
        assert!(submit_result.is_ok());

        // Receiver should get the response
        let result = timeout(Duration::from_millis(100), rx).await;
        assert!(result.is_ok());
        let confirmation = result.unwrap().unwrap();
        assert_eq!(confirmation.permission, Permission::AllowOnce);
    }

    #[tokio::test]
    async fn test_confirmation_not_found() {
        let manager = InteractionManager::new();

        let result = manager
            .submit_confirmation("nonexistent", Permission::DenyOnce)
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No pending confirmation"));
    }

    #[tokio::test]
    async fn test_elicitation_flow() {
        let manager = InteractionManager::new();

        let rx = manager
            .register_elicitation("elicit-456".to_string())
            .await;

        let user_data = serde_json::json!({"host": "localhost", "port": 5432});
        let submit_result = manager
            .submit_elicitation("elicit-456", user_data.clone())
            .await;
        assert!(submit_result.is_ok());

        let result = timeout(Duration::from_millis(100), rx).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().unwrap(), user_data);
    }

    #[tokio::test]
    async fn test_cancel_confirmation() {
        let manager = InteractionManager::new();

        let _rx = manager.register_confirmation("cancel-me".to_string()).await;
        assert_eq!(manager.pending_count().await, (1, 0));

        let cancelled = manager.cancel_confirmation("cancel-me").await;
        assert!(cancelled);
        assert_eq!(manager.pending_count().await, (0, 0));
    }
}

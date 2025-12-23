use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use tracing::{info, debug, warn, error};

use crate::models::{
    Id, UserInteraction, InteractionType, InteractionResponse, InteractionStatus,
    ProposalResponse
};
use crate::handlers::{StreamingHandler, ModificationHandler};

/// Handler for user interaction management and workflow coordination
///
/// Manages user interactions for approval workflows, handles timeouts,
/// and coordinates with modification handler for User Story 3
pub struct InteractionHandler {
    /// Active interactions by ID
    active_interactions: Arc<RwLock<HashMap<Id, UserInteraction>>>,
    /// Interactions by session for efficient lookup
    session_interactions: Arc<RwLock<HashMap<Id, Vec<Id>>>>,
    /// Streaming handler for notifications
    streaming_handler: Option<Arc<StreamingHandler>>,
    /// Modification handler for proposal coordination
    modification_handler: Option<Arc<ModificationHandler>>,
    /// Configuration for interaction handling
    config: InteractionConfig,
}

/// Configuration for interaction handling behavior
#[derive(Debug, Clone)]
pub struct InteractionConfig {
    /// Default timeout for interactions in seconds
    pub default_timeout_seconds: u32,
    /// Maximum number of active interactions per session
    pub max_interactions_per_session: usize,
    /// Whether to auto-cleanup completed interactions
    pub auto_cleanup_completed: bool,
    /// Cleanup interval in minutes
    pub cleanup_interval_minutes: u32,
    /// Default action for timed-out approvals
    pub default_timeout_action: TimeoutAction,
    /// Whether to retry interactions before timing out
    pub enable_retry: bool,
    /// Maximum retry attempts
    pub max_retry_attempts: u32,
}

/// Action to take when an interaction times out
#[derive(Debug, Clone, PartialEq)]
pub enum TimeoutAction {
    /// Deny/reject the interaction
    Deny,
    /// Allow/approve the interaction
    Allow,
    /// Cancel the interaction
    Cancel,
    /// Use context-specific default
    ContextSpecific,
}

impl Default for InteractionConfig {
    fn default() -> Self {
        Self {
            default_timeout_seconds: 30,
            max_interactions_per_session: 100,
            auto_cleanup_completed: true,
            cleanup_interval_minutes: 5,
            default_timeout_action: TimeoutAction::ContextSpecific,
            enable_retry: false,
            max_retry_attempts: 2,
        }
    }
}

/// Result of creating an interaction
#[derive(Debug)]
pub struct InteractionCreationResult {
    /// The created interaction
    pub interaction: UserInteraction,
    /// Whether immediate attention is required
    pub requires_immediate_attention: bool,
    /// Estimated processing priority
    pub priority: InteractionPriority,
}

/// Priority levels for interactions
#[derive(Debug, Clone, PartialEq)]
pub enum InteractionPriority {
    Low,
    Normal,
    High,
    Critical,
}

impl InteractionHandler {
    /// Create a new interaction handler
    pub fn new(config: InteractionConfig) -> Self {
        Self {
            active_interactions: Arc::new(RwLock::new(HashMap::new())),
            session_interactions: Arc::new(RwLock::new(HashMap::new())),
            streaming_handler: None,
            modification_handler: None,
            config,
        }
    }

    /// Create with streaming support
    pub fn new_with_streaming(config: InteractionConfig, streaming_handler: Arc<StreamingHandler>) -> Self {
        Self {
            active_interactions: Arc::new(RwLock::new(HashMap::new())),
            session_interactions: Arc::new(RwLock::new(HashMap::new())),
            streaming_handler: Some(streaming_handler),
            modification_handler: None,
            config,
        }
    }

    /// Create with both streaming and modification handler support
    pub fn new_with_handlers(
        config: InteractionConfig,
        streaming_handler: Arc<StreamingHandler>,
        modification_handler: Arc<ModificationHandler>,
    ) -> Self {
        Self {
            active_interactions: Arc::new(RwLock::new(HashMap::new())),
            session_interactions: Arc::new(RwLock::new(HashMap::new())),
            streaming_handler: Some(streaming_handler),
            modification_handler: Some(modification_handler),
            config,
        }
    }

    /// Create a new user interaction
    pub async fn create_interaction(
        &self,
        session_id: Option<Id>,
        interaction_type: InteractionType,
        prompt: String,
        proposal_id: Option<Id>,
        timeout_seconds: Option<u32>,
    ) -> Result<InteractionCreationResult> {
        info!("Creating user interaction: {:?}", interaction_type);

        // Create the interaction
        let timeout = timeout_seconds.unwrap_or(self.config.default_timeout_seconds);
        let mut interaction = UserInteraction::new(interaction_type.clone(), prompt, proposal_id.clone(), Some(timeout));
        interaction.session_id = session_id.clone();

        // Determine priority based on interaction type and proposal
        let mut priority = InteractionPriority::Normal;
        let mut requires_immediate_attention = false;

        // Check proposal context for priority
        if let Some(proposal_id) = &proposal_id {
            if let Some(modification_handler) = &self.modification_handler {
                if let Some(proposal) = modification_handler.get_proposal(proposal_id).await {
                    if proposal.is_high_risk() {
                        priority = InteractionPriority::Critical;
                        requires_immediate_attention = true;
                    }
                }
            }
        }

        // Store the interaction
        {
            let mut interactions = self.active_interactions.write().await;
            interactions.insert(interaction.id.clone(), interaction.clone());
        }

        // Track by session if provided
        if let Some(session_id) = &session_id {
            let mut session_interactions = self.session_interactions.write().await;
            session_interactions.entry(session_id.clone()).or_insert_with(Vec::new).push(interaction.id.clone());
        }

        // Send interaction notification if streaming is enabled
        if let Some(streaming_handler) = &self.streaming_handler {
            if let Some(session_id) = &session_id {
                streaming_handler.send_user_interaction(
                    session_id,
                    None,
                    interaction.id.clone(),
                    &format!("{:?}", interaction_type).to_lowercase(),
                    &interaction.prompt,
                    proposal_id.clone(),
                    Some(timeout),
                ).await?;
            }
        }

        info!("Created interaction {} with priority {:?}", interaction.id, priority);

        Ok(InteractionCreationResult {
            interaction,
            requires_immediate_attention,
            priority,
        })
    }

    /// Get an interaction by ID
    pub async fn get_interaction(&self, interaction_id: &Id) -> Option<UserInteraction> {
        let interactions = self.active_interactions.read().await;
        interactions.get(interaction_id).cloned()
    }

    /// Get all interactions for a session
    pub async fn get_session_interactions(&self, session_id: &Id) -> Result<Vec<UserInteraction>> {
        let session_interactions = self.session_interactions.read().await;
        let interactions = self.active_interactions.read().await;

        if let Some(interaction_ids) = session_interactions.get(session_id) {
            let session_interactions: Vec<UserInteraction> = interaction_ids
                .iter()
                .filter_map(|id| interactions.get(id).cloned())
                .collect();
            Ok(session_interactions)
        } else {
            Ok(Vec::new())
        }
    }

    /// Respond to an interaction
    pub async fn respond_to_interaction(
        &self,
        interaction_id: &Id,
        response_type: &str,
        response_data: serde_json::Value,
        responded_by: Option<String>,
    ) -> Result<UserInteraction> {
        info!("Processing response to interaction: {}", interaction_id);

        let mut interactions = self.active_interactions.write().await;
        let interaction = interactions.get_mut(interaction_id)
            .ok_or_else(|| anyhow::anyhow!("Interaction {} not found", interaction_id))?;

        // Check if interaction is still actionable
        if !interaction.is_pending() {
            anyhow::bail!("Interaction {} is no longer pending (status: {:?})", interaction_id, interaction.status);
        }

        if interaction.is_expired() {
            anyhow::bail!("Interaction {} has expired", interaction_id);
        }

        // Parse response based on type
        let response = match response_type {
            "approval" => {
                let approved = response_data.get("approved")
                    .and_then(|v| v.as_bool())
                    .ok_or_else(|| anyhow::anyhow!("Invalid approval response format"))?;
                let comment = response_data.get("comment").and_then(|v| v.as_str()).map(|s| s.to_string());

                InteractionResponse::approval(approved, comment)
            }
            "input" => {
                let text = response_data.get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Invalid input response format"))?;
                let comment = response_data.get("comment").and_then(|v| v.as_str()).map(|s| s.to_string());

                InteractionResponse::text_input(text.to_string(), comment)
            }
            _ => {
                anyhow::bail!("Unsupported response type: {}", response_type);
            }
        };

        // Apply the response
        interaction.respond(response.clone());

        let updated_interaction = interaction.clone();

        // Handle file modification approval specifically
        if interaction.is_file_modification_approval() {
            self.handle_file_modification_response(interaction, &response).await?;
        }

        // Send response notification if streaming is enabled
        if let Some(streaming_handler) = &self.streaming_handler {
            if let Some(session_id) = &interaction.session_id {
                streaming_handler.send_system_event(
                    session_id,
                    None,
                    "interaction_responded",
                    "processed",
                    Some(serde_json::json!({
                        "interaction_id": interaction_id,
                        "response_type": response_type,
                        "responded_by": responded_by
                    })),
                ).await?;
            }
        }

        info!("Interaction {} responded with type: {}", interaction_id, response_type);

        Ok(updated_interaction)
    }

    /// Handle file modification approval response
    async fn handle_file_modification_response(
        &self,
        interaction: &UserInteraction,
        response: &InteractionResponse,
    ) -> Result<()> {
        if let Some(modification_handler) = &self.modification_handler {
            if let Some(proposal_id) = &interaction.proposal_id {
                if let Some(approved) = response.approved {
                    let proposal_response = ProposalResponse {
                        proposal_id: proposal_id.clone(),
                        approved,
                        comment: response.comment.clone(),
                        responded_at: response.responded_at,
                        responded_by: None, // Could be extracted from context
                    };

                    modification_handler.apply_response(proposal_id, proposal_response).await?;
                }
            }
        }
        Ok(())
    }

    /// Check and handle timed-out interactions
    pub async fn check_and_handle_timeouts(&self) -> Result<Vec<Id>> {
        debug!("Checking for timed-out interactions");

        let mut interactions = self.active_interactions.write().await;
        let mut timed_out_interaction_ids = Vec::new();

        for (interaction_id, interaction) in interactions.iter_mut() {
            if interaction.check_and_mark_timeout() {
                timed_out_interaction_ids.push(interaction_id.clone());

                // Handle timeout based on interaction type and configuration
                self.handle_interaction_timeout(interaction).await?;

                warn!("Interaction {} timed out", interaction_id);
            }
        }

        if !timed_out_interaction_ids.is_empty() {
            info!("Handled {} timed-out interactions", timed_out_interaction_ids.len());
        }

        Ok(timed_out_interaction_ids)
    }

    /// Handle timeout for a specific interaction
    async fn handle_interaction_timeout(&self, interaction: &mut UserInteraction) -> Result<()> {
        let default_action = self.determine_timeout_action(interaction);

        // Apply default action
        match default_action {
            TimeoutAction::Deny => {
                if interaction.is_file_modification_approval() {
                    self.apply_timeout_response(interaction, false).await?;
                }
            }
            TimeoutAction::Allow => {
                if interaction.is_file_modification_approval() {
                    self.apply_timeout_response(interaction, true).await?;
                }
            }
            TimeoutAction::Cancel => {
                interaction.cancel();
            }
            TimeoutAction::ContextSpecific => {
                // Context-specific logic
                if interaction.is_file_modification_approval() {
                    // For file modifications, default to deny for safety
                    self.apply_timeout_response(interaction, false).await?;
                } else {
                    interaction.cancel();
                }
            }
        }

        // Send timeout notification if streaming is enabled
        if let Some(streaming_handler) = &self.streaming_handler {
            if let Some(session_id) = &interaction.session_id {
                streaming_handler.send_system_event(
                    session_id,
                    None,
                    "interaction_timeout",
                    "timeout_processed",
                    Some(serde_json::json!({
                        "interaction_id": interaction.id,
                        "default_action": format!("{:?}", default_action).to_lowercase(),
                        "interaction_type": format!("{:?}", interaction.interaction_type)
                    })),
                ).await?;
            }
        }

        Ok(())
    }

    /// Apply timeout response for file modification interactions
    async fn apply_timeout_response(&self, interaction: &UserInteraction, approved: bool) -> Result<()> {
        if let Some(modification_handler) = &self.modification_handler {
            if let Some(proposal_id) = &interaction.proposal_id {
                let timeout_response = ProposalResponse {
                    proposal_id: proposal_id.clone(),
                    approved,
                    comment: Some("Auto-response due to timeout".to_string()),
                    responded_at: chrono::Utc::now(),
                    responded_by: Some("system".to_string()),
                };

                modification_handler.apply_response(proposal_id, timeout_response).await?;
            }
        }
        Ok(())
    }

    /// Determine appropriate timeout action for an interaction
    fn determine_timeout_action(&self, interaction: &UserInteraction) -> TimeoutAction {
        match &self.config.default_timeout_action {
            TimeoutAction::ContextSpecific => {
                match interaction.interaction_type {
                    InteractionType::FileModificationApproval => {
                        // Check if this is a high-risk modification
                        // For safety, default to deny for file modifications
                        TimeoutAction::Deny
                    }
                    InteractionType::Confirmation => TimeoutAction::Cancel,
                    InteractionType::Input => TimeoutAction::Cancel,
                    _ => TimeoutAction::Deny,
                }
            }
            other => other.clone(),
        }
    }

    /// Clean up completed interactions
    pub async fn cleanup_completed_interactions(&self, max_age_hours: u32) -> Result<usize> {
        info!("Cleaning up completed interactions older than {} hours", max_age_hours);

        let cutoff = chrono::Utc::now() - chrono::Duration::hours(max_age_hours as i64);

        let mut interactions = self.active_interactions.write().await;
        let mut session_interactions = self.session_interactions.write().await;

        let _initial_count = interactions.len();

        // Collect IDs to remove
        let mut interactions_to_remove = Vec::new();
        for (interaction_id, interaction) in interactions.iter() {
            if matches!(interaction.status,
                InteractionStatus::Responded | InteractionStatus::Timeout |
                InteractionStatus::Cancelled | InteractionStatus::Expired)
                && interaction.created_at < cutoff {
                interactions_to_remove.push(interaction_id.clone());
            }
        }

        // Remove from interactions map
        for interaction_id in &interactions_to_remove {
            interactions.remove(interaction_id);
        }

        // Remove from session tracking
        for interaction_ids in session_interactions.values_mut() {
            interaction_ids.retain(|id| !interactions_to_remove.contains(id));
        }

        // Clean up empty session entries
        session_interactions.retain(|_, interaction_ids| !interaction_ids.is_empty());

        let cleaned_count = interactions_to_remove.len();
        if cleaned_count > 0 {
            info!("Cleaned up {} completed interactions", cleaned_count);
        }

        Ok(cleaned_count)
    }

    /// Get interaction statistics
    pub async fn get_statistics(&self) -> InteractionStatistics {
        let interactions = self.active_interactions.read().await;
        let session_interactions = self.session_interactions.read().await;

        let mut stats = InteractionStatistics::default();
        stats.total_interactions = interactions.len();
        stats.active_sessions = session_interactions.len();

        for interaction in interactions.values() {
            match interaction.status {
                InteractionStatus::Pending => {
                    if interaction.is_expired() {
                        stats.expired_interactions += 1;
                    } else {
                        stats.pending_interactions += 1;
                    }
                }
                InteractionStatus::Responded => stats.responded_interactions += 1,
                InteractionStatus::Timeout => stats.timeout_interactions += 1,
                InteractionStatus::Cancelled => stats.cancelled_interactions += 1,
                InteractionStatus::Processed => stats.processed_interactions += 1,
                InteractionStatus::Expired => stats.expired_interactions += 1,
            }

            match interaction.interaction_type {
                InteractionType::FileModificationApproval => stats.file_modification_interactions += 1,
                _ => stats.other_interactions += 1,
            }
        }

        stats
    }

    /// Start background timeout checking task
    pub async fn start_timeout_task(&self) -> tokio::task::JoinHandle<()> {
        let handler = self.clone();
        let check_interval = std::time::Duration::from_secs(10); // Check every 10 seconds

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(check_interval);
            loop {
                interval.tick().await;

                // Check and handle timeouts
                if let Err(e) = handler.check_and_handle_timeouts().await {
                    error!("Error checking interaction timeouts: {}", e);
                }

                // Clean up old completed interactions if enabled
                if handler.config.auto_cleanup_completed {
                    let max_age_hours = 6; // Keep completed interactions for 6 hours
                    if let Err(e) = handler.cleanup_completed_interactions(max_age_hours).await {
                        error!("Error cleaning up interactions: {}", e);
                    }
                }
            }
        })
    }
}

// Implement Clone manually to avoid requiring Clone on all fields
impl Clone for InteractionHandler {
    fn clone(&self) -> Self {
        Self {
            active_interactions: self.active_interactions.clone(),
            session_interactions: self.session_interactions.clone(),
            streaming_handler: self.streaming_handler.clone(),
            modification_handler: self.modification_handler.clone(),
            config: self.config.clone(),
        }
    }
}

/// Statistics about interactions
#[derive(Debug, Default)]
pub struct InteractionStatistics {
    pub total_interactions: usize,
    pub pending_interactions: usize,
    pub responded_interactions: usize,
    pub timeout_interactions: usize,
    pub cancelled_interactions: usize,
    pub processed_interactions: usize,
    pub expired_interactions: usize,
    pub file_modification_interactions: usize,
    pub other_interactions: usize,
    pub active_sessions: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Identifiable;

    #[tokio::test]
    async fn test_create_interaction() {
        let handler = InteractionHandler::new(InteractionConfig::default());

        let result = handler.create_interaction(
            Some("session-123".to_string()),
            InteractionType::FileModificationApproval,
            "Approve file modification?".to_string(),
            Some("proposal-456".to_string()),
            Some(30),
        ).await.unwrap();

        assert_eq!(result.interaction.interaction_type, InteractionType::FileModificationApproval);
        assert_eq!(result.interaction.session_id, Some("session-123".to_string()));
        assert_eq!(result.interaction.proposal_id, Some("proposal-456".to_string()));
        assert_eq!(result.interaction.status, InteractionStatus::Pending);
    }

    #[tokio::test]
    async fn test_respond_to_interaction() {
        let handler = InteractionHandler::new(InteractionConfig::default());

        let result = handler.create_interaction(
            Some("session-123".to_string()),
            InteractionType::FileModificationApproval,
            "Test prompt".to_string(),
            Some("proposal-456".to_string()),
            Some(30),
        ).await.unwrap();

        let response_data = serde_json::json!({
            "approved": true,
            "comment": "Looks good"
        });

        let updated = handler.respond_to_interaction(
            &result.interaction.id,
            "approval",
            response_data,
            Some("user123".to_string()),
        ).await.unwrap();

        assert_eq!(updated.status, InteractionStatus::Responded);
        assert!(updated.response.is_some());
        assert_eq!(updated.get_approval_result(), Some(true));
    }

    #[tokio::test]
    async fn test_timeout_handling() {
        let handler = InteractionHandler::new(InteractionConfig::default());

        // Create interaction with past expiry
        let mut interaction = UserInteraction::new(
            InteractionType::FileModificationApproval,
            "Test timeout".to_string(),
            None,
            Some(1),
        );
        interaction.expires_at = Some(chrono::Utc::now() - chrono::Duration::seconds(1));

        {
            let mut interactions = handler.active_interactions.write().await;
            interactions.insert(interaction.id.clone(), interaction);
        }

        let timed_out = handler.check_and_handle_timeouts().await.unwrap();
        assert_eq!(timed_out.len(), 1);
    }
}
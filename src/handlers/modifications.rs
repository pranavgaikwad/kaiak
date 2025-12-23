use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use tracing::{info, debug, warn, error};

use crate::models::{
    Id, FileModificationProposal, ProposalResponse, ApprovalStatus, ModificationType,
    UserInteraction
};
use crate::handlers::StreamingHandler;

/// Handler for file modification proposal logic and lifecycle management
///
/// Manages the creation, storage, and lifecycle of file modification proposals
/// for User Story 3: Interactive File Modification Approval
pub struct ModificationHandler {
    /// Active proposals by ID
    active_proposals: Arc<RwLock<HashMap<Id, FileModificationProposal>>>,
    /// Proposals by session for efficient lookup
    session_proposals: Arc<RwLock<HashMap<Id, Vec<Id>>>>,
    /// Streaming handler for notifications
    streaming_handler: Option<Arc<StreamingHandler>>,
    /// Configuration for proposal management
    config: ModificationConfig,
}

/// Configuration for modification handling behavior
#[derive(Debug, Clone)]
pub struct ModificationConfig {
    /// Default timeout for proposals in minutes
    pub default_timeout_minutes: u32,
    /// Maximum number of active proposals per session
    pub max_proposals_per_session: usize,
    /// Whether to auto-cleanup expired proposals
    pub auto_cleanup_expired: bool,
    /// Cleanup interval in minutes
    pub cleanup_interval_minutes: u32,
}

impl Default for ModificationConfig {
    fn default() -> Self {
        Self {
            default_timeout_minutes: 5,
            max_proposals_per_session: 50,
            auto_cleanup_expired: true,
            cleanup_interval_minutes: 1,
        }
    }
}

/// Result of creating a modification proposal
#[derive(Debug, Clone)]
pub struct ProposalCreationResult {
    pub proposal: FileModificationProposal,
    /// User interaction that was created for approval
    pub interaction: UserInteraction,
    pub requires_immediate_attention: bool,
}

impl ModificationHandler {
    /// Create a new modification handler
    pub fn new(config: ModificationConfig) -> Self {
        Self {
            active_proposals: Arc::new(RwLock::new(HashMap::new())),
            session_proposals: Arc::new(RwLock::new(HashMap::new())),
            streaming_handler: None,
            config,
        }
    }

    /// Create with streaming support
    pub fn new_with_streaming(config: ModificationConfig, streaming_handler: Arc<StreamingHandler>) -> Self {
        Self {
            active_proposals: Arc::new(RwLock::new(HashMap::new())),
            session_proposals: Arc::new(RwLock::new(HashMap::new())),
            streaming_handler: Some(streaming_handler),
            config,
        }
    }

    /// Create a new file modification proposal
    pub async fn create_proposal(
        &self,
        session_id: Option<Id>,
        file_path: String,
        modification_type: String,
        original_content: String,
        proposed_content: String,
        description: String,
        line_range: Option<(u32, u32)>,
    ) -> Result<ProposalCreationResult> {
        info!("Creating file modification proposal for: {}", file_path);

        // Create the proposal
        let mut proposal = FileModificationProposal::new(
            file_path.clone(),
            modification_type.clone(),
            original_content,
            proposed_content,
            description.clone(),
        );

        // Set session and line range if provided
        proposal.session_id = session_id.clone();
        proposal.line_range = line_range;

        // Set expiry based on modification type
        let timeout_minutes = self.get_timeout_for_modification_type(&modification_type);
        proposal.expires_at = Some(proposal.created_at + chrono::Duration::minutes(timeout_minutes as i64));

        // Check if this is a high-risk modification
        let requires_immediate_attention = proposal.is_high_risk();

        // Create user interaction for approval
        let interaction_prompt = format!(
            "File modification proposed for {}: {}",
            file_path, description
        );

        let interaction = UserInteraction::new_file_modification_approval(
            interaction_prompt,
            proposal.id.clone(),
            timeout_minutes * 60, // Convert to seconds
        );

        // Store the proposal
        {
            let mut proposals = self.active_proposals.write().await;
            proposals.insert(proposal.id.clone(), proposal.clone());
        }

        // Track by session if provided
        if let Some(ref session_id) = session_id {
            let mut session_proposals = self.session_proposals.write().await;
            session_proposals.entry(session_id.clone()).or_insert_with(Vec::new).push(proposal.id.clone());
        }

        // Send notification if streaming is enabled
        if let Some(streaming_handler) = &self.streaming_handler {
            if let Some(session_id) = &session_id {
                streaming_handler.send_system_event(
                    session_id,
                    None,
                    "proposal_created",
                    "pending",
                    Some(serde_json::json!({
                        "proposal_id": proposal.id,
                        "file_path": file_path,
                        "modification_type": modification_type,
                        "requires_immediate_attention": requires_immediate_attention,
                        "expires_at": proposal.expires_at
                    })),
                ).await?;
            }
        }

        info!("Created proposal {} for file: {}", proposal.id, file_path);

        Ok(ProposalCreationResult {
            proposal,
            interaction,
            requires_immediate_attention,
        })
    }

    /// Get a proposal by ID
    pub async fn get_proposal(&self, proposal_id: &Id) -> Option<FileModificationProposal> {
        let proposals = self.active_proposals.read().await;
        proposals.get(proposal_id).cloned()
    }

    /// Get all proposals for a session
    pub async fn get_session_proposals(&self, session_id: &Id) -> Result<Vec<FileModificationProposal>> {
        let session_proposals = self.session_proposals.read().await;
        let proposals = self.active_proposals.read().await;

        if let Some(proposal_ids) = session_proposals.get(session_id) {
            let session_proposals: Vec<FileModificationProposal> = proposal_ids
                .iter()
                .filter_map(|id| proposals.get(id).cloned())
                .collect();
            Ok(session_proposals)
        } else {
            Ok(Vec::new())
        }
    }

    /// Apply a response to a proposal
    pub async fn apply_response(
        &self,
        proposal_id: &Id,
        response: ProposalResponse,
    ) -> Result<FileModificationProposal> {
        info!("Applying response to proposal: {}", proposal_id);

        let mut proposals = self.active_proposals.write().await;
        let proposal = proposals.get_mut(proposal_id)
            .ok_or_else(|| anyhow::anyhow!("Proposal {} not found", proposal_id))?;

        // Check if proposal is still actionable
        if proposal.approval_status != ApprovalStatus::Pending {
            anyhow::bail!("Proposal {} is no longer pending (status: {:?})", proposal_id, proposal.approval_status);
        }

        if proposal.is_expired() {
            anyhow::bail!("Proposal {} has expired", proposal_id);
        }

        // Apply the response
        if response.approved {
            proposal.approve();
        } else {
            proposal.reject();
        }

        let updated_proposal = proposal.clone();

        // Send notification if streaming is enabled
        if let Some(streaming_handler) = &self.streaming_handler {
            if let Some(session_id) = &proposal.session_id {
                streaming_handler.send_system_event(
                    session_id,
                    None,
                    "proposal_responded",
                    if response.approved { "approved" } else { "rejected" },
                    Some(serde_json::json!({
                        "proposal_id": proposal_id,
                        "approved": response.approved,
                        "comment": response.comment,
                        "responded_by": response.responded_by
                    })),
                ).await?;
            }
        }

        info!(
            "Proposal {} {}",
            proposal_id,
            if response.approved { "approved" } else { "rejected" }
        );

        Ok(updated_proposal)
    }

    /// Mark a proposal as applied (after successful file modification)
    pub async fn mark_applied(&self, proposal_id: &Id) -> Result<()> {
        info!("Marking proposal as applied: {}", proposal_id);

        let mut proposals = self.active_proposals.write().await;
        let proposal = proposals.get_mut(proposal_id)
            .ok_or_else(|| anyhow::anyhow!("Proposal {} not found", proposal_id))?;

        if proposal.approval_status != ApprovalStatus::Approved {
            anyhow::bail!("Cannot mark non-approved proposal as applied");
        }

        proposal.approval_status = ApprovalStatus::Applied;

        // Send notification if streaming is enabled
        if let Some(streaming_handler) = &self.streaming_handler {
            if let Some(session_id) = &proposal.session_id {
                streaming_handler.send_system_event(
                    session_id,
                    None,
                    "proposal_applied",
                    "completed",
                    Some(serde_json::json!({
                        "proposal_id": proposal_id,
                        "file_path": proposal.file_path
                    })),
                ).await?;
            }
        }

        info!("Proposal {} marked as applied", proposal_id);
        Ok(())
    }

    /// Check and mark expired proposals
    pub async fn check_and_mark_expired(&self) -> Result<Vec<Id>> {
        debug!("Checking for expired proposals");

        let mut proposals = self.active_proposals.write().await;
        let mut expired_proposal_ids = Vec::new();

        for (proposal_id, proposal) in proposals.iter_mut() {
            if proposal.approval_status == ApprovalStatus::Pending && proposal.is_expired() {
                proposal.expire();
                expired_proposal_ids.push(proposal_id.clone());

                // Send notification if streaming is enabled
                if let Some(streaming_handler) = &self.streaming_handler {
                    if let Some(session_id) = &proposal.session_id {
                        let _ = streaming_handler.send_system_event(
                            session_id,
                            None,
                            "proposal_expired",
                            "expired",
                            Some(serde_json::json!({
                                "proposal_id": proposal_id,
                                "file_path": proposal.file_path
                            })),
                        ).await;
                    }
                }

                warn!("Proposal {} expired", proposal_id);
            }
        }

        if !expired_proposal_ids.is_empty() {
            info!("Marked {} proposals as expired", expired_proposal_ids.len());
        }

        Ok(expired_proposal_ids)
    }

    /// Clean up old completed proposals
    pub async fn cleanup_completed_proposals(&self, max_age_hours: u32) -> Result<usize> {
        info!("Cleaning up completed proposals older than {} hours", max_age_hours);

        let cutoff = chrono::Utc::now() - chrono::Duration::hours(max_age_hours as i64);

        let mut proposals = self.active_proposals.write().await;
        let mut session_proposals = self.session_proposals.write().await;

        let _initial_count = proposals.len();

        // Collect IDs to remove
        let mut proposals_to_remove = Vec::new();
        for (proposal_id, proposal) in proposals.iter() {
            if matches!(proposal.approval_status,
                ApprovalStatus::Applied | ApprovalStatus::Expired | ApprovalStatus::Rejected)
                && proposal.created_at < cutoff {
                proposals_to_remove.push(proposal_id.clone());
            }
        }

        // Remove from proposals map
        for proposal_id in &proposals_to_remove {
            proposals.remove(proposal_id);
        }

        // Remove from session tracking
        for proposal_ids in session_proposals.values_mut() {
            proposal_ids.retain(|id| !proposals_to_remove.contains(id));
        }

        // Clean up empty session entries
        session_proposals.retain(|_, proposal_ids| !proposal_ids.is_empty());

        let cleaned_count = proposals_to_remove.len();
        if cleaned_count > 0 {
            info!("Cleaned up {} completed proposals", cleaned_count);
        }

        Ok(cleaned_count)
    }

    /// Get proposal statistics
    pub async fn get_statistics(&self) -> ProposalStatistics {
        let proposals = self.active_proposals.read().await;
        let session_proposals = self.session_proposals.read().await;

        let mut stats = ProposalStatistics::default();
        stats.total_proposals = proposals.len();
        stats.active_sessions = session_proposals.len();

        for proposal in proposals.values() {
            match proposal.approval_status {
                ApprovalStatus::Pending => {
                    if proposal.is_expired() {
                        stats.expired_proposals += 1;
                    } else {
                        stats.pending_proposals += 1;
                    }
                }
                ApprovalStatus::Approved => stats.approved_proposals += 1,
                ApprovalStatus::Rejected => stats.rejected_proposals += 1,
                ApprovalStatus::Applied => stats.applied_proposals += 1,
                ApprovalStatus::Expired => stats.expired_proposals += 1,
                ApprovalStatus::Error => stats.error_proposals += 1,
            }

            if proposal.is_high_risk() {
                stats.high_risk_proposals += 1;
            }
        }

        stats
    }

    /// Get timeout for modification type
    fn get_timeout_for_modification_type(&self, modification_type: &str) -> u32 {
        // Try to parse as structured modification type
        if let Ok(mod_type) = serde_json::from_value::<ModificationType>(
            serde_json::Value::String(modification_type.to_string())
        ) {
            mod_type.default_timeout_seconds() / 60 // Convert to minutes
        } else {
            // Fall back to default timeout
            self.config.default_timeout_minutes
        }
    }

    /// Start background cleanup task
    pub async fn start_cleanup_task(&self) -> tokio::task::JoinHandle<()> {
        let handler = self.clone();
        let cleanup_interval = std::time::Duration::from_secs(
            self.config.cleanup_interval_minutes as u64 * 60
        );

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);
            loop {
                interval.tick().await;

                // Check and mark expired proposals
                if let Err(e) = handler.check_and_mark_expired().await {
                    error!("Error checking expired proposals: {}", e);
                }

                // Clean up old completed proposals if enabled
                if handler.config.auto_cleanup_expired {
                    let max_age_hours = 24; // Keep completed proposals for 24 hours
                    if let Err(e) = handler.cleanup_completed_proposals(max_age_hours).await {
                        error!("Error cleaning up proposals: {}", e);
                    }
                }
            }
        })
    }
}

// Implement Clone manually to avoid requiring Clone on all fields
impl Clone for ModificationHandler {
    fn clone(&self) -> Self {
        Self {
            active_proposals: self.active_proposals.clone(),
            session_proposals: self.session_proposals.clone(),
            streaming_handler: self.streaming_handler.clone(),
            config: self.config.clone(),
        }
    }
}

/// Statistics about proposals
#[derive(Debug, Default)]
pub struct ProposalStatistics {
    pub total_proposals: usize,
    pub pending_proposals: usize,
    pub approved_proposals: usize,
    pub rejected_proposals: usize,
    pub applied_proposals: usize,
    pub expired_proposals: usize,
    pub error_proposals: usize,
    pub high_risk_proposals: usize,
    pub active_sessions: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Identifiable;

    #[tokio::test]
    async fn test_create_proposal() {
        let handler = ModificationHandler::new(ModificationConfig::default());

        let result = handler.create_proposal(
            Some("session-123".to_string()),
            "src/test.rs".to_string(),
            "content_replace".to_string(),
            "fn old() {}".to_string(),
            "fn new() {}".to_string(),
            "Update function".to_string(),
            Some((10, 12)),
        ).await.unwrap();

        assert_eq!(result.proposal.file_path, "src/test.rs");
        assert_eq!(result.proposal.modification_type, "content_replace");
        assert_eq!(result.proposal.session_id, Some("session-123".to_string()));
        assert_eq!(result.proposal.line_range, Some((10, 12)));
        assert_eq!(result.proposal.approval_status, ApprovalStatus::Pending);
    }

    #[tokio::test]
    async fn test_apply_response() {
        let handler = ModificationHandler::new(ModificationConfig::default());

        let result = handler.create_proposal(
            Some("session-123".to_string()),
            "src/test.rs".to_string(),
            "content_replace".to_string(),
            "old".to_string(),
            "new".to_string(),
            "test".to_string(),
            None,
        ).await.unwrap();

        let response = ProposalResponse::approve(
            result.proposal.id.clone(),
            Some("LGTM".to_string()),
        );

        let updated = handler.apply_response(&result.proposal.id, response).await.unwrap();
        assert_eq!(updated.approval_status, ApprovalStatus::Approved);
        assert!(updated.approved_at.is_some());
    }

    #[tokio::test]
    async fn test_expire_proposals() {
        let handler = ModificationHandler::new(ModificationConfig::default());

        // Create a proposal with past expiry
        let mut proposal = FileModificationProposal::new(
            "test.rs".to_string(),
            "content_replace".to_string(),
            "old".to_string(),
            "new".to_string(),
            "test".to_string(),
        );
        proposal.expires_at = Some(chrono::Utc::now() - chrono::Duration::minutes(1));

        {
            let mut proposals = handler.active_proposals.write().await;
            proposals.insert(proposal.id.clone(), proposal);
        }

        let expired = handler.check_and_mark_expired().await.unwrap();
        assert_eq!(expired.len(), 1);
    }
}
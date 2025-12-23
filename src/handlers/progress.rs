use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

use crate::models::{Id, StreamMessage, MessageType, MessageContent};
use tracing::{debug, warn};

/// Progress tracking state for individual operations
#[derive(Debug, Clone)]
pub struct ProgressState {
    pub operation_id: Id,
    pub session_id: Id,
    pub request_id: Option<Id>,
    pub current_phase: String,
    pub percentage: u8,
    pub description: String,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub estimated_completion: Option<DateTime<Utc>>,
}

/// Progress tracker for managing operation progress and streaming updates
pub struct ProgressTracker {
    progress_states: Arc<RwLock<HashMap<Id, ProgressState>>>,
    message_sender: Option<mpsc::UnboundedSender<StreamMessage>>,
}

impl ProgressTracker {
    pub fn new() -> Self {
        Self {
            progress_states: Arc::new(RwLock::new(HashMap::new())),
            message_sender: None,
        }
    }

    /// Create a new progress tracker with streaming capability
    pub fn with_streaming(sender: mpsc::UnboundedSender<StreamMessage>) -> Self {
        Self {
            progress_states: Arc::new(RwLock::new(HashMap::new())),
            message_sender: Some(sender),
        }
    }

    /// Start tracking progress for a new operation
    pub async fn start_operation(
        &self,
        operation_id: Id,
        session_id: Id,
        request_id: Option<Id>,
        initial_description: &str,
    ) -> Result<()> {
        let now = Utc::now();
        let progress_state = ProgressState {
            operation_id: operation_id.clone(),
            session_id: session_id.clone(),
            request_id: request_id.clone(),
            current_phase: "initializing".to_string(),
            percentage: 0,
            description: initial_description.to_string(),
            started_at: now,
            updated_at: now,
            estimated_completion: None,
        };

        // Store progress state
        {
            let mut states = self.progress_states.write().await;
            states.insert(operation_id.clone(), progress_state.clone());
        }

        // Send initial progress message
        self.send_progress_update(&progress_state).await?;

        debug!("Started progress tracking for operation: {}", operation_id);
        Ok(())
    }

    /// Update progress for an existing operation
    pub async fn update_progress(
        &self,
        operation_id: &Id,
        phase: &str,
        percentage: u8,
        description: &str,
    ) -> Result<()> {
        self.update_progress_with_estimate(operation_id, phase, percentage, description, None).await
    }

    /// Update progress with estimated completion time
    pub async fn update_progress_with_estimate(
        &self,
        operation_id: &Id,
        phase: &str,
        percentage: u8,
        description: &str,
        estimated_completion: Option<DateTime<Utc>>,
    ) -> Result<()> {
        let percentage = percentage.min(100); // Ensure percentage doesn't exceed 100

        let updated_state = {
            let mut states = self.progress_states.write().await;

            if let Some(state) = states.get_mut(operation_id) {
                state.current_phase = phase.to_string();
                state.percentage = percentage;
                state.description = description.to_string();
                state.updated_at = Utc::now();
                state.estimated_completion = estimated_completion;

                state.clone()
            } else {
                warn!("Progress update for unknown operation: {}", operation_id);
                return Ok(()); // Silently ignore unknown operations
            }
        };

        // Send progress update message
        self.send_progress_update(&updated_state).await?;

        debug!("Updated progress for operation {}: {}% - {}", operation_id, percentage, phase);
        Ok(())
    }

    /// Complete an operation and clean up tracking
    pub async fn complete_operation(&self, operation_id: &Id, final_message: &str) -> Result<()> {
        let completed_state = {
            let mut states = self.progress_states.write().await;

            if let Some(mut state) = states.remove(operation_id) {
                state.current_phase = "completed".to_string();
                state.percentage = 100;
                state.description = final_message.to_string();
                state.updated_at = Utc::now();
                Some(state)
            } else {
                None
            }
        };

        if let Some(state) = completed_state {
            // Send final progress message
            self.send_progress_update(&state).await?;
            debug!("Completed operation: {}", operation_id);
        }

        Ok(())
    }

    /// Mark an operation as failed and clean up tracking
    pub async fn fail_operation(&self, operation_id: &Id, error_message: &str) -> Result<()> {
        let failed_state = {
            let mut states = self.progress_states.write().await;

            if let Some(mut state) = states.remove(operation_id) {
                state.current_phase = "failed".to_string();
                state.description = format!("Failed: {}", error_message);
                state.updated_at = Utc::now();
                Some(state)
            } else {
                None
            }
        };

        if let Some(state) = failed_state {
            // Send failure progress message
            self.send_progress_update(&state).await?;
            debug!("Failed operation: {} - {}", operation_id, error_message);
        }

        Ok(())
    }

    /// Get current progress state for an operation
    pub async fn get_progress(&self, operation_id: &Id) -> Option<ProgressState> {
        let states = self.progress_states.read().await;
        states.get(operation_id).cloned()
    }

    /// Get all active operations
    pub async fn get_active_operations(&self) -> Vec<ProgressState> {
        let states = self.progress_states.read().await;
        states.values().cloned().collect()
    }

    /// Clean up old completed operations (for memory management)
    pub async fn cleanup_old_operations(&self, max_age_seconds: i64) -> usize {
        let cutoff = Utc::now() - chrono::Duration::seconds(max_age_seconds);
        let mut states = self.progress_states.write().await;

        let initial_count = states.len();
        states.retain(|_, state| {
            // Keep operations that are still recent or not completed/failed
            state.updated_at > cutoff ||
            (!state.current_phase.eq("completed") && !state.current_phase.eq("failed"))
        });

        let removed_count = initial_count - states.len();
        if removed_count > 0 {
            debug!("Cleaned up {} old progress operations", removed_count);
        }

        removed_count
    }

    /// Send progress update message through the configured sender
    async fn send_progress_update(&self, state: &ProgressState) -> Result<()> {
        if let Some(sender) = &self.message_sender {
            let progress_message = StreamMessage::new(
                state.session_id.clone(),
                state.request_id.clone(),
                MessageType::Progress,
                MessageContent::Progress {
                    percentage: state.percentage,
                    phase: state.current_phase.clone(),
                    description: state.description.clone(),
                },
            );

            if let Err(_) = sender.send(progress_message) {
                warn!("Failed to send progress message - receiver may be closed");
            }
        }

        Ok(())
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility for creating standardized progress phases
pub struct ProgressPhases;

impl ProgressPhases {
    // Standard phases for fix generation workflow
    pub const INITIALIZING: &'static str = "initializing";
    pub const ANALYZING_INCIDENTS: &'static str = "analyzing_incidents";
    pub const GENERATING_CONTEXT: &'static str = "generating_context";
    pub const CALLING_AI_AGENT: &'static str = "calling_ai_agent";
    pub const PROCESSING_RESPONSE: &'static str = "processing_response";
    pub const GENERATING_FIXES: &'static str = "generating_fixes";
    pub const VALIDATING_FIXES: &'static str = "validating_fixes";
    pub const COMPLETED: &'static str = "completed";
    pub const FAILED: &'static str = "failed";

    /// Get typical percentage for each phase
    pub fn get_typical_percentage(phase: &str) -> u8 {
        match phase {
            Self::INITIALIZING => 5,
            Self::ANALYZING_INCIDENTS => 15,
            Self::GENERATING_CONTEXT => 25,
            Self::CALLING_AI_AGENT => 40,
            Self::PROCESSING_RESPONSE => 60,
            Self::GENERATING_FIXES => 80,
            Self::VALIDATING_FIXES => 95,
            Self::COMPLETED => 100,
            _ => 50, // Default for unknown phases
        }
    }

    /// Get descriptive message for each phase
    pub fn get_phase_description(phase: &str) -> &'static str {
        match phase {
            Self::INITIALIZING => "Initializing fix generation process",
            Self::ANALYZING_INCIDENTS => "Analyzing code incidents and gathering context",
            Self::GENERATING_CONTEXT => "Generating contextual information for AI agent",
            Self::CALLING_AI_AGENT => "Requesting fix suggestions from AI agent",
            Self::PROCESSING_RESPONSE => "Processing AI agent response and recommendations",
            Self::GENERATING_FIXES => "Generating specific fix implementations",
            Self::VALIDATING_FIXES => "Validating and finalizing fix proposals",
            Self::COMPLETED => "Fix generation completed successfully",
            Self::FAILED => "Fix generation process failed",
            _ => "Processing request",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_progress_tracking_lifecycle() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let tracker = ProgressTracker::with_streaming(tx);

        let operation_id = uuid::Uuid::new_v4().to_string();
        let session_id = uuid::Uuid::new_v4().to_string();

        // Start operation
        tracker.start_operation(
            operation_id.clone(),
            session_id.clone(),
            None,
            "Test operation"
        ).await.unwrap();

        // Should receive initial progress message
        let initial_message = rx.try_recv().unwrap();
        assert_eq!(initial_message.session_id, session_id);

        // Update progress
        tracker.update_progress(
            &operation_id,
            "processing",
            50,
            "Halfway done"
        ).await.unwrap();

        let update_message = rx.try_recv().unwrap();
        if let MessageContent::Progress { percentage, phase, .. } = update_message.content {
            assert_eq!(percentage, 50);
            assert_eq!(phase, "processing");
        } else {
            panic!("Expected progress content");
        }

        // Complete operation
        tracker.complete_operation(&operation_id, "All done").await.unwrap();

        let completion_message = rx.try_recv().unwrap();
        if let MessageContent::Progress { percentage, phase, .. } = completion_message.content {
            assert_eq!(percentage, 100);
            assert_eq!(phase, "completed");
        } else {
            panic!("Expected progress content");
        }

        // Operation should be removed from tracking
        assert!(tracker.get_progress(&operation_id).await.is_none());
    }

    #[test]
    fn test_progress_phases_utilities() {
        assert_eq!(ProgressPhases::get_typical_percentage(ProgressPhases::INITIALIZING), 5);
        assert_eq!(ProgressPhases::get_typical_percentage(ProgressPhases::COMPLETED), 100);

        assert!(ProgressPhases::get_phase_description(ProgressPhases::ANALYZING_INCIDENTS)
            .contains("Analyzing"));
        assert!(ProgressPhases::get_phase_description(ProgressPhases::COMPLETED)
            .contains("completed"));
    }
}
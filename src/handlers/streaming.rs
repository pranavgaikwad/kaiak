use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use std::collections::HashMap;

use crate::models::{Id, StreamMessage, MessageType, MessageContent};
use crate::handlers::ProgressTracker;
use tracing::{debug, error, info, warn};

/// Configuration for streaming behavior
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    /// Buffer size for streaming messages
    pub buffer_size: usize,
    /// Maximum messages per second to prevent overwhelming clients
    pub max_messages_per_second: u32,
    /// Whether to batch multiple messages together
    pub enable_batching: bool,
    /// Timeout for message delivery in milliseconds
    pub delivery_timeout_ms: u32,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            buffer_size: 1000,
            max_messages_per_second: 50,
            enable_batching: false,
            delivery_timeout_ms: 1000,
        }
    }
}

/// Handles coordinated streaming of AI messages, progress updates, and tool calls
pub struct StreamingHandler {
    config: StreamingConfig,
    /// Active streaming channels by session ID
    active_streams: Arc<RwLock<HashMap<Id, StreamingSession>>>,
    /// Progress tracker for coordinating progress updates
    progress_tracker: Arc<ProgressTracker>,
}

/// Represents an active streaming session
struct StreamingSession {
    session_id: Id,
    sender: mpsc::UnboundedSender<StreamMessage>,
    created_at: chrono::DateTime<chrono::Utc>,
    message_count: u64,
}

impl StreamingHandler {
    pub fn new(config: StreamingConfig) -> Self {
        Self {
            config,
            active_streams: Arc::new(RwLock::new(HashMap::new())),
            progress_tracker: Arc::new(ProgressTracker::new()),
        }
    }

    /// Create a new streaming session for a given session ID
    pub async fn create_stream(&self, session_id: Id) -> Result<mpsc::UnboundedReceiver<StreamMessage>> {
        let (sender, receiver) = mpsc::unbounded_channel();

        let streaming_session = StreamingSession {
            session_id: session_id.clone(),
            sender,
            created_at: chrono::Utc::now(),
            message_count: 0,
        };

        // Store the streaming session
        {
            let mut streams = self.active_streams.write().await;
            streams.insert(session_id.clone(), streaming_session);
        }

        info!("Created streaming session: {}", session_id);
        Ok(receiver)
    }

    /// Send an AI response message through the stream
    pub async fn send_ai_response(
        &self,
        session_id: &Id,
        request_id: Option<Id>,
        text: &str,
        partial: bool,
        confidence: Option<f32>,
    ) -> Result<()> {
        let message = StreamMessage::new(
            session_id.clone(),
            request_id,
            MessageType::AiResponse,
            MessageContent::AiResponse {
                text: text.to_string(),
                partial,
                confidence,
            },
        );

        self.send_message(session_id, message).await
    }

    /// Send a thinking process message through the stream
    pub async fn send_thinking(
        &self,
        session_id: &Id,
        request_id: Option<Id>,
        thinking_text: &str,
    ) -> Result<()> {
        let message = StreamMessage::new(
            session_id.clone(),
            request_id,
            MessageType::Thinking,
            MessageContent::Thinking {
                text: thinking_text.to_string(),
            },
        );

        self.send_message(session_id, message).await
    }

    /// Send a tool call message through the stream
    pub async fn send_tool_call(
        &self,
        session_id: &Id,
        request_id: Option<Id>,
        tool_name: &str,
        operation: crate::models::ToolOperation,
        parameters: serde_json::Value,
        result: Option<crate::models::ToolResult>,
    ) -> Result<()> {
        let message = StreamMessage::new(
            session_id.clone(),
            request_id,
            MessageType::ToolCall,
            MessageContent::ToolCall {
                tool_name: tool_name.to_string(),
                operation,
                parameters,
                result,
            },
        );

        self.send_message(session_id, message).await
    }

    /// Send a user interaction request through the stream
    pub async fn send_user_interaction(
        &self,
        session_id: &Id,
        request_id: Option<Id>,
        interaction_id: Id,
        interaction_type: &str,
        prompt: &str,
        proposal_id: Option<Id>,
        timeout_seconds: Option<u32>,
    ) -> Result<()> {
        let message = StreamMessage::new(
            session_id.clone(),
            request_id,
            MessageType::UserInteraction,
            MessageContent::UserInteraction {
                interaction_id,
                interaction_type: interaction_type.to_string(),
                prompt: prompt.to_string(),
                proposal_id,
                timeout: timeout_seconds,
            },
        );

        self.send_message(session_id, message).await
    }

    /// Send an error message through the stream
    pub async fn send_error(
        &self,
        session_id: &Id,
        request_id: Option<Id>,
        error_code: &str,
        message: &str,
        details: Option<&str>,
        recoverable: bool,
    ) -> Result<()> {
        let stream_message = StreamMessage::new(
            session_id.clone(),
            request_id,
            MessageType::Error,
            MessageContent::Error {
                error_code: error_code.to_string(),
                message: message.to_string(),
                details: details.map(|d| d.to_string()),
                recoverable,
            },
        );

        self.send_message(session_id, stream_message).await
    }

    /// Send a system event message through the stream
    pub async fn send_system_event(
        &self,
        session_id: &Id,
        request_id: Option<Id>,
        event: &str,
        status: &str,
        summary: Option<serde_json::Value>,
    ) -> Result<()> {
        let message = StreamMessage::new(
            session_id.clone(),
            request_id.clone(),
            MessageType::System,
            MessageContent::System {
                event: event.to_string(),
                request_id: request_id.clone(),
                status: status.to_string(),
                summary,
            },
        );

        self.send_message(session_id, message).await
    }

    /// Send a progress update (delegated to progress tracker)
    pub async fn send_progress_update(
        &self,
        _session_id: &Id,
        _request_id: Option<Id>,
        operation_id: &Id,
        phase: &str,
        percentage: u8,
        description: &str,
    ) -> Result<()> {
        // Use the progress tracker for consistent progress management
        self.progress_tracker.update_progress(operation_id, phase, percentage, description).await
    }

    /// Get the progress tracker for direct access
    pub fn progress_tracker(&self) -> &Arc<ProgressTracker> {
        &self.progress_tracker
    }

    /// Close a streaming session
    pub async fn close_stream(&self, session_id: &Id) -> Result<()> {
        let removed = {
            let mut streams = self.active_streams.write().await;
            streams.remove(session_id)
        };

        if removed.is_some() {
            info!("Closed streaming session: {}", session_id);
        } else {
            warn!("Attempted to close non-existent streaming session: {}", session_id);
        }

        Ok(())
    }

    /// Get statistics about active streams
    pub async fn get_stream_stats(&self) -> StreamingStats {
        let streams = self.active_streams.read().await;
        let total_sessions = streams.len();
        let total_messages = streams.values().map(|s| s.message_count).sum();

        StreamingStats {
            active_sessions: total_sessions,
            total_messages_sent: total_messages,
            average_messages_per_session: if total_sessions > 0 {
                total_messages as f64 / total_sessions as f64
            } else {
                0.0
            },
        }
    }

    /// Clean up old or inactive streaming sessions
    pub async fn cleanup_inactive_sessions(&self, max_age_hours: u32) -> usize {
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(max_age_hours as i64);
        let mut streams = self.active_streams.write().await;

        let initial_count = streams.len();
        streams.retain(|_, session| session.created_at > cutoff);

        let removed_count = initial_count - streams.len();
        if removed_count > 0 {
            info!("Cleaned up {} inactive streaming sessions", removed_count);
        }

        removed_count
    }

    /// Internal method to send a message through the appropriate stream
    async fn send_message(&self, session_id: &Id, message: StreamMessage) -> Result<()> {
        let send_result = {
            let mut streams = self.active_streams.write().await;

            if let Some(session) = streams.get_mut(session_id) {
                session.message_count += 1;
                session.sender.send(message).map_err(|e| anyhow::anyhow!("Send failed: {}", e))
            } else {
                Err(anyhow::anyhow!("No active stream for session: {}", session_id))
            }
        };

        match send_result {
            Ok(_) => {
                debug!("Sent streaming message to session: {}", session_id);
                Ok(())
            }
            Err(e) => {
                error!("Failed to send streaming message to session {}: {}", session_id, e);
                // Remove the failed session
                let mut streams = self.active_streams.write().await;
                streams.remove(session_id);
                Err(e)
            }
        }
    }
}

/// Statistics about streaming performance and usage
#[derive(Debug, Clone)]
pub struct StreamingStats {
    pub active_sessions: usize,
    pub total_messages_sent: u64,
    pub average_messages_per_session: f64,
}

/// Helper trait for components that want to stream messages
#[async_trait::async_trait]
pub trait Streamable {
    async fn get_streaming_handler(&self) -> Option<&Arc<StreamingHandler>>;

    async fn stream_ai_response(&self, session_id: &Id, text: &str, partial: bool) -> Result<()> {
        if let Some(handler) = self.get_streaming_handler().await {
            handler.send_ai_response(session_id, None, text, partial, None).await
        } else {
            Ok(()) // Silently ignore if no streaming handler available
        }
    }

    async fn stream_thinking(&self, session_id: &Id, thinking: &str) -> Result<()> {
        if let Some(handler) = self.get_streaming_handler().await {
            handler.send_thinking(session_id, None, thinking).await
        } else {
            Ok(())
        }
    }

    async fn stream_error(&self, session_id: &Id, error_code: &str, message: &str) -> Result<()> {
        if let Some(handler) = self.get_streaming_handler().await {
            handler.send_error(session_id, None, error_code, message, None, true).await
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_streaming_session_lifecycle() {
        let handler = StreamingHandler::new(StreamingConfig::default());
        let session_id = uuid::Uuid::new_v4().to_string();

        // Create stream
        let mut receiver = handler.create_stream(session_id.clone()).await.unwrap();

        // Send AI response
        handler.send_ai_response(
            &session_id,
            None,
            "Test response",
            false,
            Some(0.95)
        ).await.unwrap();

        // Receive message
        let message = timeout(Duration::from_millis(100), receiver.recv()).await.unwrap().unwrap();
        assert_eq!(message.session_id, session_id);

        if let MessageContent::AiResponse { text, confidence, .. } = message.content {
            assert_eq!(text, "Test response");
            assert_eq!(confidence, Some(0.95));
        } else {
            panic!("Expected AI response content");
        }

        // Close stream
        handler.close_stream(&session_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_streaming_stats() {
        let handler = StreamingHandler::new(StreamingConfig::default());
        let session_id1 = uuid::Uuid::new_v4().to_string();
        let session_id2 = uuid::Uuid::new_v4().to_string();

        // Create multiple streams
        let _receiver1 = handler.create_stream(session_id1.clone()).await.unwrap();
        let _receiver2 = handler.create_stream(session_id2.clone()).await.unwrap();

        // Send some messages
        handler.send_ai_response(&session_id1, None, "Response 1", false, None).await.unwrap();
        handler.send_ai_response(&session_id2, None, "Response 2", false, None).await.unwrap();
        handler.send_thinking(&session_id1, None, "Thinking...").await.unwrap();

        // Check stats
        let stats = handler.get_stream_stats().await;
        assert_eq!(stats.active_sessions, 2);
        assert_eq!(stats.total_messages_sent, 3);
        assert_eq!(stats.average_messages_per_session, 1.5);
    }
}
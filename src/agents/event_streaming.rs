// Event streaming handler for mapping Goose AgentEvent to Kaiak notification formats
// User Story 4: Stream Agent Events to Clients

use tracing::{debug, error, info};
use futures::StreamExt;

use goose::agents::AgentEvent;
use goose::conversation::message::Message;
use crate::models::events::{
    AgentEventNotification, AgentEventType, AgentEventContent,
    ToolCallStatus, UserInteractionType,
    FileOperation, EventMetadata,
};
use crate::KaiakResult;

/// Event streaming handler for Goose agent events
pub struct EventStreamingHandler {
    /// Sequence number for event tracking
    sequence_number: std::sync::atomic::AtomicU64,
}

impl EventStreamingHandler {
    pub fn new() -> Self {
        Self {
            sequence_number: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// T039: Create event streaming handler mapping Goose AgentEvent to Kaiak notification formats
    /// Stream Goose agent events and convert them to Kaiak notifications
    pub async fn stream_agent_events<F>(
        &self,
        session_id: &str,
        request_id: Option<String>,
        mut event_stream: futures::stream::BoxStream<'_, Result<AgentEvent, anyhow::Error>>,
        mut notification_sender: F,
    ) -> KaiakResult<()>
    where
        F: FnMut(AgentEventNotification) -> futures::future::Ready<()>,
    {
        info!("Starting event stream for session: {}", session_id);

        while let Some(event_result) = event_stream.next().await {
            match event_result {
                Ok(event) => {
                    debug!("Received Goose event: {:?}", event);

                    // Map Goose event to Kaiak notification
                    match self.map_event_to_notification(session_id, request_id.as_deref(), event).await {
                        Ok(Some(notification)) => {
                            debug!("Sending notification: {:?}", notification.event_type);
                            notification_sender(notification).await;
                        }
                        Ok(None) => {
                            debug!("Event did not map to a notification (skipped)");
                        }
                        Err(e) => {
                            error!("Failed to map event to notification: {}", e);
                            // Send error notification
                            let error_notification = self.create_error_notification(
                                session_id,
                                request_id.as_deref(),
                                &format!("Event mapping error: {}", e),
                            );
                            notification_sender(error_notification).await;
                        }
                    }
                }
                Err(e) => {
                    error!("Error in event stream: {}", e);
                    // Send error notification
                    let error_notification = self.create_error_notification(
                        session_id,
                        request_id.as_deref(),
                        &format!("Stream error: {}", e),
                    );
                    notification_sender(error_notification).await;
                    break;
                }
            }
        }

        info!("Event stream completed for session: {}", session_id);
        Ok(())
    }

    /// Map Goose AgentEvent to Kaiak AgentEventNotification
    async fn map_event_to_notification(
        &self,
        session_id: &str,
        request_id: Option<&str>,
        event: AgentEvent,
    ) -> KaiakResult<Option<AgentEventNotification>> {
        let notification = match event {
            AgentEvent::Message(msg) => {
                // T041: Implement AI response notification mapping
                self.map_ai_response_notification(session_id, request_id, msg).await?
            }
            AgentEvent::McpNotification((req_id, notif)) => {
                // T042: Implement tool call notification mapping
                let notif_json = serde_json::to_value(&notif)
                    .unwrap_or_else(|_| serde_json::json!({"method": "unknown"}));
                self.map_tool_call_notification(session_id, request_id, req_id, notif_json).await?
            }
            AgentEvent::ModelChange { model, mode } => {
                // T046: Implement system notification mapping (model change)
                self.map_model_change_notification(session_id, request_id, model, mode).await?
            }
            AgentEvent::HistoryReplaced(conv) => {
                // T046: Implement system notification mapping (history compaction)
                self.map_history_compacted_notification(session_id, request_id, conv.messages().clone()).await?
            }
        };

        Ok(Some(notification))
    }

    /// T040: Implement progress notification mapping
    fn create_progress_notification(
        &self,
        session_id: &str,
        request_id: Option<&str>,
        percentage: u8,
        phase: String,
        description: String,
    ) -> AgentEventNotification {
        AgentEventNotification {
            session_id: session_id.to_string(),
            request_id: request_id.map(String::from),
            message_id: format!("msg-{}", self.next_sequence()),
            timestamp: chrono::Utc::now(),
            event_type: AgentEventType::Progress,
            content: AgentEventContent::Progress {
                percentage,
                phase,
                description,
                current_step: None,
                total_steps: None,
            },
            metadata: self.create_metadata(),
        }
    }

    /// T041: Implement AI response notification mapping
    async fn map_ai_response_notification(
        &self,
        session_id: &str,
        request_id: Option<&str>,
        message: Message,
    ) -> KaiakResult<AgentEventNotification> {
        debug!("Mapping AI response notification");

        // Extract text content from message
        let text = message.content.iter()
            .filter_map(|content| {
                // Try to extract text from message content
                match content {
                    _ => Some(format!("{:?}", content))
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(AgentEventNotification {
            session_id: session_id.to_string(),
            request_id: request_id.map(String::from),
            message_id: format!("msg-{}", self.next_sequence()),
            timestamp: chrono::Utc::now(),
            event_type: AgentEventType::AiResponse,
            content: AgentEventContent::AiResponse {
                text,
                partial: false,
                confidence: None,
                tokens: None,
            },
            metadata: self.create_metadata(),
        })
    }

    /// T042: Implement tool call notification mapping
    async fn map_tool_call_notification(
        &self,
        session_id: &str,
        request_id: Option<&str>,
        tool_req_id: String,
        notification: serde_json::Value,
    ) -> KaiakResult<AgentEventNotification> {
        debug!("Mapping tool call notification: request_id={}", tool_req_id);

        // Parse MCP notification into tool call details
        let tool_name = notification.get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("mcp_tool")
            .to_string();

        let operation = "execute".to_string();
        let parameters = notification.get("params")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        Ok(AgentEventNotification {
            session_id: session_id.to_string(),
            request_id: request_id.map(String::from),
            message_id: format!("msg-{}", self.next_sequence()),
            timestamp: chrono::Utc::now(),
            event_type: AgentEventType::ToolCall,
            content: AgentEventContent::ToolCall {
                tool_name,
                operation,
                parameters,
                status: ToolCallStatus::Executing,
                result: None,
            },
            metadata: self.create_metadata(),
        })
    }

    /// T043: Implement user interaction notification mapping
    fn create_user_interaction_notification(
        &self,
        session_id: &str,
        request_id: Option<&str>,
        interaction_id: String,
        interaction_type: UserInteractionType,
        prompt: String,
    ) -> AgentEventNotification {
        AgentEventNotification {
            session_id: session_id.to_string(),
            request_id: request_id.map(String::from),
            message_id: format!("msg-{}", self.next_sequence()),
            timestamp: chrono::Utc::now(),
            event_type: AgentEventType::UserInteraction,
            content: AgentEventContent::UserInteraction {
                interaction_id,
                interaction_type,
                prompt,
                options: None,
                default_response: None,
                timeout: Some(60), // Default 60 second timeout
            },
            metadata: self.create_metadata(),
        }
    }

    /// T044: Implement file modification notification mapping
    fn create_file_modification_notification(
        &self,
        session_id: &str,
        request_id: Option<&str>,
        file_path: std::path::PathBuf,
        operation: FileOperation,
    ) -> AgentEventNotification {
        AgentEventNotification {
            session_id: session_id.to_string(),
            request_id: request_id.map(String::from),
            message_id: format!("msg-{}", self.next_sequence()),
            timestamp: chrono::Utc::now(),
            event_type: AgentEventType::FileModification,
            content: AgentEventContent::FileModification {
                proposal_id: format!("proposal-{}", self.next_sequence()),
                file_path,
                operation,
                diff: None,
                requires_approval: true,
            },
            metadata: self.create_metadata(),
        }
    }

    /// T045: Implement error notification mapping
    fn create_error_notification(
        &self,
        session_id: &str,
        request_id: Option<&str>,
        error_message: &str,
    ) -> AgentEventNotification {
        AgentEventNotification {
            session_id: session_id.to_string(),
            request_id: request_id.map(String::from),
            message_id: format!("msg-{}", self.next_sequence()),
            timestamp: chrono::Utc::now(),
            event_type: AgentEventType::Error,
            content: AgentEventContent::Error {
                error_code: "AGENT_ERROR".to_string(),
                message: error_message.to_string(),
                details: None,
                recoverable: true,
                suggested_action: None,
            },
            metadata: self.create_metadata(),
        }
    }

    /// T046: Implement system notification mapping (model change)
    async fn map_model_change_notification(
        &self,
        session_id: &str,
        request_id: Option<&str>,
        model: String,
        mode: String,
    ) -> KaiakResult<AgentEventNotification> {
        debug!("Mapping model change notification: {} -> {}", model, mode);

        Ok(AgentEventNotification {
            session_id: session_id.to_string(),
            request_id: request_id.map(String::from),
            message_id: format!("msg-{}", self.next_sequence()),
            timestamp: chrono::Utc::now(),
            event_type: AgentEventType::ModelChange,
            content: AgentEventContent::ModelChange {
                old_model: "".to_string(), // Goose doesn't provide old model
                new_model: model,
                reason: mode,
            },
            metadata: self.create_metadata(),
        })
    }

    /// T046: Implement system notification mapping (history compaction)
    async fn map_history_compacted_notification(
        &self,
        session_id: &str,
        request_id: Option<&str>,
        conversation: Vec<Message>,
    ) -> KaiakResult<AgentEventNotification> {
        debug!("Mapping history compacted notification");

        let compacted_length = conversation.len() as u32;

        Ok(AgentEventNotification {
            session_id: session_id.to_string(),
            request_id: request_id.map(String::from),
            message_id: format!("msg-{}", self.next_sequence()),
            timestamp: chrono::Utc::now(),
            event_type: AgentEventType::HistoryCompacted,
            content: AgentEventContent::HistoryCompacted {
                original_length: 0, // Goose doesn't provide original length
                compacted_length,
                tokens_saved: None,
            },
            metadata: self.create_metadata(),
        })
    }

    /// Create event metadata with sequence tracking
    fn create_metadata(&self) -> EventMetadata {
        EventMetadata {
            sequence_number: self.next_sequence(),
            correlation_id: None,
            trace_id: None,
            processing_duration: None,
        }
    }

    /// Get next sequence number
    fn next_sequence(&self) -> u64 {
        self.sequence_number.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }
}

impl Default for EventStreamingHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_streaming_handler_creation() {
        let handler = EventStreamingHandler::new();
        assert_eq!(handler.next_sequence(), 0);
        assert_eq!(handler.next_sequence(), 1);
    }

    #[test]
    fn test_error_notification_creation() {
        let handler = EventStreamingHandler::new();
        let notification = handler.create_error_notification(
            "test-session",
            Some("test-request"),
            "Test error message",
        );

        assert_eq!(notification.session_id, "test-session");
        assert_eq!(notification.request_id, Some("test-request".to_string()));

        if let AgentEventContent::Error { message, .. } = notification.content {
            assert_eq!(message, "Test error message");
        } else {
            panic!("Expected Error content");
        }
    }
}

//! Generate fix handler for processing migration incidents with Goose agent.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use validator::Validate;

use goose::agents::AgentEvent;
use goose::conversation::message::{ActionRequiredData, Message, MessageContent};
use goose::permission::permission_confirmation::PrincipalType;
use goose::permission::{Permission, PermissionConfirmation};

use super::interaction_manager::InteractionManager;
use crate::agent::GooseAgentManager;
use crate::jsonrpc::{methods::GENERATE_FIX_DATA, JsonRpcNotification, NotificationSender};
use crate::models::{configuration::AgentConfig, incidents::MigrationIncident};
use crate::KaiakResult;

const INTERACTION_TIMEOUT_SECS: u64 = 300;

/// Request type for kaiak/generate_fix endpoint
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct GenerateFixRequest {
    /// Optional session identifier - if not provided, a new session will be created
    /// and the generated session ID will be returned in the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Array of migration incidents to process
    #[validate(length(min = 1, max = 1000, message = "Must provide 1-1000 incidents"))]
    #[validate(nested)]
    pub incidents: Vec<MigrationIncident>,
    /// Optional context for the migration process
    pub migration_context: Option<serde_json::Value>,
    pub agent_config: AgentConfig,
}

/// Response type for kaiak/generate_fix endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateFixResponse {
    pub request_id: String,
    pub session_id: String,
    pub created_at: String,
}

/// Kind of data being sent in generate_fix notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerateFixDataKind {
    AiMessage,
    ToolCall,
    ToolResponse,
    UserInteraction,
    Thinking,
    Error,
    System,
}

/// Data notification sent to client during generate_fix processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateFixData {
    pub request_id: String,
    pub session_id: String,
    pub kind: GenerateFixDataKind,
    pub payload: serde_json::Value,
}

/// User interaction types that require client response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "interaction_type", rename_all = "snake_case")]
pub enum UserInteractionPayload {
    /// Tool needs approval to execute
    ToolConfirmation {
        id: String,
        tool_name: String,
        arguments: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        prompt: Option<String>,
    },
    /// MCP tool requesting user input
    Elicitation {
        id: String,
        message: String,
        requested_schema: serde_json::Value,
    },
}

/// Handler for kaiak/generate_fix endpoint
/// Coordinates with Goose agent to process migration incidents
pub struct GenerateFixHandler {
    agent_manager: Arc<GooseAgentManager>,
    interaction_manager: Arc<InteractionManager>,
    active_requests: Arc<RwLock<std::collections::HashMap<String, GenerateFixRequest>>>,
    #[allow(dead_code)]
    base_config: Arc<crate::models::configuration::BaseConfig>,
}

impl GenerateFixHandler {
    pub fn new(
        agent_manager: Arc<GooseAgentManager>,
        interaction_manager: Arc<InteractionManager>,
        base_config: Arc<crate::models::configuration::BaseConfig>,
    ) -> Self {
        Self {
            agent_manager,
            interaction_manager,
            active_requests: Arc::new(RwLock::new(std::collections::HashMap::new())),
            base_config,
        }
    }

    pub async fn handle_generate_fix(
        &self,
        request: GenerateFixRequest,
        notifier: NotificationSender,
    ) -> KaiakResult<GenerateFixResponse> {
        info!(
            "Processing generate_fix request for session: {:?}",
            request.session_id
        );

        if let Err(validation_errors) = request.validate() {
            error!("Request validation failed: {:?}", validation_errors);
            let error_messages: Vec<String> = validation_errors
                .field_errors()
                .into_iter()
                .flat_map(|(field, errors)| {
                    errors.iter().map(move |error| {
                        format!(
                            "Field '{}': {}",
                            field,
                            error
                                .message
                                .as_ref()
                                .map(|m| m.as_ref())
                                .unwrap_or("validation error")
                        )
                    })
                })
                .collect();

            return Err(crate::KaiakError::agent(
                format!("Request validation failed: {}", error_messages.join(", ")),
                None,
            ));
        }

        let request_id = Uuid::new_v4().to_string();
        {
            let mut active = self.active_requests.write().await;
            active.insert(request_id.clone(), request.clone());
        }

        info!("Processing {} migration incidents", request.incidents.len());

        match self
            .initiate_agent_processing(&request_id, &request, &notifier)
            .await
        {
            Ok(session_id) => {
                info!(
                    "Generate fix request {} completed successfully with session {}",
                    request_id, session_id
                );

                // Clean up active request
                {
                    let mut active = self.active_requests.write().await;
                    active.remove(&request_id);
                }

                Ok(GenerateFixResponse {
                    request_id,
                    session_id,
                    created_at: chrono::Utc::now().to_rfc3339(),
                })
            }
            Err(e) => {
                error!(
                    "Failed to initiate generate fix request {}: {}",
                    request_id, e
                );

                // Remove from active requests on failure
                {
                    let mut active = self.active_requests.write().await;
                    active.remove(&request_id);
                }

                Err(e)
            }
        }
    }

    /// Send a typed notification to the client
    fn send_notification(
        &self,
        notifier: &NotificationSender,
        request_id: &str,
        session_id: &str,
        kind: GenerateFixDataKind,
        payload: serde_json::Value,
    ) {
        let data = GenerateFixData {
            request_id: request_id.to_string(),
            session_id: session_id.to_string(),
            kind,
            payload,
        };

        let notification = JsonRpcNotification::new(
            GENERATE_FIX_DATA,
            Some(serde_json::to_value(&data).unwrap_or_default()),
        );

        if let Err(e) = notifier.send(notification) {
            warn!("Failed to send notification: {}", e);
        }
    }

    /// Send user interaction notification and wait for response
    async fn handle_tool_confirmation(
        &self,
        notifier: &NotificationSender,
        request_id: &str,
        session_id: &str,
        agent: &goose::agents::Agent,
        id: &str,
        tool_name: &str,
        arguments: &rmcp::model::JsonObject,
        prompt: &Option<String>,
    ) {
        // Register that we're waiting for this confirmation
        let rx = self
            .interaction_manager
            .register_confirmation(id.to_string())
            .await;

        // Send notification to client
        let payload = UserInteractionPayload::ToolConfirmation {
            id: id.to_string(),
            tool_name: tool_name.to_string(),
            arguments: serde_json::to_value(arguments).unwrap_or_default(),
            prompt: prompt.clone(),
        };

        self.send_notification(
            notifier,
            request_id,
            session_id,
            GenerateFixDataKind::UserInteraction,
            serde_json::to_value(&payload).unwrap_or_default(),
        );

        // Wait for client response (with timeout)
        let confirmation = match tokio::time::timeout(
            Duration::from_secs(INTERACTION_TIMEOUT_SECS),
            rx,
        )
        .await
        {
            Ok(Ok(confirmation)) => {
                debug!("Received tool confirmation for {}: {:?}", id, confirmation);
                confirmation
            }
            Ok(Err(_)) => {
                warn!("Tool confirmation channel closed for {}, denying", id);
                PermissionConfirmation {
                    principal_type: PrincipalType::Tool,
                    permission: Permission::DenyOnce,
                }
            }
            Err(_) => {
                warn!("Tool confirmation timeout for {}, denying", id);
                self.interaction_manager.cancel_confirmation(id).await;
                PermissionConfirmation {
                    principal_type: PrincipalType::Tool,
                    permission: Permission::DenyOnce,
                }
            }
        };

        // Forward to agent
        agent.handle_confirmation(id.to_string(), confirmation).await;
    }

    /// Handle elicitation request
    async fn handle_elicitation(
        &self,
        notifier: &NotificationSender,
        request_id: &str,
        session_id: &str,
        agent: &goose::agents::Agent,
        session_config: &goose::agents::SessionConfig,
        id: &str,
        message: &str,
        requested_schema: &serde_json::Value,
    ) {
        // Register that we're waiting for this elicitation
        let rx = self
            .interaction_manager
            .register_elicitation(id.to_string())
            .await;

        // Send notification to client
        let payload = UserInteractionPayload::Elicitation {
            id: id.to_string(),
            message: message.to_string(),
            requested_schema: requested_schema.clone(),
        };

        self.send_notification(
            notifier,
            request_id,
            session_id,
            GenerateFixDataKind::UserInteraction,
            serde_json::to_value(&payload).unwrap_or_default(),
        );

        // Wait for client response
        match tokio::time::timeout(Duration::from_secs(INTERACTION_TIMEOUT_SECS), rx).await {
            Ok(Ok(user_data)) => {
                debug!("Received elicitation response for {}", id);

                // Create response message and send to agent
                let response_msg = Message::user()
                    .with_content(MessageContent::action_required_elicitation_response(
                        id.to_string(),
                        user_data,
                    ))
                    .with_visibility(false, true);

                // This call triggers ActionRequiredManager::submit_response internally
                if let Err(e) = agent.reply(response_msg, session_config.clone(), None).await {
                    error!("Failed to submit elicitation response: {}", e);
                }
            }
            Ok(Err(_)) => {
                warn!("Elicitation channel closed for {}", id);
                self.interaction_manager.cancel_elicitation(id).await;
            }
            Err(_) => {
                warn!("Elicitation timeout for {}", id);
                self.interaction_manager.cancel_elicitation(id).await;
            }
        }
    }

    /// Get status of active requests
    pub async fn get_active_request_count(&self) -> usize {
        let active = self.active_requests.read().await;
        active.len()
    }

    /// Initiate agent processing with Goose session management
    async fn initiate_agent_processing(
        &self,
        request_id: &str,
        request: &GenerateFixRequest,
        notifier: &NotificationSender,
    ) -> KaiakResult<String> {
        debug!("Initiating agent processing for request: {}", request_id);

        let session_info = match self
            .agent_manager
            .get_or_create_session(request.session_id.as_deref(), &request.agent_config)
            .await
        {
            Ok(session_info) => {
                debug!(
                    "Session ready for processing: {} (workspace: {:?})",
                    session_info.session.id, request.agent_config.workspace
                );
                session_info
            }
            Err(e) => {
                error!("Failed to get or create session: {}", e);
                return Err(e);
            }
        };

        let session_id = session_info.session.id.clone();

        // Lock the session to prevent other requests from using it
        match self.agent_manager.lock_session(&session_id).await {
            Ok(_) => {
                debug!("Successfully locked session: {}", session_id);
            }
            Err(e) => {
                error!("Failed to lock session {}: {}", session_id, e);
                return Err(e);
            }
        }

        let (agent, session_config) = self
            .agent_manager
            .create_agent(&session_id, &request.agent_config)
            .await?;

        let incident_messages: Vec<String> =
            request.incidents.iter().map(|i| i.message.clone()).collect();
        let prompt = format!(
            "We found migration issues identified by static analysis tools in the project. Help fix them. Here are the issues:{}{}",
            if incident_messages.is_empty() { " (no incidents provided)" } else { "" },
            if incident_messages.len() == 1 {
                format!(" {}", incident_messages[0])
            } else {
                incident_messages
                    .iter()
                    .enumerate()
                    .map(|(idx, msg)| format!("\n  {}. {}", idx + 1, msg))
                    .collect::<String>()
            }
        );

        let message = Message::user().with_text(&prompt);

        let mut stream = match agent.reply(message, session_config.clone(), None).await {
            Ok(stream) => stream,
            Err(e) => {
                error!("Failed to reply to message: {}", e);
                if let Err(unlock_err) = self.agent_manager.unlock_session(&session_id).await {
                    warn!("Failed to unlock session after error: {}", unlock_err);
                }
                return Err(crate::KaiakError::agent(
                    format!("Failed to reply to message: {}", e),
                    None,
                ));
            }
        };

        // Process the stream
        while let Some(event) = futures::StreamExt::next(&mut stream).await {
            match event {
                Ok(AgentEvent::Message(msg)) => {
                    self.process_message(
                        notifier,
                        request_id,
                        &session_id,
                        &agent,
                        &session_config,
                        &msg,
                    )
                    .await;
                }
                Ok(AgentEvent::HistoryReplaced(_history)) => {
                    debug!("History replaced");
                }
                Ok(AgentEvent::McpNotification((_req_id, notif))) => {
                    debug!("MCP notification: {:?}", notif);
                }
                Ok(AgentEvent::ModelChange { model, mode }) => {
                    debug!("Model change: {} ({})", model, mode);
                    self.send_notification(
                        notifier,
                        request_id,
                        &session_id,
                        GenerateFixDataKind::System,
                        serde_json::json!({
                            "event": "model_change",
                            "model": model,
                            "mode": mode,
                        }),
                    );
                }
                Err(e) => {
                    error!("Error getting stream event: {:?}", e);
                    self.send_notification(
                        notifier,
                        request_id,
                        &session_id,
                        GenerateFixDataKind::Error,
                        serde_json::json!({
                            "error": e.to_string(),
                        }),
                    );
                }
            }
        }

        if let Err(unlock_err) = self.agent_manager.unlock_session(&session_id).await {
            warn!("Failed to unlock session after processing: {}", unlock_err);
        }

        Ok(session_id)
    }

    /// Process a single message from the agent stream
    async fn process_message(
        &self,
        notifier: &NotificationSender,
        request_id: &str,
        session_id: &str,
        agent: &goose::agents::Agent,
        session_config: &goose::agents::SessionConfig,
        message: &Message,
    ) {
        for content in &message.content {
            match content {
                MessageContent::Text(text) => {
                    self.send_notification(
                        notifier,
                        request_id,
                        session_id,
                        GenerateFixDataKind::AiMessage,
                        serde_json::json!({
                            "role": format!("{:?}", message.role),
                            "text": text.text,
                        }),
                    );
                }

                MessageContent::Thinking(thinking) => {
                    self.send_notification(
                        notifier,
                        request_id,
                        session_id,
                        GenerateFixDataKind::Thinking,
                        serde_json::json!({
                            "thinking": thinking.thinking,
                        }),
                    );
                }

                MessageContent::ToolRequest(req) => {
                    let tool_info = match &req.tool_call {
                        Ok(call) => serde_json::json!({
                            "id": req.id,
                            "tool_name": call.name,
                            "arguments": call.arguments,
                        }),
                        Err(e) => serde_json::json!({
                            "id": req.id,
                            "error": format!("{:?}", e),
                        }),
                    };
                    self.send_notification(
                        notifier,
                        request_id,
                        session_id,
                        GenerateFixDataKind::ToolCall,
                        tool_info,
                    );
                }

                MessageContent::ToolResponse(resp) => {
                    let result_info = match &resp.tool_result {
                        Ok(result) => serde_json::json!({
                            "id": resp.id,
                            "is_error": result.is_error,
                            "content_count": result.content.len(),
                        }),
                        Err(e) => serde_json::json!({
                            "id": resp.id,
                            "error": format!("{:?}", e),
                        }),
                    };
                    self.send_notification(
                        notifier,
                        request_id,
                        session_id,
                        GenerateFixDataKind::ToolResponse,
                        result_info,
                    );
                }

                MessageContent::ActionRequired(action) => match &action.data {
                    ActionRequiredData::ToolConfirmation {
                        id,
                        tool_name,
                        arguments,
                        prompt,
                    } => {
                        self.handle_tool_confirmation(
                            notifier,
                            request_id,
                            session_id,
                            agent,
                            id,
                            tool_name,
                            arguments,
                            prompt,
                        )
                        .await;
                    }
                    ActionRequiredData::Elicitation {
                        id,
                        message: elicit_msg,
                        requested_schema,
                    } => {
                        self.handle_elicitation(
                            notifier,
                            request_id,
                            session_id,
                            agent,
                            session_config,
                            id,
                            elicit_msg,
                            requested_schema,
                        )
                        .await;
                    }
                    ActionRequiredData::ElicitationResponse { .. } => {
                        // This is a response we sent, not something we need to handle
                        debug!("Received ElicitationResponse in stream (expected)");
                    }
                },

                MessageContent::SystemNotification(notif) => {
                    self.send_notification(
                        notifier,
                        request_id,
                        session_id,
                        GenerateFixDataKind::System,
                        serde_json::json!({
                            "notification_type": format!("{:?}", notif.notification_type),
                            "message": notif.msg,
                        }),
                    );
                }

                // Content types we don't specifically handle
                MessageContent::Image(_)
                | MessageContent::ToolConfirmationRequest(_)
                | MessageContent::FrontendToolRequest(_)
                | MessageContent::RedactedThinking(_) => {
                    debug!("Unhandled content type: {:?}", content);
                }
            }
        }
    }

    /// Cancel a generate fix request
    pub async fn cancel_request(&self, request_id: &str) -> KaiakResult<bool> {
        let mut active = self.active_requests.write().await;
        Ok(active.remove(request_id).is_some())
    }
}

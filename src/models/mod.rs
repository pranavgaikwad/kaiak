// Public exports for data models per data-model.md specification

pub mod configuration;
pub mod incidents;
pub mod events;
pub mod interactions;

// Re-export key types for convenience
pub use configuration::{AgentConfiguration, WorkspaceConfig, ToolConfig, PermissionConfig, ToolPermission, CustomToolConfig, ExtensionType};
pub use incidents::{MigrationIncident, IncidentSeverity};
pub use events::{AgentEventNotification, AgentEventType, AgentEventContent, ToolCallStatus, ToolCallResult, UserInteractionType, FileOperation, RiskLevel, SystemEventLevel, EventMetadata};
pub use interactions::{UserInteractionRequest, InteractionContext, ResponseOptions, UserInteractionResponse, UserResponseType};

// Session management types (delegated to Goose but with local tracking)
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub id: String,                    // Client-generated UUID
    pub goose_session_id: String,     // Goose internal session ID
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub configuration: AgentConfiguration,
    pub metrics: SessionMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Creating,     // Session being created
    Ready,        // Session ready for requests
    Processing,   // Agent actively processing
    Waiting,      // Waiting for user interaction
    Completed,    // Processing completed successfully
    Error,        // Session in error state
    Terminated,   // Session terminated by user/system
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetrics {
    pub message_count: u32,
    pub tool_calls_count: u32,
    pub tokens_used: Option<u32>,
    pub processing_time: Option<u64>, // milliseconds
    pub interaction_count: u32,
}
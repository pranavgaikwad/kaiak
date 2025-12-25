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

// Session management now fully delegated to Goose SessionManager
// See src/agents/session_wrapper.rs for the integration layer
// Public exports for data models per data-model.md specification

pub mod configuration;
pub mod incidents;
pub mod events;
pub mod interactions;
pub mod client;
pub mod session;

// Re-export key types for convenience
pub use configuration::{AgentConfig};
pub use incidents::{MigrationIncident, IncidentSeverity};
pub use events::{AgentEventNotification, AgentEventType, AgentEventContent, ToolCallStatus, ToolCallResult, UserInteractionType, FileOperation, RiskLevel, SystemEventLevel, EventMetadata};
pub use interactions::{UserInteractionRequest, InteractionContext, ResponseOptions, UserInteractionResponse, UserResponseType};
pub use client::{ClientConnection, ClientState};
pub use session::SessionManager;

// Session management now fully delegated to Goose SessionManager
// See src/agents/session_wrapper.rs for the integration layer
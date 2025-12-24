// Session management delegated to Goose SessionManager
// All custom session logic has been removed in favor of Goose's native session management
// See src/agents/session_wrapper.rs for the Goose integration wrapper

// Re-export the new session types from the main models module
pub use super::{AgentSession, SessionStatus, SessionMetrics};

// Import Goose session types for compatibility
pub use goose::session::{Session, SessionManager, SessionType};

// Deprecated: Legacy session types for backward compatibility during transition
// These will be removed once all references are updated
#[deprecated(note = "Use AgentSession instead")]
pub type AiSession = AgentSession;

#[deprecated(note = "Use AgentConfiguration instead")]
pub type SessionConfiguration = crate::models::AgentConfiguration;
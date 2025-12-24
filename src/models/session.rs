// Session management delegated to Goose SessionManager
// All custom session logic has been removed in favor of Goose's native session management
// See src/agents/session_wrapper.rs for the Goose integration wrapper

// Import Goose session types - these are the only session types we use now
pub use goose::session::{Session, SessionManager, SessionType};

// Import our session wrapper types
pub use crate::agents::session_wrapper::{GooseSessionWrapper, SessionInfo};
//! Kaiak Migration Server
//!
//! A standalone server integrating Goose AI agent for code migration workflows.
//! Provides LSP-style JSON-RPC communication for IDE extensions.

use anyhow::Result;

pub mod config;
pub mod server;
pub mod goose;
pub mod models;
pub mod handlers;
pub mod agents;

/// Application-wide error types with context preservation
#[derive(Debug, thiserror::Error)]
pub enum KaiakError {
    #[error("Configuration error: {message}")]
    Configuration { message: String },

    #[error("Session error: {message}")]
    Session { message: String, session_id: Option<String> },

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Agent error: {message}")]
    Agent { message: String, context: Option<String> },

    #[error("Transport error: {message}")]
    Transport { message: String },

    #[error("Workspace error: {message}")]
    Workspace { message: String, path: Option<String> },

    #[error("Invalid workspace path: {0}")]
    InvalidWorkspacePath(String),

    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("IO error: {source}")]
    Io {
        #[from]
        source: std::io::Error,
    },

    #[error("Serialization error: {source}")]
    Serialization {
        #[from]
        source: serde_json::Error,
    },

    #[error("Session in use: {session_id}")]
    SessionInUse {
        session_id: String,
        in_use_since: Option<chrono::DateTime<chrono::Utc>>
    },

    #[error("Goose integration error: {message}")]
    GooseIntegration { message: String, context: Option<String> },

    #[error("Agent initialization failed: {message}")]
    AgentInitialization { message: String },

    #[error("Tool execution error: {message}")]
    ToolExecution { message: String, tool_name: Option<String> },

    #[error("User interaction timeout: {message}")]
    InteractionTimeout { message: String },

    #[error("File operation error: {message}")]
    FileOperation { message: String, file_path: Option<String> },
}

impl KaiakError {
    /// Create a configuration error
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration {
            message: message.into(),
        }
    }

    /// Create a session error with optional session ID
    pub fn session(message: impl Into<String>, session_id: Option<String>) -> Self {
        Self::Session {
            message: message.into(),
            session_id,
        }
    }

    /// Create an agent error with optional context
    pub fn agent(message: impl Into<String>, context: Option<String>) -> Self {
        Self::Agent {
            message: message.into(),
            context,
        }
    }

    /// Create a transport error
    pub fn transport(message: impl Into<String>) -> Self {
        Self::Transport {
            message: message.into(),
        }
    }

    /// Create a workspace error with optional path
    pub fn workspace(message: impl Into<String>, path: Option<String>) -> Self {
        Self::Workspace {
            message: message.into(),
            path,
        }
    }

    /// Create a session in use error
    pub fn session_in_use(session_id: impl Into<String>, in_use_since: Option<chrono::DateTime<chrono::Utc>>) -> Self {
        Self::SessionInUse {
            session_id: session_id.into(),
            in_use_since,
        }
    }

    /// Create a Goose integration error
    pub fn goose_integration(message: impl Into<String>, context: Option<String>) -> Self {
        Self::GooseIntegration {
            message: message.into(),
            context,
        }
    }

    /// Create an agent initialization error
    pub fn agent_initialization(message: impl Into<String>) -> Self {
        Self::AgentInitialization {
            message: message.into(),
        }
    }

    /// Create a tool execution error
    pub fn tool_execution(message: impl Into<String>, tool_name: Option<String>) -> Self {
        Self::ToolExecution {
            message: message.into(),
            tool_name,
        }
    }

    /// Create an interaction timeout error
    pub fn interaction_timeout(message: impl Into<String>) -> Self {
        Self::InteractionTimeout {
            message: message.into(),
        }
    }

    /// Create a file operation error
    pub fn file_operation(message: impl Into<String>, file_path: Option<String>) -> Self {
        Self::FileOperation {
            message: message.into(),
            file_path,
        }
    }

    /// Get error code for JSON-RPC responses
    pub fn error_code(&self) -> i32 {
        match self {
            KaiakError::Configuration { .. } => -32014,
            KaiakError::Session { .. } => -32003,
            KaiakError::SessionNotFound(_) => -32003,
            KaiakError::SessionInUse { .. } => -32016, // NEW: for concurrent access blocking
            KaiakError::Agent { .. } => -32006,
            KaiakError::AgentInitialization { .. } => -32006,
            KaiakError::GooseIntegration { .. } => -32006,
            KaiakError::ToolExecution { .. } => -32013,
            KaiakError::InteractionTimeout { .. } => -32013,
            KaiakError::FileOperation { .. } => -32012,
            KaiakError::Transport { .. } => -32001,
            KaiakError::Workspace { .. } => -32002,
            KaiakError::InvalidWorkspacePath(_) => -32002,
            KaiakError::ResourceExhausted(_) => -32015,
            KaiakError::Internal(_) => -32603,
            KaiakError::Io { .. } => -32603,
            KaiakError::Serialization { .. } => -32700,
        }
    }

    /// Get user-friendly error message
    pub fn user_message(&self) -> String {
        match self {
            KaiakError::Configuration { message } => {
                format!("Configuration issue: {}", message)
            }
            KaiakError::Session { message, session_id } => {
                if let Some(id) = session_id {
                    format!("Session error ({}): {}", id, message)
                } else {
                    format!("Session error: {}", message)
                }
            }
            KaiakError::SessionNotFound(id) => {
                format!("Session not found: {}", id)
            }
            KaiakError::Agent { message, .. } => {
                format!("AI agent error: {}", message)
            }
            KaiakError::Transport { message } => {
                format!("Communication error: {}", message)
            }
            KaiakError::Workspace { message, path } => {
                if let Some(p) = path {
                    format!("Workspace error ({}): {}", p, message)
                } else {
                    format!("Workspace error: {}", message)
                }
            }
            KaiakError::InvalidWorkspacePath(path) => {
                format!("Invalid workspace path: {}", path)
            }
            KaiakError::ResourceExhausted(message) => {
                format!("Resource limit exceeded: {}", message)
            }
            KaiakError::Internal(message) => {
                format!("Internal error: {}", message)
            }
            KaiakError::Io { source } => {
                format!("File system error: {}", source)
            }
            KaiakError::Serialization { source } => {
                format!("Data format error: {}", source)
            }
            KaiakError::SessionInUse { session_id, in_use_since } => {
                if let Some(since) = in_use_since {
                    format!("Session {} is currently in use since {}", session_id, since.format("%Y-%m-%d %H:%M:%S UTC"))
                } else {
                    format!("Session {} is currently in use by another client", session_id)
                }
            }
            KaiakError::GooseIntegration { message, context } => {
                if let Some(ctx) = context {
                    format!("Goose integration error ({}): {}", ctx, message)
                } else {
                    format!("Goose integration error: {}", message)
                }
            }
            KaiakError::AgentInitialization { message } => {
                format!("Agent initialization failed: {}", message)
            }
            KaiakError::ToolExecution { message, tool_name } => {
                if let Some(tool) = tool_name {
                    format!("Tool execution error ({}): {}", tool, message)
                } else {
                    format!("Tool execution error: {}", message)
                }
            }
            KaiakError::InteractionTimeout { message } => {
                format!("User interaction timeout: {}", message)
            }
            KaiakError::FileOperation { message, file_path } => {
                if let Some(path) = file_path {
                    format!("File operation error ({}): {}", path, message)
                } else {
                    format!("File operation error: {}", message)
                }
            }
        }
    }
}

/// Convenience type alias for Results
pub type KaiakResult<T> = Result<T, KaiakError>;

/// Extension trait for adding context to Results
pub trait ResultExt<T> {
    fn with_session_context(self, session_id: &str) -> KaiakResult<T>;
    fn with_workspace_context(self, path: &str) -> KaiakResult<T>;
}

impl<T, E> ResultExt<T> for Result<T, E>
where
    E: Into<anyhow::Error>,
{
    fn with_session_context(self, session_id: &str) -> KaiakResult<T> {
        self.map_err(|e| {
            KaiakError::session(
                format!("Operation failed: {}", e.into()),
                Some(session_id.to_string()),
            )
        })
    }

    fn with_workspace_context(self, path: &str) -> KaiakResult<T> {
        self.map_err(|e| {
            KaiakError::workspace(
                format!("Operation failed: {}", e.into()),
                Some(path.to_string()),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = KaiakError::configuration("Invalid config");
        assert_eq!(err.error_code(), -32014);
        assert!(err.user_message().contains("Configuration issue"));
    }

    #[test]
    fn test_session_error_with_id() {
        let err = KaiakError::session("Session failed", Some("session-123".to_string()));
        assert_eq!(err.error_code(), -32003);
        assert!(err.user_message().contains("session-123"));
    }

    #[test]
    fn test_result_extension() {
        let result: Result<(), std::io::Error> = Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "File not found",
        ));

        let kaiak_result = result.with_session_context("test-session");
        assert!(kaiak_result.is_err());

        if let Err(KaiakError::Session { session_id, .. }) = kaiak_result {
            assert_eq!(session_id, Some("test-session".to_string()));
        } else {
            panic!("Expected Session error");
        }
    }
}
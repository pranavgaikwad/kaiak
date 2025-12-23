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

/// Application-wide error types with context preservation
#[derive(Debug, thiserror::Error)]
pub enum KaiakError {
    #[error("Configuration error: {message}")]
    Configuration { message: String },

    #[error("Session error: {message}")]
    Session { message: String, session_id: Option<String> },

    #[error("Agent error: {message}")]
    Agent { message: String, context: Option<String> },

    #[error("Transport error: {message}")]
    Transport { message: String },

    #[error("Workspace error: {message}")]
    Workspace { message: String, path: Option<String> },

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

    /// Get error code for JSON-RPC responses
    pub fn error_code(&self) -> i32 {
        match self {
            KaiakError::Configuration { .. } => -32014,
            KaiakError::Session { .. } => -32003,
            KaiakError::Agent { .. } => -32006,
            KaiakError::Transport { .. } => -32001,
            KaiakError::Workspace { .. } => -32002,
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
            KaiakError::Io { source } => {
                format!("File system error: {}", source)
            }
            KaiakError::Serialization { source } => {
                format!("Data format error: {}", source)
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
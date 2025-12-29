use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use tokio::fs as async_fs;
use validator::{Validate, ValidationError};

/// Represents the persistent connection state between client and server
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[validate(schema(function = "validate_client_connection"))]
pub struct ClientConnection {
    /// Unix socket path to the server
    pub socket_path: String,

    /// Timestamp when connection was established
    pub connected_at: DateTime<Utc>,

    /// Last successful connection validation (for health checks)
    pub last_validated: Option<DateTime<Utc>>,

    /// Optional server version for compatibility checking
    pub server_version: Option<String>,
}

impl ClientConnection {
    /// Create a new client connection
    pub fn new(socket_path: String, server_version: Option<String>) -> Result<Self> {
        let connection = Self {
            socket_path,
            connected_at: Utc::now(),
            last_validated: None,
            server_version,
        };

        // Validate the connection
        connection.validate()?;
        Ok(connection)
    }

    /// Update the last validation timestamp
    pub fn update_validation(&mut self) {
        self.last_validated = Some(Utc::now());
    }
}

/// Manages client connection lifecycle and state persistence
pub struct ClientState {
    /// Current connection information
    pub connection: Option<ClientConnection>,

    /// Path to state file (default: ~/.kaiak/client.state)
    state_file: PathBuf,
}

impl ClientState {
    /// Create a new ClientState with default state file location
    pub fn new() -> Result<Self> {
        let state_file = Self::default_state_path()?;
        Ok(Self {
            connection: None,
            state_file,
        })
    }

    /// Create a new ClientState with custom state file path
    pub fn with_state_file(state_file: PathBuf) -> Self {
        Self {
            connection: None,
            state_file,
        }
    }

    /// Get the default state file path: ~/.kaiak/client.state
    pub fn default_state_path() -> Result<PathBuf> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| anyhow!("Unable to determine home directory"))?;

        let kaiak_dir = home_dir.join(".kaiak");

        // Create .kaiak directory if it doesn't exist
        if !kaiak_dir.exists() {
            fs::create_dir_all(&kaiak_dir)
                .map_err(|e| anyhow!("Failed to create ~/.kaiak directory: {}", e))?;
        }

        Ok(kaiak_dir.join("client.state"))
    }

    /// Load client state from file system
    pub fn load() -> Result<Self> {
        let state_file = Self::default_state_path()?;
        Self::load_from_path(state_file)
    }

    /// Load client state from a specific path
    pub fn load_from_path(state_file: PathBuf) -> Result<Self> {
        let mut client_state = Self::with_state_file(state_file.clone());

        if state_file.exists() {
            let content = fs::read_to_string(&state_file)
                .map_err(|e| anyhow!("Failed to read state file {}: {}", state_file.display(), e))?;

            if !content.trim().is_empty() {
                let connection: ClientConnection = serde_json::from_str(&content)
                    .map_err(|e| anyhow!("Failed to parse state file: {}", e))?;

                // Validate the loaded connection
                connection.validate()
                    .map_err(|e| anyhow!("Invalid connection state: {}", e))?;

                client_state.connection = Some(connection);
            }
        }

        Ok(client_state)
    }

    /// Save current state to file system
    pub fn save(&self) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.state_file.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| anyhow!("Failed to create state directory: {}", e))?;
        }

        let content = match &self.connection {
            Some(connection) => serde_json::to_string_pretty(connection)
                .map_err(|e| anyhow!("Failed to serialize connection state: {}", e))?,
            None => String::new(), // Empty file when no connection
        };

        fs::write(&self.state_file, content)
            .map_err(|e| anyhow!("Failed to write state file {}: {}", self.state_file.display(), e))?;

        Ok(())
    }

    /// Establish new connection
    pub fn connect(&mut self, socket_path: String, server_version: Option<String>) -> Result<()> {
        // Validate socket path format
        Self::validate_socket_path(&socket_path)?;

        let connection = ClientConnection::new(socket_path, server_version)?;
        self.connection = Some(connection);
        self.save()?;
        Ok(())
    }

    /// Remove connection state
    pub fn disconnect(&mut self) -> Result<()> {
        self.connection = None;
        self.save()?;
        Ok(())
    }

    /// Validate current connection is still active
    pub async fn validate_connection(&mut self) -> Result<bool> {
        match &mut self.connection {
            Some(connection) => {
                // Check if socket file exists and is accessible
                let socket_path = Path::new(&connection.socket_path);

                if !socket_path.exists() {
                    return Ok(false);
                }

                // Try to access the socket file metadata
                match async_fs::metadata(&connection.socket_path).await {
                    Ok(metadata) => {
                        // On Unix, check if it's actually a socket
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::FileTypeExt;
                            if !metadata.file_type().is_socket() {
                                return Ok(false);
                            }
                        }

                        // Update validation timestamp
                        connection.update_validation();
                        self.save()?;
                        Ok(true)
                    }
                    Err(_) => Ok(false),
                }
            }
            None => Ok(false),
        }
    }

    /// Get current connection or error if not connected
    pub fn require_connection(&self) -> Result<&ClientConnection> {
        self.connection.as_ref()
            .ok_or_else(|| anyhow!("No connection established. Run 'kaiak connect --socket <path>' first."))
    }

    /// Check if currently connected
    pub fn is_connected(&self) -> bool {
        self.connection.is_some()
    }

    /// Get the state file path
    pub fn state_file_path(&self) -> &Path {
        &self.state_file
    }

    /// Validate socket path format and accessibility
    fn validate_socket_path(socket_path: &str) -> Result<()> {
        let path = Path::new(socket_path);

        // Socket path must be absolute
        if !path.is_absolute() {
            return Err(anyhow!("Socket path must be absolute: {}", socket_path));
        }

        // Check if parent directory exists and is writable
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                return Err(anyhow!("Socket directory does not exist: {}", parent.display()));
            }

            // Try to check write permissions
            match fs::metadata(parent) {
                Ok(metadata) => {
                    if metadata.permissions().readonly() {
                        return Err(anyhow!("Socket directory is not writable: {}", parent.display()));
                    }
                }
                Err(e) => {
                    return Err(anyhow!("Cannot access socket directory {}: {}", parent.display(), e));
                }
            }
        }

        Ok(())
    }
}

impl Default for ClientState {
    fn default() -> Self {
        // Use a fallback path if default_state_path fails
        let state_file = Self::default_state_path()
            .unwrap_or_else(|_| PathBuf::from(".kaiak_client.state"));

        Self {
            connection: None,
            state_file,
        }
    }
}

/// Validation function for ClientConnection
fn validate_client_connection(connection: &ClientConnection) -> Result<(), ValidationError> {
    // Socket path must be absolute
    if !connection.socket_path.starts_with('/') {
        return Err(ValidationError::new("Socket path must be absolute"));
    }

    // connected_at cannot be in future
    if connection.connected_at > Utc::now() {
        return Err(ValidationError::new("Connection time cannot be in future"));
    }

    // last_validated must be after connected_at
    if let Some(validated) = connection.last_validated {
        if validated < connection.connected_at {
            return Err(ValidationError::new("Validation time must be after connection time"));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_client_connection_creation() {
        let socket_path = "/tmp/test.sock".to_string();
        let connection = ClientConnection::new(socket_path.clone(), None).unwrap();

        assert_eq!(connection.socket_path, socket_path);
        assert!(connection.last_validated.is_none());
        assert!(connection.server_version.is_none());
    }

    #[test]
    fn test_client_connection_validation() {
        // Test invalid socket path (not absolute)
        let result = ClientConnection::new("test.sock".to_string(), None);
        assert!(result.is_err());

        // Test valid socket path
        let result = ClientConnection::new("/tmp/test.sock".to_string(), None);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_client_state_persistence() {
        let temp_dir = tempdir().unwrap();
        let state_file = temp_dir.path().join("client.state");

        let mut client_state = ClientState::with_state_file(state_file.clone());

        // Test save without connection
        client_state.save().unwrap();
        assert!(state_file.exists());

        // Test connect and save
        client_state.connect("/tmp/test.sock".to_string(), Some("1.0.0".to_string())).unwrap();
        assert!(client_state.is_connected());

        // Test load from file
        let loaded_state = ClientState::load_from_path(state_file).unwrap();
        assert!(loaded_state.is_connected());
        let connection = loaded_state.require_connection().unwrap();
        assert_eq!(connection.socket_path, "/tmp/test.sock");
        assert_eq!(connection.server_version, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_socket_path_validation() {
        // Test relative path (should fail)
        let result = ClientState::validate_socket_path("test.sock");
        assert!(result.is_err());

        // Test absolute path to non-existent directory (should fail)
        let result = ClientState::validate_socket_path("/non/existent/dir/test.sock");
        assert!(result.is_err());

        // Test absolute path to temp directory (should pass)
        let temp_dir = tempdir().unwrap();
        let socket_path = format!("{}/test.sock", temp_dir.path().display());
        let result = ClientState::validate_socket_path(&socket_path);
        assert!(result.is_ok());
    }
}
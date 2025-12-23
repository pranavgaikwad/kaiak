//! Security hardening for enterprise deployment

use anyhow::Result;
use std::path::Path;
use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
use tracing::{info, warn, error};

/// Security configuration and hardening for enterprise deployment
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Whether to require approval for all file modifications
    pub require_approval: bool,
    /// Timeout for approval requests in seconds
    pub approval_timeout: u32,
    /// Maximum file size for processing (bytes)
    pub max_file_size: u64,
    /// List of file patterns to exclude from processing
    pub excluded_patterns: Vec<String>,
    /// Socket file permissions (Unix only)
    pub socket_permissions: u32,
    /// Workspace validation rules
    pub workspace_validation: WorkspaceValidationConfig,
}

#[derive(Debug, Clone)]
pub struct WorkspaceValidationConfig {
    /// Whether to validate workspace paths are within allowed directories
    pub validate_workspace_paths: bool,
    /// List of allowed workspace root directories
    pub allowed_workspace_roots: Vec<String>,
    /// Whether to follow symbolic links
    pub follow_symlinks: bool,
    /// Maximum workspace depth for traversal
    pub max_workspace_depth: u32,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            require_approval: true,
            approval_timeout: 300, // 5 minutes
            max_file_size: 10 * 1024 * 1024, // 10MB
            excluded_patterns: vec![
                ".git/".to_string(),
                "node_modules/".to_string(),
                "target/".to_string(),
                "*.exe".to_string(),
                "*.so".to_string(),
                "*.dll".to_string(),
                "*.dylib".to_string(),
                ".env".to_string(),
                "*.key".to_string(),
                "*.pem".to_string(),
                "*.crt".to_string(),
                "*.p12".to_string(),
                "secrets.json".to_string(),
                "credentials.json".to_string(),
                "config/secrets/*".to_string(),
            ],
            socket_permissions: 0o600, // Read/write for owner only
            workspace_validation: WorkspaceValidationConfig {
                validate_workspace_paths: true,
                allowed_workspace_roots: vec![
                    "/home".to_string(),
                    "/Users".to_string(),
                    "/opt/workspaces".to_string(),
                    "/workspace".to_string(),
                ],
                follow_symlinks: false,
                max_workspace_depth: 20,
            },
        }
    }
}

impl SecurityConfig {
    /// Validate that a workspace path is allowed according to security policy
    pub fn validate_workspace_path<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();

        if !self.workspace_validation.validate_workspace_paths {
            return Ok(());
        }

        // Convert to canonical path
        let canonical_path = path.canonicalize()
            .map_err(|e| anyhow::anyhow!("Failed to resolve workspace path: {}", e))?;

        // Check if path is within allowed roots
        let path_str = canonical_path.to_string_lossy();
        let is_allowed = self.workspace_validation.allowed_workspace_roots
            .iter()
            .any(|root| path_str.starts_with(root));

        if !is_allowed {
            return Err(anyhow::anyhow!(
                "Workspace path '{}' is not within allowed directories: {:?}",
                path_str,
                self.workspace_validation.allowed_workspace_roots
            ));
        }

        info!("Workspace path validation passed: {}", path_str);
        Ok(())
    }

    /// Check if a file should be excluded from processing
    pub fn is_file_excluded<P: AsRef<Path>>(&self, file_path: P) -> bool {
        let path_str = file_path.as_ref().to_string_lossy();

        for pattern in &self.excluded_patterns {
            if path_str.contains(pattern) ||
               wildmatch::WildMatch::new(pattern).matches(&path_str) {
                warn!("File excluded by security policy: {} (pattern: {})", path_str, pattern);
                return true;
            }
        }

        false
    }

    /// Validate file size is within limits
    pub fn validate_file_size(&self, size: u64, file_path: &str) -> Result<()> {
        if size > self.max_file_size {
            return Err(anyhow::anyhow!(
                "File size {} bytes exceeds maximum allowed size {} bytes for file: {}",
                size, self.max_file_size, file_path
            ));
        }
        Ok(())
    }

    /// Set secure permissions on socket file (Unix only)
    pub fn secure_socket_file<P: AsRef<Path>>(&self, socket_path: P) -> Result<()> {
        #[cfg(unix)]
        {
            let path = socket_path.as_ref();
            let permissions = Permissions::from_mode(self.socket_permissions);
            std::fs::set_permissions(path, permissions)
                .map_err(|e| anyhow::anyhow!("Failed to set socket permissions: {}", e))?;

            info!("Set secure permissions on socket: {:?} (mode: {:o})",
                  path, self.socket_permissions);
        }

        #[cfg(not(unix))]
        {
            warn!("Socket permission setting not supported on this platform");
        }

        Ok(())
    }

    /// Sanitize file paths to prevent directory traversal attacks
    pub fn sanitize_file_path(&self, file_path: &str, workspace_root: &str) -> Result<String> {
        // Normalize the path
        let normalized = Path::new(file_path)
            .components()
            .filter_map(|component| {
                match component {
                    std::path::Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
                    std::path::Component::RootDir => None,
                    std::path::Component::CurDir => None,
                    std::path::Component::ParentDir => None,
                    _ => None,
                }
            })
            .collect::<Vec<String>>()
            .join("/");

        // Ensure the sanitized path is still within workspace
        let full_path = Path::new(workspace_root).join(&normalized);
        let canonical = full_path.canonicalize()
            .map_err(|e| anyhow::anyhow!("Invalid file path: {}", e))?;

        let workspace_canonical = Path::new(workspace_root).canonicalize()
            .map_err(|e| anyhow::anyhow!("Invalid workspace root: {}", e))?;

        if !canonical.starts_with(&workspace_canonical) {
            return Err(anyhow::anyhow!(
                "File path attempts to escape workspace: {}",
                file_path
            ));
        }

        Ok(normalized)
    }

    /// Validate API key format and basic security
    pub fn validate_api_key(&self, api_key: &str, provider: &str) -> Result<()> {
        if api_key.is_empty() {
            return Err(anyhow::anyhow!("Empty API key for provider: {}", provider));
        }

        if api_key.len() < 10 {
            return Err(anyhow::anyhow!("API key too short for provider: {}", provider));
        }

        // Basic pattern validation for known providers
        match provider.to_lowercase().as_str() {
            "openai" => {
                if !api_key.starts_with("sk-") && !api_key.starts_with("sk-proj-") {
                    return Err(anyhow::anyhow!("Invalid OpenAI API key format"));
                }
            }
            "anthropic" => {
                if !api_key.starts_with("sk-ant-") {
                    return Err(anyhow::anyhow!("Invalid Anthropic API key format"));
                }
            }
            _ => {
                // Generic validation for unknown providers
                if api_key.contains(" ") || api_key.contains("\n") || api_key.contains("\t") {
                    return Err(anyhow::anyhow!("API key contains invalid characters"));
                }
            }
        }

        Ok(())
    }

    /// Check for potentially dangerous file operations
    pub fn validate_file_operation(&self, operation: &str, file_path: &str) -> Result<()> {
        // Check for sensitive file patterns
        if self.is_file_excluded(file_path) {
            return Err(anyhow::anyhow!(
                "Operation '{}' not allowed on excluded file: {}",
                operation, file_path
            ));
        }

        // Validate operation type
        match operation.to_lowercase().as_str() {
            "read" | "write" | "edit" | "create" => Ok(()),
            "delete" | "move" | "rename" => {
                if !self.require_approval {
                    return Err(anyhow::anyhow!(
                        "Destructive operation '{}' requires approval to be enabled",
                        operation
                    ));
                }
                Ok(())
            }
            "execute" | "run" | "exec" => {
                Err(anyhow::anyhow!(
                    "Code execution operation '{}' is not allowed",
                    operation
                ))
            }
            _ => Err(anyhow::anyhow!(
                "Unknown file operation: {}",
                operation
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_file_exclusion() {
        let config = SecurityConfig::default();

        // Should exclude
        assert!(config.is_file_excluded(".git/config"));
        assert!(config.is_file_excluded("node_modules/package.json"));
        assert!(config.is_file_excluded("target/debug/app"));
        assert!(config.is_file_excluded("secrets.json"));
        assert!(config.is_file_excluded("app.exe"));

        // Should not exclude
        assert!(!config.is_file_excluded("src/main.rs"));
        assert!(!config.is_file_excluded("README.md"));
        assert!(!config.is_file_excluded("tests/integration.rs"));
    }

    #[test]
    fn test_file_size_validation() {
        let config = SecurityConfig::default();

        // Should pass
        assert!(config.validate_file_size(1024, "test.txt").is_ok());
        assert!(config.validate_file_size(1024 * 1024, "test.txt").is_ok());

        // Should fail
        assert!(config.validate_file_size(20 * 1024 * 1024, "test.txt").is_err());
    }

    #[test]
    fn test_path_sanitization() {
        let config = SecurityConfig::default();
        let temp_dir = tempdir().unwrap();
        let workspace = temp_dir.path().to_str().unwrap();

        // Should sanitize successfully
        assert!(config.sanitize_file_path("src/main.rs", workspace).is_ok());
        assert!(config.sanitize_file_path("./src/main.rs", workspace).is_ok());

        // Should reject traversal attempts
        assert!(config.sanitize_file_path("../../../etc/passwd", workspace).is_err());
        assert!(config.sanitize_file_path("/etc/passwd", workspace).is_err());
    }

    #[test]
    fn test_api_key_validation() {
        let config = SecurityConfig::default();

        // Valid keys
        assert!(config.validate_api_key("sk-1234567890abcdef", "openai").is_ok());
        assert!(config.validate_api_key("sk-ant-1234567890abcdef", "anthropic").is_ok());

        // Invalid keys
        assert!(config.validate_api_key("", "openai").is_err());
        assert!(config.validate_api_key("short", "openai").is_err());
        assert!(config.validate_api_key("invalid-format", "openai").is_err());
        assert!(config.validate_api_key("sk-1234567890abcdef", "anthropic").is_err()); // Wrong format for provider
    }

    #[test]
    fn test_file_operation_validation() {
        let config = SecurityConfig::default();

        // Safe operations
        assert!(config.validate_file_operation("read", "src/main.rs").is_ok());
        assert!(config.validate_file_operation("write", "src/main.rs").is_ok());

        // Destructive operations (require approval enabled)
        assert!(config.validate_file_operation("delete", "src/main.rs").is_ok());

        // Forbidden operations
        assert!(config.validate_file_operation("execute", "script.sh").is_err());

        // Excluded files
        assert!(config.validate_file_operation("read", ".env").is_err());
    }
}
//! Configuration validation for Kaiak server

use anyhow::Result;
use std::path::Path;
use tracing::{info, warn, error};
use crate::config::settings::Settings;
// Security config is imported via settings module

/// Comprehensive configuration validator
pub struct ConfigurationValidator {
    /// Whether to perform strict validation (fails on warnings)
    strict_mode: bool,
    /// List of validation warnings
    warnings: Vec<String>,
    /// List of validation errors
    errors: Vec<String>,
}

impl ConfigurationValidator {
    pub fn new(strict_mode: bool) -> Self {
        Self {
            strict_mode,
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Validate complete configuration
    pub fn validate_settings(&mut self, settings: &Settings) -> Result<()> {
        info!("Starting configuration validation");

        // Validate server configuration
        self.validate_server_config(&settings.server);

        // Validate AI configuration
        self.validate_ai_config(&settings.ai);

        // Validate workspace configuration
        self.validate_workspace_config(&settings.workspace);

        // Validate security configuration
        self.validate_security_config(&settings.security);

        // Validate performance configuration
        self.validate_performance_config(&settings.performance);

        // Check for environment variable requirements
        self.validate_environment_variables(settings);

        // Print summary
        self.print_validation_summary();

        // Return result based on errors and strict mode
        if !self.errors.is_empty() {
            return Err(anyhow::anyhow!(
                "Configuration validation failed with {} errors",
                self.errors.len()
            ));
        }

        if self.strict_mode && !self.warnings.is_empty() {
            return Err(anyhow::anyhow!(
                "Configuration validation failed in strict mode with {} warnings",
                self.warnings.len()
            ));
        }

        info!("Configuration validation passed");
        Ok(())
    }

    fn validate_server_config(&mut self, server_config: &crate::config::settings::ServerConfig) {
        // Validate transport
        match server_config.transport.as_str() {
            "stdio" => {
                info!("Transport: stdio (recommended)");
            }
            "socket" => {
                if server_config.socket_path.is_none() {
                    self.errors.push("Socket transport requires socket_path to be set".to_string());
                } else if let Some(path) = &server_config.socket_path {
                    if let Some(parent) = Path::new(path).parent() {
                        if !parent.exists() {
                            self.errors.push(format!("Socket path parent directory does not exist: {:?}", parent));
                        }
                    }
                }
            }
            other => {
                self.errors.push(format!("Invalid transport type: {}. Must be 'stdio' or 'socket'", other));
            }
        }

        // Validate log level
        match server_config.log_level.to_lowercase().as_str() {
            "trace" | "debug" | "info" | "warn" | "error" => {},
            other => {
                self.warnings.push(format!("Non-standard log level: {}. Recommended: trace, debug, info, warn, error", other));
            }
        }

        // Validate concurrent sessions limit
        if server_config.max_concurrent_sessions == 0 {
            self.errors.push("max_concurrent_sessions must be greater than 0".to_string());
        } else if server_config.max_concurrent_sessions > 100 {
            self.warnings.push(format!(
                "max_concurrent_sessions is very high ({}). This may impact performance",
                server_config.max_concurrent_sessions
            ));
        }
    }

    fn validate_ai_config(&mut self, ai_config: &crate::config::settings::AiConfig) {
        // Validate provider
        match ai_config.provider.as_str() {
            "openai" | "anthropic" => {},
            other => {
                self.warnings.push(format!("Untested AI provider: {}. Supported: openai, anthropic", other));
            }
        }

        // Validate model
        let known_models = [
            "gpt-4", "gpt-4-turbo", "gpt-3.5-turbo",
            "claude-3-opus", "claude-3-sonnet", "claude-3-haiku"
        ];

        if !known_models.contains(&ai_config.model.as_str()) {
            self.warnings.push(format!(
                "Unknown model: {}. Ensure it's supported by the provider",
                ai_config.model
            ));
        }

        // Validate timeout
        if ai_config.timeout < 30 {
            self.warnings.push("AI timeout is very short (< 30s). May cause premature failures".to_string());
        } else if ai_config.timeout > 600 {
            self.warnings.push("AI timeout is very long (> 10m). Consider reducing for better UX".to_string());
        }

        // Validate max_turns
        if ai_config.max_turns < 5 {
            self.warnings.push("max_turns is very low (< 5). May limit complex interactions".to_string());
        } else if ai_config.max_turns > 100 {
            self.warnings.push("max_turns is very high (> 100). May cause excessive API usage".to_string());
        }

        // Validate API keys
        self.validate_provider_config(&ai_config.providers.openai, "openai");
        self.validate_provider_config(&ai_config.providers.anthropic, "anthropic");

        // Ensure at least one provider has an API key
        let has_openai = ai_config.providers.openai.api_key.is_some();
        let has_anthropic = ai_config.providers.anthropic.api_key.is_some();

        if !has_openai && !has_anthropic {
            self.errors.push("At least one AI provider must have an API key configured".to_string());
        }
    }

    fn validate_provider_config(&mut self, provider: &crate::config::settings::ProviderConfig, name: &str) {
        if let Some(api_key) = &provider.api_key {
            if api_key.is_empty() {
                self.errors.push(format!("{} API key is empty", name));
            } else {
                // Basic format validation
                match name {
                    "openai" => {
                        if !api_key.starts_with("sk-") && !api_key.starts_with("sk-proj-") {
                            self.warnings.push("OpenAI API key format may be invalid (should start with 'sk-')".to_string());
                        }
                    }
                    "anthropic" => {
                        if !api_key.starts_with("sk-ant-") {
                            self.warnings.push("Anthropic API key format may be invalid (should start with 'sk-ant-')".to_string());
                        }
                    }
                    _ => {}
                }
            }
        }

        // Validate base URL if provided
        if let Some(base_url) = &provider.base_url {
            if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
                self.errors.push(format!("{} base_url must be a valid HTTP/HTTPS URL", name));
            }
        }
    }

    fn validate_workspace_config(&mut self, workspace_config: &crate::config::settings::WorkspaceConfig) {
        // Validate exclude patterns
        if workspace_config.exclude_patterns.is_empty() {
            self.warnings.push("No workspace exclude patterns defined. Consider adding patterns for build artifacts, dependencies, etc.".to_string());
        }

        // Validate essential exclude patterns
        let essential_patterns = ["target/", "node_modules/", ".git/"];
        for pattern in essential_patterns {
            if !workspace_config.exclude_patterns.contains(&pattern.to_string()) {
                self.warnings.push(format!("Consider adding '{}' to exclude patterns", pattern));
            }
        }

        // Validate max file size
        if workspace_config.max_file_size < 1024 {
            self.warnings.push("max_file_size is very small (< 1KB). May exclude too many files".to_string());
        } else if workspace_config.max_file_size > 50_000_000 {
            self.warnings.push("max_file_size is very large (> 50MB). May impact performance".to_string());
        }
    }

    fn validate_security_config(&mut self, security_config: &crate::config::settings::SecurityConfig) {
        // Validate approval timeout
        if security_config.approval_timeout < 30 {
            self.warnings.push("approval_timeout is very short (< 30s). Users may not have enough time to review".to_string());
        } else if security_config.approval_timeout > 1800 {
            self.warnings.push("approval_timeout is very long (> 30m). Consider shorter timeout for security".to_string());
        }

        // Recommend approval for production
        if !security_config.require_approval {
            self.warnings.push("require_approval is disabled. Enable for production environments".to_string());
        }
    }

    fn validate_performance_config(&mut self, performance_config: &crate::config::settings::PerformanceConfig) {
        // Validate stream buffer size
        if performance_config.stream_buffer_size < 100 {
            self.warnings.push("stream_buffer_size is very small (< 100). May impact streaming performance".to_string());
        } else if performance_config.stream_buffer_size > 10000 {
            self.warnings.push("stream_buffer_size is very large (> 10000). May use excessive memory".to_string());
        }

        // Validate session cache size
        if performance_config.session_cache_size < 10 {
            self.warnings.push("session_cache_size is very small (< 10). May reduce cache effectiveness".to_string());
        } else if performance_config.session_cache_size > 1000 {
            self.warnings.push("session_cache_size is very large (> 1000). May use excessive memory".to_string());
        }
    }

    fn validate_environment_variables(&mut self, _settings: &Settings) {
        // Check for required environment variables
        let required_env_vars = [
            ("OPENAI_API_KEY", "OpenAI API access"),
            ("ANTHROPIC_API_KEY", "Anthropic API access"),
        ];

        let mut has_any_key = false;

        for (env_var, description) in required_env_vars {
            if std::env::var(env_var).is_ok() {
                has_any_key = true;
                info!("Found environment variable: {}", env_var);
            }
        }

        if !has_any_key {
            self.errors.push("No AI provider API keys found in environment variables".to_string());
        }

        // Check for optional environment variables
        let optional_env_vars = [
            ("KAIAK_LOG_LEVEL", "Custom log level"),
            ("KAIAK_CONFIG_PATH", "Custom config file path"),
            ("KAIAK_WORKSPACE_ROOT", "Default workspace root"),
        ];

        for (env_var, description) in optional_env_vars {
            if std::env::var(env_var).is_ok() {
                info!("Found optional environment variable: {} ({})", env_var, description);
            }
        }
    }

    fn print_validation_summary(&self) {
        if !self.warnings.is_empty() {
            warn!("Configuration warnings ({}):", self.warnings.len());
            for (i, warning) in self.warnings.iter().enumerate() {
                warn!("  {}: {}", i + 1, warning);
            }
        }

        if !self.errors.is_empty() {
            error!("Configuration errors ({}):", self.errors.len());
            for (i, error) in self.errors.iter().enumerate() {
                error!("  {}: {}", i + 1, error);
            }
        }

        if self.warnings.is_empty() && self.errors.is_empty() {
            info!("Configuration validation completed successfully with no issues");
        } else {
            info!(
                "Configuration validation completed with {} warnings and {} errors",
                self.warnings.len(),
                self.errors.len()
            );
        }
    }

    /// Get validation warnings
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Get validation errors
    pub fn errors(&self) -> &[String] {
        &self.errors
    }
}

/// Quick validation function for use in main application
pub fn validate_configuration(settings: &Settings, strict: bool) -> Result<()> {
    let mut validator = ConfigurationValidator::new(strict);
    validator.validate_settings(settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::settings::*;

    fn create_test_settings() -> Settings {
        Settings {
            server: ServerConfig {
                transport: "stdio".to_string(),
                socket_path: Some("/tmp/kaiak.sock".to_string()),
                log_level: "info".to_string(),
                max_concurrent_sessions: 10,
            },
            ai: AiConfig {
                provider: "openai".to_string(),
                model: "gpt-4".to_string(),
                timeout: 300,
                max_turns: 50,
                providers: ProviderConfigs {
                    openai: ProviderConfig {
                        api_key: Some("sk-test1234567890".to_string()),
                        base_url: Some("https://api.openai.com/v1".to_string()),
                    },
                    anthropic: ProviderConfig {
                        api_key: None,
                        base_url: None,
                    },
                },
            },
            workspace: WorkspaceConfig {
                exclude_patterns: vec!["target/".to_string(), ".git/".to_string()],
                max_file_size: 1_048_576,
            },
            security: crate::config::settings::SecurityConfig {
                require_approval: true,
                approval_timeout: 300,
            },
            performance: PerformanceConfig {
                stream_buffer_size: 1000,
                session_cache_size: 100,
            },
        }
    }

    #[test]
    fn test_valid_configuration() {
        let settings = create_test_settings();
        let mut validator = ConfigurationValidator::new(false);

        let result = validator.validate_settings(&settings);
        assert!(result.is_ok(), "Valid configuration should pass validation");
    }

    #[test]
    fn test_invalid_transport() {
        let mut settings = create_test_settings();
        settings.server.transport = "invalid".to_string();

        let mut validator = ConfigurationValidator::new(false);
        let result = validator.validate_settings(&settings);

        assert!(result.is_err(), "Invalid transport should fail validation");
        assert!(!validator.errors.is_empty(), "Should have validation errors");
    }

    #[test]
    fn test_missing_api_key() {
        let mut settings = create_test_settings();
        settings.ai.providers.openai.api_key = None;

        let mut validator = ConfigurationValidator::new(false);
        let result = validator.validate_settings(&settings);

        assert!(result.is_err(), "Missing API key should fail validation");
    }

    #[test]
    fn test_strict_mode_warnings() {
        let mut settings = create_test_settings();
        settings.ai.model = "unknown-model".to_string(); // This should generate a warning

        let mut validator = ConfigurationValidator::new(true);
        let result = validator.validate_settings(&settings);

        assert!(result.is_err(), "Warnings in strict mode should fail validation");
        assert!(!validator.warnings.is_empty(), "Should have validation warnings");
    }

    #[test]
    fn test_lenient_mode_warnings() {
        let mut settings = create_test_settings();
        settings.ai.model = "unknown-model".to_string(); // This should generate a warning

        let mut validator = ConfigurationValidator::new(false);
        let result = validator.validate_settings(&settings);

        assert!(result.is_ok(), "Warnings in lenient mode should pass validation");
        assert!(!validator.warnings.is_empty(), "Should have validation warnings");
    }
}
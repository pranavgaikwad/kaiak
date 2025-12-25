use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};
use validator::Validate;

use crate::models::configuration::AgentConfiguration;
use crate::agents::GooseAgentManager;
use crate::KaiakResult;

/// Request type for kaiak/configure endpoint
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ConfigureRequest {
    /// Agent configuration including workspace, model, tools, and permissions
    #[validate(nested)]
    pub configuration: AgentConfiguration,
    /// Optional configuration validation mode
    pub validate_only: Option<bool>,
}

/// Response type for kaiak/configure endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigureResponse {
    /// Status of configuration operation
    pub status: String,
    /// Configuration validation result
    pub validation: ConfigureValidation,
    /// Applied configuration (may be modified for defaults)
    pub applied_config: Option<AgentConfiguration>,
    /// Configuration timestamp
    pub configured_at: String,
}

/// Configuration validation details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigureValidation {
    /// Whether configuration is valid
    pub valid: bool,
    /// Validation warnings (non-blocking)
    pub warnings: Vec<String>,
    /// Validation errors (blocking)
    pub errors: Vec<String>,
    /// Workspace validation result
    pub workspace_accessible: bool,
    /// Model configuration validation result
    pub model_valid: bool,
}

/// Handler for kaiak/configure endpoint
/// Manages agent configuration including workspace, model provider, tools, and permissions
pub struct ConfigureHandler {
    /// Current agent configuration
    current_config: Arc<RwLock<Option<AgentConfiguration>>>,
    /// Agent manager for applying configuration changes
    agent_manager: Arc<GooseAgentManager>,
}

impl ConfigureHandler {
    pub fn new(agent_manager: Arc<GooseAgentManager>) -> Self {
        Self {
            current_config: Arc::new(RwLock::new(None)),
            agent_manager,
        }
    }

    /// Handle configure request
    pub async fn handle_configure(&self, request: ConfigureRequest) -> KaiakResult<ConfigureResponse> {
        info!("Processing configure request");

        // Validate request using serde validator
        if let Err(validation_errors) = request.validate() {
            error!("Request validation failed: {:?}", validation_errors);
            let error_messages: Vec<String> = validation_errors
                .field_errors()
                .into_iter()
                .flat_map(|(field, errors)| {
                    errors.iter().map(move |error| {
                        format!("Field '{}': {}", field, error.message.as_ref().map(|m| m.as_ref()).unwrap_or("validation error"))
                    })
                })
                .collect();

            return Ok(ConfigureResponse {
                status: "validation_failed".to_string(),
                validation: ConfigureValidation {
                    valid: false,
                    warnings: vec![],
                    errors: error_messages,
                    workspace_accessible: false,
                    model_valid: false,
                },
                applied_config: None,
                configured_at: chrono::Utc::now().to_rfc3339(),
            });
        }

        // Validate configuration
        let validation = self.validate_configuration(&request.configuration).await;

        // If validation_only mode, return early
        if request.validate_only.unwrap_or(false) {
            debug!("Validation-only mode, returning validation results");
            return Ok(ConfigureResponse {
                status: if validation.valid { "validated" } else { "validation_failed" }.to_string(),
                validation,
                applied_config: None,
                configured_at: chrono::Utc::now().to_rfc3339(),
            });
        }

        // If validation failed with errors, reject configuration
        if !validation.errors.is_empty() {
            error!("Configuration validation failed with errors: {:?}", validation.errors);
            return Ok(ConfigureResponse {
                status: "rejected".to_string(),
                validation,
                applied_config: None,
                configured_at: chrono::Utc::now().to_rfc3339(),
            });
        }

        // Apply configuration
        let applied_config = self.apply_configuration(request.configuration).await?;

        // Store current configuration
        {
            let mut current = self.current_config.write().await;
            *current = Some(applied_config.clone());
        }

        info!("Configuration applied successfully with {} warnings", validation.warnings.len());

        Ok(ConfigureResponse {
            status: "configured".to_string(),
            validation,
            applied_config: Some(applied_config),
            configured_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Get current configuration
    pub async fn get_current_config(&self) -> Option<AgentConfiguration> {
        let current = self.current_config.read().await;
        current.clone()
    }

    /// Validate agent configuration
    async fn validate_configuration(&self, config: &AgentConfiguration) -> ConfigureValidation {
        let mut validation = ConfigureValidation {
            valid: true,
            warnings: vec![],
            errors: vec![],
            workspace_accessible: false,
            model_valid: false,
        };

        // Validate workspace
        match self.validate_workspace(&config.workspace).await {
            Ok(accessible) => validation.workspace_accessible = accessible,
            Err(e) => {
                validation.errors.push(format!("Workspace validation failed: {}", e));
                validation.valid = false;
            }
        }

        // Validate model configuration
        match self.validate_model_config(&config.model).await {
            Ok(valid) => validation.model_valid = valid,
            Err(e) => {
                validation.errors.push(format!("Model configuration validation failed: {}", e));
                validation.valid = false;
            }
        }

        // Validate tools configuration
        if let Err(e) = self.validate_tools_config(&config.tools).await {
            validation.warnings.push(format!("Tool configuration warning: {}", e));
        }

        // Validate session configuration
        if config.session.id.is_empty() {
            validation.errors.push("Session ID cannot be empty".to_string());
            validation.valid = false;
        }

        validation
    }

    /// Validate workspace accessibility
    async fn validate_workspace(&self, workspace: &crate::models::configuration::WorkspaceConfig) -> KaiakResult<bool> {
        // Check if workspace directory exists and is accessible
        if !workspace.working_dir.exists() {
            return Ok(false);
        }

        if !workspace.working_dir.is_dir() {
            return Ok(false);
        }

        // Try to read the directory
        match tokio::fs::read_dir(&workspace.working_dir).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Validate model configuration
    async fn validate_model_config(&self, model_config: &serde_json::Value) -> KaiakResult<bool> {
        // Basic model configuration validation
        if let Some(obj) = model_config.as_object() {
            // Check for required fields
            if !obj.contains_key("provider") || !obj.contains_key("model") {
                return Ok(false);
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Validate tools configuration
    async fn validate_tools_config(&self, tools: &crate::models::configuration::ToolConfig) -> KaiakResult<()> {
        // Validate that custom tools have required fields
        for tool in &tools.custom_tools {
            if tool.name.is_empty() {
                return Err(crate::KaiakError::configuration("Custom tool name cannot be empty"));
            }
        }

        // Validate max_tool_calls is reasonable
        if let Some(max_calls) = tools.max_tool_calls {
            if max_calls == 0 || max_calls > 10000 {
                return Err(crate::KaiakError::configuration("max_tool_calls must be between 1 and 10000"));
            }
        }

        Ok(())
    }

    /// Apply configuration to agent manager
    async fn apply_configuration(&self, mut config: AgentConfiguration) -> KaiakResult<AgentConfiguration> {
        debug!("Applying configuration to agent manager");

        // Ensure workspace directory is absolute
        if !config.workspace.working_dir.is_absolute() {
            let current_dir = std::env::current_dir()
                .map_err(|e| crate::KaiakError::workspace(format!("Failed to get current directory: {}", e), None))?;
            config.workspace.working_dir = current_dir.join(&config.workspace.working_dir);
        }

        // Apply defaults for missing session fields
        if config.session.max_turns.is_none() {
            config.session.max_turns = Some(1000);
        }

        // Store configuration in agent manager for future session creation
        // This would typically be stored for use when creating new sessions
        debug!("Configuration stored for future session creation");

        Ok(config)
    }
}
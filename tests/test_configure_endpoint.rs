use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;
use tempfile::TempDir;
use tokio::time::{timeout, Duration};
use uuid::Uuid;

use kaiak::config::init_test_logging;
use kaiak::server::{jsonrpc::JsonRpcServer, transport::StdioTransport};
mod common;
use common::{TestProvider, TestProviderMode};

/// Integration test for kaiak/configure endpoint
/// Tests the complete configuration workflow using TestProvider for AI responses
#[tokio::test]
async fn test_configure_endpoint_basic_workspace_setup() -> Result<()> {
    let _ = init_test_logging();

    // Initialize TestProvider for this test
    let mut test_provider = TestProvider::new("configure_basic_workspace")?;

    // Create temporary workspace
    let temp_dir = TempDir::new()?;
    let workspace_path = temp_dir.path().to_string_lossy().to_string();

    // Create sample project structure
    let src_dir = temp_dir.path().join("src");
    tokio::fs::create_dir_all(&src_dir).await?;
    tokio::fs::write(
        src_dir.join("main.rs"),
        "fn main() {\n    println!(\"Hello, world!\");\n}"
    ).await?;

    // Setup server components (mocked/test mode)
    let test_session_id = Uuid::new_v4().to_string();

    // Test 1: Basic configuration request
    let configure_request = json!({
        "jsonrpc": "2.0",
        "method": "workspace/executeCommand",
        "params": {
            "command": "kaiak/configure",
            "arguments": [{
                "configuration": {
                    "workspace": {
                        "working_dir": workspace_path,
                        "include_patterns": ["**/*.rs", "**/*.toml"],
                        "exclude_patterns": ["target/**", ".git/**"]
                    },
                    "model": {
                        "provider": "openai",
                        "model": "gpt-4",
                        "temperature": 0.1,
                        "max_tokens": 4096
                    },
                    "tools": {
                        "enabled_extensions": ["developer", "todo"],
                        "custom_tools": [],
                        "planning_mode": true,
                        "max_tool_calls": 10
                    },
                    "session": {
                        "max_turns": 1000,
                        "retry_config": null
                    },
                    "permissions": {
                        "tool_permissions": {
                            "read_file": "allow",
                            "write_file": "approve",
                            "shell_command": "deny",
                            "web_search": "allow"
                        }
                    }
                },
                "reset_existing": false
            }]
        },
        "id": 1
    });

    // Record/replay the configuration interaction
    let configure_result = test_provider.interact(
        "workspace_configuration",
        configure_request.clone()
    ).await?;

    // Test 2: Validate success response structure
    let response: Value = serde_json::from_value(configure_result)?;

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response.get("result").is_some(), "Configure should return result");

    if let Some(result) = response["result"].as_object() {
        assert_eq!(result["status"], "success");
        assert!(result.contains_key("configuration_applied"));
        assert!(result.contains_key("timestamp"));

        // Validate that configuration was applied correctly
        let config_applied = &result["configuration_applied"];
        assert!(config_applied["workspace"].is_object());
        assert!(config_applied["model"].is_object());
        assert!(config_applied["tools"].is_object());
        assert!(config_applied["permissions"].is_object());

        println!("✅ Configure endpoint returned valid success response");
    }

    // Test 3: Configuration with invalid workspace path
    let invalid_configure_request = json!({
        "jsonrpc": "2.0",
        "method": "workspace/executeCommand",
        "params": {
            "command": "kaiak/configure",
            "arguments": [{
                "configuration": {
                    "workspace": {
                        "working_dir": "/invalid/nonexistent/path",
                        "include_patterns": ["**/*.rs"],
                        "exclude_patterns": []
                    },
                    "model": {
                        "provider": "openai",
                        "model": "gpt-4"
                    }
                }
            }]
        },
        "id": 2
    });

    let error_result = test_provider.interact(
        "workspace_configuration_invalid_path",
        invalid_configure_request.clone()
    ).await?;

    let error_response: Value = serde_json::from_value(error_result)?;

    // Should return an error for invalid workspace
    assert_eq!(error_response["jsonrpc"], "2.0");
    assert_eq!(error_response["id"], 2);
    assert!(error_response.get("error").is_some(), "Invalid workspace should return error");

    if let Some(error) = error_response["error"].as_object() {
        assert!(error.contains_key("code"));
        assert!(error.contains_key("message"));

        // Should be a configuration error (code -32014)
        assert_eq!(error["code"], -32014);

        println!("✅ Configure endpoint properly handles invalid workspace");
    }

    // Test 4: Reset existing configuration
    let reset_configure_request = json!({
        "jsonrpc": "2.0",
        "method": "workspace/executeCommand",
        "params": {
            "command": "kaiak/configure",
            "arguments": [{
                "configuration": {
                    "workspace": {
                        "working_dir": workspace_path,
                        "include_patterns": ["**/*.py"],
                        "exclude_patterns": ["__pycache__/**"]
                    },
                    "model": {
                        "provider": "anthropic",
                        "model": "claude-3-opus",
                        "temperature": 0.7
                    },
                    "tools": {
                        "enabled_extensions": ["extensionmanager"],
                        "planning_mode": false
                    }
                },
                "reset_existing": true
            }]
        },
        "id": 3
    });

    let reset_result = test_provider.interact(
        "workspace_configuration_reset",
        reset_configure_request.clone()
    ).await?;

    let reset_response: Value = serde_json::from_value(reset_result)?;

    assert_eq!(reset_response["jsonrpc"], "2.0");
    assert_eq!(reset_response["id"], 3);
    assert!(reset_response.get("result").is_some());

    if let Some(result) = reset_response["result"].as_object() {
        assert_eq!(result["status"], "success");
        let config_applied = &result["configuration_applied"];

        // Verify the configuration was reset to new values
        assert_eq!(config_applied["model"]["provider"], "anthropic");
        assert_eq!(config_applied["model"]["model"], "claude-3-opus");
        assert_eq!(config_applied["workspace"]["include_patterns"][0], "**/*.py");

        println!("✅ Configure endpoint successfully resets existing configuration");
    }

    // Finalize test provider (saves recordings if in record mode)
    test_provider.finalize().await?;

    println!("🎯 Configure endpoint integration test completed successfully");
    Ok(())
}

/// Test advanced configuration scenarios
#[tokio::test]
async fn test_configure_endpoint_advanced_scenarios() -> Result<()> {
    let _ = init_test_logging();

    let mut test_provider = TestProvider::new("configure_advanced")?;

    let temp_dir = TempDir::new()?;
    let workspace_path = temp_dir.path().to_string_lossy().to_string();

    // Test 1: Configuration with custom tools
    let custom_tools_request = json!({
        "jsonrpc": "2.0",
        "method": "workspace/executeCommand",
        "params": {
            "command": "kaiak/configure",
            "arguments": [{
                "configuration": {
                    "workspace": {
                        "working_dir": workspace_path,
                        "include_patterns": ["**/*.java", "**/*.xml"],
                        "exclude_patterns": ["target/**", ".mvn/**"]
                    },
                    "model": {
                        "provider": "databricks",
                        "model": "databricks-meta-llama-3-1-405b-instruct"
                    },
                    "tools": {
                        "enabled_extensions": ["developer", "todo", "extensionmanager"],
                        "custom_tools": [
                            {
                                "name": "migration_helper",
                                "description": "Helps with Java migration tasks"
                            }
                        ],
                        "planning_mode": true,
                        "max_tool_calls": 20
                    },
                    "session": {
                        "max_turns": 500,
                        "retry_config": {
                            "max_retries": 3,
                            "backoff_multiplier": 2
                        }
                    },
                    "permissions": {
                        "tool_permissions": {
                            "read_file": "allow",
                            "write_file": "approve",
                            "shell_command": "approve",
                            "web_search": "deny",
                            "migration_helper": "allow"
                        }
                    }
                }
            }]
        },
        "id": 1
    });

    let custom_result = test_provider.interact(
        "configuration_with_custom_tools",
        custom_tools_request
    ).await?;

    let response: Value = serde_json::from_value(custom_result)?;

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response.get("result").is_some());

    if let Some(result) = response["result"].as_object() {
        assert_eq!(result["status"], "success");
        let config = &result["configuration_applied"];

        // Verify custom tools were configured
        assert!(config["tools"]["custom_tools"].is_array());
        assert_eq!(config["tools"]["custom_tools"][0]["name"], "migration_helper");

        // Verify retry configuration
        assert!(config["session"]["retry_config"].is_object());
        assert_eq!(config["session"]["retry_config"]["max_retries"], 3);

        // Verify custom tool permissions
        assert_eq!(config["permissions"]["tool_permissions"]["migration_helper"], "allow");

        println!("✅ Advanced configuration with custom tools successful");
    }

    // Test 2: Minimal configuration (only required fields)
    let minimal_request = json!({
        "jsonrpc": "2.0",
        "method": "workspace/executeCommand",
        "params": {
            "command": "kaiak/configure",
            "arguments": [{
                "configuration": {
                    "workspace": {
                        "working_dir": workspace_path
                    },
                    "model": {
                        "provider": "openai",
                        "model": "gpt-3.5-turbo"
                    }
                }
            }]
        },
        "id": 2
    });

    let minimal_result = test_provider.interact(
        "configuration_minimal",
        minimal_request
    ).await?;

    let minimal_response: Value = serde_json::from_value(minimal_result)?;

    assert_eq!(minimal_response["jsonrpc"], "2.0");
    assert!(minimal_response.get("result").is_some());

    if let Some(result) = minimal_response["result"].as_object() {
        assert_eq!(result["status"], "success");
        let config = &result["configuration_applied"];

        // Should have default values for missing optional fields
        assert!(config["tools"].is_object());
        assert!(config["permissions"].is_object());

        println!("✅ Minimal configuration with defaults successful");
    }

    test_provider.finalize().await?;

    println!("🎯 Advanced configure endpoint scenarios completed successfully");
    Ok(())
}

/// Test configuration validation and error handling
#[tokio::test]
async fn test_configure_endpoint_validation_errors() -> Result<()> {
    let _ = init_test_logging();

    let mut test_provider = TestProvider::new("configure_validation")?;

    // Test 1: Missing required fields
    let missing_fields_request = json!({
        "jsonrpc": "2.0",
        "method": "workspace/executeCommand",
        "params": {
            "command": "kaiak/configure",
            "arguments": [{
                "configuration": {
                    "workspace": {
                        // Missing working_dir
                        "include_patterns": ["**/*.rs"]
                    }
                    // Missing model section entirely
                }
            }]
        },
        "id": 1
    });

    let error_result = test_provider.interact(
        "configuration_missing_fields",
        missing_fields_request
    ).await?;

    let error_response: Value = serde_json::from_value(error_result)?;

    assert_eq!(error_response["jsonrpc"], "2.0");
    assert!(error_response.get("error").is_some());
    assert_eq!(error_response["error"]["code"], -32602); // Invalid params

    // Test 2: Invalid provider
    let invalid_provider_request = json!({
        "jsonrpc": "2.0",
        "method": "workspace/executeCommand",
        "params": {
            "command": "kaiak/configure",
            "arguments": [{
                "configuration": {
                    "workspace": {
                        "working_dir": "/tmp"
                    },
                    "model": {
                        "provider": "invalid_provider",
                        "model": "some-model"
                    }
                }
            }]
        },
        "id": 2
    });

    let provider_error_result = test_provider.interact(
        "configuration_invalid_provider",
        invalid_provider_request
    ).await?;

    let provider_error_response: Value = serde_json::from_value(provider_error_result)?;

    assert_eq!(provider_error_response["jsonrpc"], "2.0");
    assert!(provider_error_response.get("error").is_some());
    assert_eq!(provider_error_response["error"]["code"], -32014); // Configuration error

    test_provider.finalize().await?;

    println!("🎯 Configuration validation and error handling completed successfully");
    Ok(())
}
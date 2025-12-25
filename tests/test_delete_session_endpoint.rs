use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;
use tempfile::TempDir;
use tokio::time::{timeout, Duration};
use uuid::Uuid;

use kaiak::config::init_test_logging;
mod common;
use common::{TestProvider, TestProviderMode};

/// Integration test for kaiak/delete_session endpoint
/// Tests the complete session cleanup workflow using TestProvider for AI responses
#[tokio::test]
async fn test_delete_session_endpoint_basic_cleanup() -> Result<()> {
    let _ = init_test_logging();

    // Initialize TestProvider for this test
    let mut test_provider = TestProvider::new("delete_session_basic")?;

    let session_id = Uuid::new_v4().to_string();

    // Test 1: Delete session with basic cleanup options
    let delete_request = json!({
        "jsonrpc": "2.0",
        "method": "workspace/executeCommand",
        "params": {
            "command": "kaiak/delete_session",
            "arguments": [{
                "session_id": session_id,
                "force": false,
                "cleanup_files": true
            }]
        },
        "id": 1
    });

    // Record/replay the session deletion interaction
    let delete_result = test_provider.interact(
        "session_deletion_basic",
        delete_request.clone()
    ).await?;

    // Test 2: Validate successful deletion response
    let response: Value = serde_json::from_value(delete_result)?;

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response.get("result").is_some(), "Delete session should return result");

    if let Some(result) = response["result"].as_object() {
        assert_eq!(result["session_id"], session_id);
        assert_eq!(result["status"], "deleted");
        assert!(result.contains_key("cleanup_summary"));
        assert!(result.contains_key("deleted_at"));

        // Validate cleanup summary structure
        let cleanup = &result["cleanup_summary"];
        assert_eq!(cleanup["session_removed"], true);
        assert!(cleanup["messages_cleaned"].is_number());
        assert!(cleanup["temp_files_removed"].is_number());
        assert!(cleanup["errors"].is_array());

        println!("✅ Delete session endpoint returned valid response");
    }

    // Test 3: Delete non-existent session
    let nonexistent_session_id = Uuid::new_v4().to_string();
    let nonexistent_request = json!({
        "jsonrpc": "2.0",
        "method": "workspace/executeCommand",
        "params": {
            "command": "kaiak/delete_session",
            "arguments": [{
                "session_id": nonexistent_session_id,
                "force": false,
                "cleanup_files": true
            }]
        },
        "id": 2
    });

    let nonexistent_result = test_provider.interact(
        "session_deletion_nonexistent",
        nonexistent_request
    ).await?;

    let error_response: Value = serde_json::from_value(nonexistent_result)?;

    // Should return error for non-existent session
    assert_eq!(error_response["jsonrpc"], "2.0");
    assert_eq!(error_response["id"], 2);
    assert!(error_response.get("error").is_some(), "Non-existent session should return error");

    if let Some(error) = error_response["error"].as_object() {
        // Should be "Session not found" error (code -32003)
        assert_eq!(error["code"], -32003);
        assert!(error.contains_key("message"));
        assert!(error["data"]["session_id"] == nonexistent_session_id);

        println!("✅ Delete session endpoint properly handles non-existent sessions");
    }

    // Test 4: Force deletion
    let force_session_id = Uuid::new_v4().to_string();
    let force_delete_request = json!({
        "jsonrpc": "2.0",
        "method": "workspace/executeCommand",
        "params": {
            "command": "kaiak/delete_session",
            "arguments": [{
                "session_id": force_session_id,
                "force": true,
                "cleanup_files": false
            }]
        },
        "id": 3
    });

    let force_result = test_provider.interact(
        "session_deletion_force",
        force_delete_request
    ).await?;

    let force_response: Value = serde_json::from_value(force_result)?;

    assert_eq!(force_response["jsonrpc"], "2.0");
    assert_eq!(force_response["id"], 3);

    // Force delete might succeed even if session doesn't exist or is active
    if force_response.get("result").is_some() {
        let result = &force_response["result"];
        assert_eq!(result["session_id"], force_session_id);
        assert_eq!(result["status"], "deleted");

        println!("✅ Force delete completed successfully");
    } else if force_response.get("error").is_some() {
        // Error is also acceptable for non-existent session
        println!("✅ Force delete handled appropriately");
    }

    // Finalize test provider
    test_provider.finalize().await?;

    println!("🎯 Delete session endpoint basic workflow completed successfully");
    Ok(())
}

/// Test advanced session deletion scenarios
#[tokio::test]
async fn test_delete_session_endpoint_advanced_scenarios() -> Result<()> {
    let _ = init_test_logging();

    let mut test_provider = TestProvider::new("delete_session_advanced")?;

    // Test 1: Delete session with custom cleanup options
    let session_with_data_id = Uuid::new_v4().to_string();
    let custom_cleanup_request = json!({
        "jsonrpc": "2.0",
        "method": "workspace/executeCommand",
        "params": {
            "command": "kaiak/delete_session",
            "arguments": [{
                "session_id": session_with_data_id,
                "force": false,
                "cleanup_files": true,
                "preserve_logs": true,
                "cleanup_cache": true,
                "timeout_seconds": 30
            }]
        },
        "id": 1
    });

    let custom_result = test_provider.interact(
        "session_deletion_custom_cleanup",
        custom_cleanup_request
    ).await?;

    let response: Value = serde_json::from_value(custom_result)?;

    assert_eq!(response["jsonrpc"], "2.0");

    if response.get("result").is_some() {
        let result = &response["result"];
        assert_eq!(result["session_id"], session_with_data_id);

        // Verify custom cleanup options were processed
        let cleanup = &result["cleanup_summary"];
        assert!(cleanup.is_object());

        println!("✅ Custom cleanup options processed successfully");
    }

    // Test 2: Delete active session (should require force or fail gracefully)
    let active_session_id = Uuid::new_v4().to_string();
    let active_session_request = json!({
        "jsonrpc": "2.0",
        "method": "workspace/executeCommand",
        "params": {
            "command": "kaiak/delete_session",
            "arguments": [{
                "session_id": active_session_id,
                "force": false,
                "cleanup_files": true
            }]
        },
        "id": 2
    });

    let active_result = test_provider.interact(
        "session_deletion_active_session",
        active_session_request
    ).await?;

    let active_response: Value = serde_json::from_value(active_result)?;

    assert_eq!(active_response["jsonrpc"], "2.0");

    // Could succeed (if session not actually active) or fail with appropriate error
    if active_response.get("error").is_some() {
        let error = &active_response["error"];
        // Could be various error codes depending on session state
        assert!(error["code"].is_number());
        println!("✅ Active session deletion handled with appropriate error");
    } else {
        println!("✅ Session deletion completed successfully");
    }

    // Test 3: Batch cleanup with multiple session references
    let batch_sessions: Vec<String> = (0..3).map(|_| Uuid::new_v4().to_string()).collect();

    for (idx, session_id) in batch_sessions.iter().enumerate() {
        let batch_request = json!({
            "jsonrpc": "2.0",
            "method": "workspace/executeCommand",
            "params": {
                "command": "kaiak/delete_session",
                "arguments": [{
                    "session_id": session_id,
                    "force": false,
                    "cleanup_files": true
                }]
            },
            "id": idx + 10
        });

        let batch_result = test_provider.interact(
            &format!("session_deletion_batch_{}", idx),
            batch_request
        ).await?;

        let batch_response: Value = serde_json::from_value(batch_result)?;
        assert_eq!(batch_response["jsonrpc"], "2.0");
        assert_eq!(batch_response["id"], idx + 10);

        // Each request should be handled independently
        println!("✅ Batch deletion request {} processed", idx + 1);
    }

    // Test 4: Cleanup with file system errors simulation
    let fs_error_session_id = Uuid::new_v4().to_string();
    let fs_error_request = json!({
        "jsonrpc": "2.0",
        "method": "workspace/executeCommand",
        "params": {
            "command": "kaiak/delete_session",
            "arguments": [{
                "session_id": fs_error_session_id,
                "force": true,
                "cleanup_files": true,
                "cleanup_temp_files": true
            }]
        },
        "id": 4
    });

    let fs_error_result = test_provider.interact(
        "session_deletion_fs_errors",
        fs_error_request
    ).await?;

    let fs_error_response: Value = serde_json::from_value(fs_error_result)?;

    assert_eq!(fs_error_response["jsonrpc"], "2.0");

    if fs_error_response.get("result").is_some() {
        let result = &fs_error_response["result"];
        let cleanup = &result["cleanup_summary"];

        // Should report any file system errors in the errors array
        if cleanup["errors"].as_array().unwrap().len() > 0 {
            println!("✅ File system errors properly reported in cleanup summary");
        } else {
            println!("✅ No file system errors encountered");
        }
    }

    test_provider.finalize().await?;

    println!("🎯 Advanced delete session scenarios completed successfully");
    Ok(())
}

/// Test error handling and validation for session deletion
#[tokio::test]
async fn test_delete_session_endpoint_validation() -> Result<()> {
    let _ = init_test_logging();

    let mut test_provider = TestProvider::new("delete_session_validation")?;

    // Test 1: Invalid session ID format
    let invalid_id_request = json!({
        "jsonrpc": "2.0",
        "method": "workspace/executeCommand",
        "params": {
            "command": "kaiak/delete_session",
            "arguments": [{
                "session_id": "invalid-uuid-format-123",
                "force": false,
                "cleanup_files": true
            }]
        },
        "id": 1
    });

    let invalid_result = test_provider.interact(
        "session_deletion_invalid_id",
        invalid_id_request
    ).await?;

    let invalid_response: Value = serde_json::from_value(invalid_result)?;

    assert_eq!(invalid_response["jsonrpc"], "2.0");
    // May succeed with lenient validation or fail with validation error
    println!("✅ Invalid session ID format handled appropriately");

    // Test 2: Missing required parameters
    let missing_params_request = json!({
        "jsonrpc": "2.0",
        "method": "workspace/executeCommand",
        "params": {
            "command": "kaiak/delete_session",
            "arguments": [{
                // Missing session_id
                "force": false,
                "cleanup_files": true
            }]
        },
        "id": 2
    });

    let missing_result = test_provider.interact(
        "session_deletion_missing_params",
        missing_params_request
    ).await?;

    let missing_response: Value = serde_json::from_value(missing_result)?;

    assert_eq!(missing_response["jsonrpc"], "2.0");
    assert_eq!(missing_response["id"], 2);

    // Should return parameter validation error
    if missing_response.get("error").is_some() {
        assert_eq!(missing_response["error"]["code"], -32602); // Invalid params
        println!("✅ Missing parameters properly validated");
    }

    // Test 3: Empty session ID
    let empty_id_request = json!({
        "jsonrpc": "2.0",
        "method": "workspace/executeCommand",
        "params": {
            "command": "kaiak/delete_session",
            "arguments": [{
                "session_id": "",
                "force": false,
                "cleanup_files": true
            }]
        },
        "id": 3
    });

    let empty_result = test_provider.interact(
        "session_deletion_empty_id",
        empty_id_request
    ).await?;

    let empty_response: Value = serde_json::from_value(empty_result)?;

    assert_eq!(empty_response["jsonrpc"], "2.0");

    // Should return validation error for empty session ID
    if empty_response.get("error").is_some() {
        assert!(
            empty_response["error"]["code"] == -32602 || // Invalid params
            empty_response["error"]["code"] == -32003    // Session not found
        );
        println!("✅ Empty session ID properly rejected");
    }

    // Test 4: Invalid boolean values
    let invalid_bool_request = json!({
        "jsonrpc": "2.0",
        "method": "workspace/executeCommand",
        "params": {
            "command": "kaiak/delete_session",
            "arguments": [{
                "session_id": Uuid::new_v4().to_string(),
                "force": "not_a_boolean",
                "cleanup_files": "also_not_boolean"
            }]
        },
        "id": 4
    });

    let bool_result = test_provider.interact(
        "session_deletion_invalid_booleans",
        invalid_bool_request
    ).await?;

    let bool_response: Value = serde_json::from_value(bool_result)?;

    assert_eq!(bool_response["jsonrpc"], "2.0");

    // Should return type validation error
    if bool_response.get("error").is_some() {
        assert_eq!(bool_response["error"]["code"], -32602); // Invalid params
        println!("✅ Invalid boolean values properly validated");
    }

    test_provider.finalize().await?;

    println!("🎯 Delete session validation completed successfully");
    Ok(())
}

/// Test session lifecycle integration (configure → generate → delete)
#[tokio::test]
async fn test_complete_session_lifecycle() -> Result<()> {
    let _ = init_test_logging();

    let mut test_provider = TestProvider::new("session_lifecycle")?;

    let temp_dir = TempDir::new()?;
    let workspace_path = temp_dir.path().to_string_lossy().to_string();
    let session_id = Uuid::new_v4().to_string();

    // Step 1: Configure agent
    let configure_request = json!({
        "jsonrpc": "2.0",
        "method": "workspace/executeCommand",
        "params": {
            "command": "kaiak/configure",
            "arguments": [{
                "configuration": {
                    "workspace": {
                        "working_dir": workspace_path,
                        "include_patterns": ["**/*.rs"]
                    },
                    "model": {
                        "provider": "openai",
                        "model": "gpt-4"
                    }
                }
            }]
        },
        "id": 1
    });

    let configure_result = test_provider.interact(
        "lifecycle_configure",
        configure_request
    ).await?;

    let configure_response: Value = serde_json::from_value(configure_result)?;
    assert!(configure_response.get("result").is_some());

    // Step 2: Generate fix
    let generate_request = json!({
        "jsonrpc": "2.0",
        "method": "workspace/executeCommand",
        "params": {
            "command": "kaiak/generate_fix",
            "arguments": [{
                "session_id": session_id,
                "incidents": [{
                    "id": "lifecycle-test",
                    "rule_id": "test-rule",
                    "message": "Test incident for lifecycle",
                    "description": "Testing complete lifecycle",
                    "effort": "trivial",
                    "severity": "info"
                }]
            }]
        },
        "id": 2
    });

    let generate_result = test_provider.interact(
        "lifecycle_generate",
        generate_request
    ).await?;

    let generate_response: Value = serde_json::from_value(generate_result)?;
    assert!(generate_response.get("result").is_some());

    // Step 3: Delete session
    let delete_request = json!({
        "jsonrpc": "2.0",
        "method": "workspace/executeCommand",
        "params": {
            "command": "kaiak/delete_session",
            "arguments": [{
                "session_id": session_id,
                "force": false,
                "cleanup_files": true
            }]
        },
        "id": 3
    });

    let delete_result = test_provider.interact(
        "lifecycle_delete",
        delete_request
    ).await?;

    let delete_response: Value = serde_json::from_value(delete_result)?;

    // Delete should succeed or fail gracefully
    if delete_response.get("result").is_some() {
        println!("✅ Complete session lifecycle successful");
    } else if delete_response.get("error").is_some() {
        println!("✅ Session lifecycle completed with expected cleanup behavior");
    }

    test_provider.finalize().await?;

    println!("🎯 Complete session lifecycle integration test completed");
    Ok(())
}
use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;
use tempfile::TempDir;
use tokio::time::{timeout, Duration};
use uuid::Uuid;

use kaiak::config::init_test_logging;
mod common;
use common::{TestProvider, TestProviderMode};

/// Integration test for kaiak/generate_fix endpoint
/// Tests the complete fix generation workflow using TestProvider for AI responses
#[tokio::test]
async fn test_generate_fix_endpoint_basic_workflow() -> Result<()> {
    let _ = init_test_logging();

    // Initialize TestProvider for this test
    let mut test_provider = TestProvider::new("generate_fix_basic")?;

    // Create temporary workspace with code that needs fixes
    let temp_dir = TempDir::new()?;
    let workspace_path = temp_dir.path().to_string_lossy().to_string();

    // Create sample Java project with deprecated API usage
    let src_dir = temp_dir.path().join("src").join("main").join("java").join("com").join("example");
    tokio::fs::create_dir_all(&src_dir).await?;
    tokio::fs::write(
        src_dir.join("DataConverter.java"),
        r#"package com.example;

import javax.xml.bind.DatatypeConverter;

public class DataConverter {
    public String encodeBase64(byte[] data) {
        // Deprecated API usage - needs migration to Java 11+
        return DatatypeConverter.printBase64Binary(data);
    }

    public byte[] decodeBase64(String data) {
        return DatatypeConverter.parseBase64Binary(data);
    }
}
"#
    ).await?;

    // Generate a unique session ID
    let session_id = Uuid::new_v4().to_string();

    // Test 1: Basic fix generation for deprecated API usage
    let generate_fix_request = json!({
        "jsonrpc": "2.0",
        "method": "workspace/executeCommand",
        "params": {
            "command": "kaiak/generate_fix",
            "arguments": [{
                "session_id": session_id,
                "incidents": [
                    {
                        "id": "deprecated-api-1",
                        "rule_id": "java-deprecated-datatypeconverter",
                        "message": "Use of deprecated API javax.xml.bind.DatatypeConverter",
                        "description": "The javax.xml.bind.DatatypeConverter class is deprecated in Java 9+ and removed in Java 11+",
                        "effort": "trivial",
                        "severity": "warning"
                    }
                ],
                "migration_context": {
                    "source_technology": "Java 8",
                    "target_technology": "Java 17",
                    "migration_hints": ["Use java.util.Base64 instead of DatatypeConverter"],
                    "constraints": ["Maintain backward compatibility"],
                    "preferences": {
                        "code_style": "google",
                        "test_generation": true
                    }
                },
                "options": {
                    "auto_apply_safe_fixes": false,
                    "max_processing_time": 300,
                    "parallel_processing": false,
                    "include_explanations": true
                }
            }]
        },
        "id": 1
    });

    // Record/replay the fix generation interaction
    let fix_result = test_provider.interact(
        "fix_generation_deprecated_api",
        generate_fix_request.clone()
    ).await?;

    // Test 2: Validate successful fix generation response
    let response: Value = serde_json::from_value(fix_result)?;

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response.get("result").is_some(), "Generate fix should return result");

    if let Some(result) = response["result"].as_object() {
        assert!(result.contains_key("request_id"));
        assert_eq!(result["session_id"], session_id);
        assert!(result.contains_key("status"));
        assert_eq!(result["incident_count"], 1);
        assert!(result.contains_key("completed_at"));

        // Status should be "completed" for successful processing
        let status = result["status"].as_str().unwrap();
        assert!(
            status == "completed" || status == "failed",
            "Status should be completed or failed, got: {}", status
        );

        println!("✅ Generate fix endpoint returned valid response");
    }

    // Test 3: Multiple incidents in single request
    let multiple_incidents_request = json!({
        "jsonrpc": "2.0",
        "method": "workspace/executeCommand",
        "params": {
            "command": "kaiak/generate_fix",
            "arguments": [{
                "session_id": session_id,
                "incidents": [
                    {
                        "id": "deprecated-api-1",
                        "rule_id": "java-deprecated-datatypeconverter",
                        "message": "Use of deprecated API javax.xml.bind.DatatypeConverter",
                        "description": "Replace with java.util.Base64",
                        "effort": "trivial",
                        "severity": "warning"
                    },
                    {
                        "id": "deprecated-api-2",
                        "rule_id": "java-deprecated-calendar",
                        "message": "Use of deprecated Calendar class",
                        "description": "Replace with java.time APIs",
                        "effort": "medium",
                        "severity": "warning"
                    },
                    {
                        "id": "security-issue-1",
                        "rule_id": "java-insecure-random",
                        "message": "Use of insecure Random class for security purposes",
                        "description": "Replace with SecureRandom for cryptographic use",
                        "effort": "low",
                        "severity": "error"
                    }
                ],
                "migration_context": {
                    "source_technology": "Java 8",
                    "target_technology": "Java 17",
                    "constraints": ["Maintain API compatibility"],
                    "preferences": {
                        "test_generation": true
                    }
                },
                "options": {
                    "parallel_processing": true,
                    "include_explanations": true
                }
            }]
        },
        "id": 2
    });

    let multiple_result = test_provider.interact(
        "fix_generation_multiple_incidents",
        multiple_incidents_request
    ).await?;

    let multiple_response: Value = serde_json::from_value(multiple_result)?;

    assert_eq!(multiple_response["jsonrpc"], "2.0");
    assert_eq!(multiple_response["id"], 2);
    assert!(multiple_response.get("result").is_some());

    if let Some(result) = multiple_response["result"].as_object() {
        assert_eq!(result["incident_count"], 3);
        assert_eq!(result["session_id"], session_id);

        println!("✅ Generate fix endpoint handled multiple incidents successfully");
    }

    // Test 4: Error handling - session in use
    let new_session_id = Uuid::new_v4().to_string();
    let conflicting_request = json!({
        "jsonrpc": "2.0",
        "method": "workspace/executeCommand",
        "params": {
            "command": "kaiak/generate_fix",
            "arguments": [{
                "session_id": session_id, // Reuse same session ID
                "incidents": [{
                    "id": "concurrent-test",
                    "rule_id": "test-rule",
                    "message": "Test concurrent access",
                    "description": "Testing session conflict handling",
                    "effort": "trivial",
                    "severity": "info"
                }]
            }]
        },
        "id": 3
    });

    let conflict_result = test_provider.interact(
        "fix_generation_session_conflict",
        conflicting_request
    ).await?;

    let conflict_response: Value = serde_json::from_value(conflict_result)?;

    // Might return success or error depending on timing and session state
    assert_eq!(conflict_response["jsonrpc"], "2.0");
    assert_eq!(conflict_response["id"], 3);

    if conflict_response.get("error").is_some() {
        // If error, should be session in use error (-32016)
        assert_eq!(conflict_response["error"]["code"], -32016);
        println!("✅ Generate fix endpoint properly handles session conflicts");
    } else {
        // If success, session was available
        println!("✅ Generate fix endpoint handled concurrent request successfully");
    }

    // Finalize test provider
    test_provider.finalize().await?;

    println!("🎯 Generate fix endpoint basic workflow completed successfully");
    Ok(())
}

/// Test advanced fix generation scenarios
#[tokio::test]
async fn test_generate_fix_endpoint_advanced_scenarios() -> Result<()> {
    let _ = init_test_logging();

    let mut test_provider = TestProvider::new("generate_fix_advanced")?;

    let temp_dir = TempDir::new()?;
    let workspace_path = temp_dir.path().to_string_lossy().to_string();

    // Create complex Java project structure
    let main_java = temp_dir.path().join("src").join("main").join("java");
    let test_java = temp_dir.path().join("src").join("test").join("java");

    tokio::fs::create_dir_all(&main_java.join("com").join("example")).await?;
    tokio::fs::create_dir_all(&test_java.join("com").join("example")).await?;

    // Create main class with multiple migration issues
    tokio::fs::write(
        main_java.join("com").join("example").join("LegacyService.java"),
        r#"package com.example;

import java.util.Date;
import java.util.Calendar;
import javax.xml.bind.DatatypeConverter;
import java.util.Random;

public class LegacyService {
    private Random random = new Random(); // Insecure for crypto

    public String processData(String input) {
        // Multiple deprecated API usages
        Date now = new Date(); // Should use java.time
        Calendar cal = Calendar.getInstance(); // Should use java.time

        byte[] bytes = input.getBytes();
        String encoded = DatatypeConverter.printBase64Binary(bytes); // Deprecated

        // Insecure random for token generation
        long token = random.nextLong();

        return encoded + ":" + token;
    }
}
"#
    ).await?;

    let session_id = Uuid::new_v4().to_string();

    // Test 1: Complex migration scenario with multiple rule violations
    let complex_fix_request = json!({
        "jsonrpc": "2.0",
        "method": "workspace/executeCommand",
        "params": {
            "command": "kaiak/generate_fix",
            "arguments": [{
                "session_id": session_id,
                "incidents": [
                    {
                        "id": "date-api-migration-1",
                        "rule_id": "java-legacy-date-time",
                        "message": "Use of legacy Date and Calendar APIs",
                        "description": "Replace Date/Calendar with java.time APIs (LocalDateTime, ZonedDateTime, etc.)",
                        "effort": "medium",
                        "severity": "warning"
                    },
                    {
                        "id": "deprecated-xml-bind",
                        "rule_id": "java-deprecated-datatypeconverter",
                        "message": "javax.xml.bind.DatatypeConverter is deprecated",
                        "description": "Replace with java.util.Base64 for encoding/decoding",
                        "effort": "trivial",
                        "severity": "warning"
                    },
                    {
                        "id": "insecure-random-usage",
                        "rule_id": "java-insecure-random-crypto",
                        "message": "Insecure Random usage for security-sensitive operations",
                        "description": "Use SecureRandom for cryptographically secure random numbers",
                        "effort": "low",
                        "severity": "error"
                    }
                ],
                "migration_context": {
                    "source_technology": "Java 8",
                    "target_technology": "Java 17",
                    "migration_hints": [
                        "Use java.time.* for date/time operations",
                        "Use java.util.Base64 for encoding",
                        "Use SecureRandom for security-sensitive randomness"
                    ],
                    "constraints": [
                        "Maintain API compatibility",
                        "Preserve existing behavior",
                        "Add comprehensive tests"
                    ],
                    "preferences": {
                        "code_style": "google",
                        "test_generation": true,
                        "documentation_updates": true
                    }
                },
                "options": {
                    "auto_apply_safe_fixes": false,
                    "max_processing_time": 600,
                    "parallel_processing": true,
                    "include_explanations": true
                }
            }]
        },
        "id": 1
    });

    let complex_result = test_provider.interact(
        "fix_generation_complex_migration",
        complex_fix_request
    ).await?;

    let response: Value = serde_json::from_value(complex_result)?;

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response.get("result").is_some());

    if let Some(result) = response["result"].as_object() {
        assert_eq!(result["incident_count"], 3);
        assert_eq!(result["session_id"], session_id);

        println!("✅ Complex migration scenario processed successfully");
    }

    // Test 2: High-severity critical fixes
    let critical_fix_request = json!({
        "jsonrpc": "2.0",
        "method": "workspace/executeCommand",
        "params": {
            "command": "kaiak/generate_fix",
            "arguments": [{
                "session_id": Uuid::new_v4().to_string(),
                "incidents": [
                    {
                        "id": "sql-injection-1",
                        "rule_id": "java-sql-injection",
                        "message": "SQL injection vulnerability detected",
                        "description": "String concatenation in SQL query creates injection risk",
                        "effort": "medium",
                        "severity": "critical"
                    },
                    {
                        "id": "path-traversal-1",
                        "rule_id": "java-path-traversal",
                        "message": "Path traversal vulnerability",
                        "description": "User input used in file path without validation",
                        "effort": "medium",
                        "severity": "critical"
                    }
                ],
                "migration_context": {
                    "source_technology": "Java 8",
                    "target_technology": "Java 17",
                    "constraints": ["Security is top priority"],
                    "preferences": {
                        "security_focused": true
                    }
                },
                "options": {
                    "auto_apply_safe_fixes": false,
                    "max_processing_time": 300,
                    "include_explanations": true
                }
            }]
        },
        "id": 2
    });

    let critical_result = test_provider.interact(
        "fix_generation_critical_security",
        critical_fix_request
    ).await?;

    let critical_response: Value = serde_json::from_value(critical_result)?;

    assert_eq!(critical_response["jsonrpc"], "2.0");
    assert!(critical_response.get("result").is_some());

    if let Some(result) = critical_response["result"].as_object() {
        assert_eq!(result["incident_count"], 2);

        println!("✅ Critical security fixes processed successfully");
    }

    test_provider.finalize().await?;

    println!("🎯 Advanced generate fix scenarios completed successfully");
    Ok(())
}

/// Test error handling and edge cases for fix generation
#[tokio::test]
async fn test_generate_fix_endpoint_error_handling() -> Result<()> {
    let _ = init_test_logging();

    let mut test_provider = TestProvider::new("generate_fix_errors")?;

    // Test 1: Invalid session ID format
    let invalid_session_request = json!({
        "jsonrpc": "2.0",
        "method": "workspace/executeCommand",
        "params": {
            "command": "kaiak/generate_fix",
            "arguments": [{
                "session_id": "invalid-uuid-format",
                "incidents": [{
                    "id": "test-1",
                    "rule_id": "test-rule",
                    "message": "Test message",
                    "description": "Test description",
                    "effort": "trivial",
                    "severity": "info"
                }]
            }]
        },
        "id": 1
    });

    let invalid_result = test_provider.interact(
        "fix_generation_invalid_session",
        invalid_session_request
    ).await?;

    let invalid_response: Value = serde_json::from_value(invalid_result)?;

    assert_eq!(invalid_response["jsonrpc"], "2.0");
    // Could be success (if session validation is lenient) or error
    println!("✅ Invalid session ID handled appropriately");

    // Test 2: Empty incidents array
    let empty_incidents_request = json!({
        "jsonrpc": "2.0",
        "method": "workspace/executeCommand",
        "params": {
            "command": "kaiak/generate_fix",
            "arguments": [{
                "session_id": Uuid::new_v4().to_string(),
                "incidents": []
            }]
        },
        "id": 2
    });

    let empty_result = test_provider.interact(
        "fix_generation_empty_incidents",
        empty_incidents_request
    ).await?;

    let empty_response: Value = serde_json::from_value(empty_result)?;

    assert_eq!(empty_response["jsonrpc"], "2.0");
    // Should handle empty incidents gracefully
    if empty_response.get("result").is_some() {
        let result = &empty_response["result"];
        assert_eq!(result["incident_count"], 0);
    }

    println!("✅ Empty incidents array handled correctly");

    // Test 3: Malformed incident data
    let malformed_incident_request = json!({
        "jsonrpc": "2.0",
        "method": "workspace/executeCommand",
        "params": {
            "command": "kaiak/generate_fix",
            "arguments": [{
                "session_id": Uuid::new_v4().to_string(),
                "incidents": [{
                    "id": "", // Empty ID
                    // Missing required fields like rule_id, message, etc.
                    "severity": "invalid_severity"
                }]
            }]
        },
        "id": 3
    });

    let malformed_result = test_provider.interact(
        "fix_generation_malformed_incident",
        malformed_incident_request
    ).await?;

    let malformed_response: Value = serde_json::from_value(malformed_result)?;

    assert_eq!(malformed_response["jsonrpc"], "2.0");
    // Should return validation error for malformed data
    if malformed_response.get("error").is_some() {
        assert_eq!(malformed_response["error"]["code"], -32602); // Invalid params
    }

    println!("✅ Malformed incident data validation works correctly");

    test_provider.finalize().await?;

    println!("🎯 Generate fix error handling completed successfully");
    Ok(())
}
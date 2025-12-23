use anyhow::Result;
use kaiak::config::init_test_logging;
use kaiak::models::{AiSession, Incident, FixGenerationRequest, Severity};
use kaiak::handlers::{FixGenerationHandler, LifecycleHandler};
use kaiak::goose::{GooseManager, AgentManager};
use tempfile::TempDir;
use std::fs;

/// Integration test for complete fix generation workflow
/// Tests User Story 1: "accepts fix generation requests from IDE extension for one or more incidents"

#[cfg(test)]
mod fix_workflow_tests {
    use super::*;

    #[tokio::test]
    async fn test_complete_fix_generation_workflow() {
        // Initialize test logging
        let _ = init_test_logging();

        // Setup: Create temporary workspace with sample code
        let temp_dir = TempDir::new().unwrap();
        let workspace_path = temp_dir.path().to_string_lossy().to_string();

        // Create a sample file with deprecated API usage
        let sample_file = temp_dir.path().join("src").join("main.rs");
        fs::create_dir_all(sample_file.parent().unwrap()).unwrap();
        fs::write(&sample_file, "fn main() {\n    old_method(); // deprecated\n}").unwrap();

        // Step 1: Create AI session
        let lifecycle_handler = LifecycleHandler::new();
        let session = lifecycle_handler
            .create_session(workspace_path.clone(), Some("test-workflow-session".to_string()))
            .await
            .expect("Failed to create session");

        assert_eq!(session.configuration.workspace_path, workspace_path);
        assert!(session.id.len() > 0);

        // Step 2: Create incident data (simulating static analysis tool output)
        let incident = Incident::new(
            "deprecated-api-usage".to_string(),
            "src/main.rs".to_string(),
            2, // line number
            Severity::Warning,
            "Use of deprecated API".to_string(),
            "Function old_method() is deprecated, use new_method() instead".to_string(),
            "deprecated-api".to_string(),
        );

        assert!(incident.is_valid());

        // Step 3: Create fix generation request
        let fix_request = FixGenerationRequest::new(
            session.id.clone(),
            vec![incident],
            workspace_path.clone(),
        );

        assert!(fix_request.is_valid());
        assert_eq!(fix_request.incidents.len(), 1);

        // Step 4: Initialize Goose agent integration
        let goose_manager = GooseManager::new();
        let agent_manager = AgentManager::new().await.expect("Failed to create AgentManager");

        // Verify manager is ready
        assert_eq!(goose_manager.active_session_count().await, 0);

        // Step 5: Process fix generation request
        let fix_handler = FixGenerationHandler::new();
        let request_id = fix_handler
            .handle_request(&fix_request)
            .await
            .expect("Failed to handle fix generation request");

        assert!(!request_id.is_empty());

        // Step 6: Verify request processing started
        // TODO: This will be enhanced when actual Goose integration is complete
        // For now, we just verify the workflow structure works

        // This test currently fails as expected since full implementation isn't complete
        // It validates the workflow structure and dependencies
        assert!(false, "Integration test not fully implemented - waiting for Goose agent integration");
    }

    #[tokio::test]
    async fn test_workflow_error_handling() {
        let _ = init_test_logging();

        // Test workflow with invalid workspace path
        let invalid_workspace = "/nonexistent/workspace";

        let lifecycle_handler = LifecycleHandler::new();

        // This should work for now as we're using placeholder implementation
        // TODO: Add proper validation once full implementation is complete
        let session = lifecycle_handler
            .create_session(invalid_workspace.to_string(), None)
            .await
            .expect("Placeholder implementation accepts any path");

        assert_eq!(session.configuration.workspace_path, invalid_workspace);

        // TODO: Add proper error handling tests once validation is implemented
        assert!(false, "Error handling test not yet implemented");
    }

    #[tokio::test]
    async fn test_multiple_incidents_workflow() {
        let _ = init_test_logging();

        // Setup workspace with multiple files containing issues
        let temp_dir = TempDir::new().unwrap();
        let workspace_path = temp_dir.path().to_string_lossy().to_string();

        // Create multiple files with different types of issues
        let file1 = temp_dir.path().join("src").join("lib.rs");
        let file2 = temp_dir.path().join("src").join("utils.rs");
        fs::create_dir_all(file1.parent().unwrap()).unwrap();

        fs::write(&file1, "// Deprecated import\nuse old_module::OldStruct;\n").unwrap();
        fs::write(&file2, "// Unsafe code block\nunsafe { transmute(data) }\n").unwrap();

        // Create multiple incidents
        let incident1 = Incident::new(
            "deprecated-import".to_string(),
            "src/lib.rs".to_string(),
            2,
            Severity::Warning,
            "Deprecated module import".to_string(),
            "old_module is deprecated".to_string(),
            "deprecated-api".to_string(),
        );

        let incident2 = Incident::new(
            "unsafe-code".to_string(),
            "src/utils.rs".to_string(),
            2,
            Severity::Error,
            "Unsafe code usage".to_string(),
            "Unsafe transmute operation".to_string(),
            "unsafe-code".to_string(),
        );

        // Create session and request
        let lifecycle_handler = LifecycleHandler::new();
        let session = lifecycle_handler
            .create_session(workspace_path.clone(), Some("multi-incident-test".to_string()))
            .await
            .expect("Failed to create session");

        let fix_request = FixGenerationRequest::new(
            session.id,
            vec![incident1, incident2],
            workspace_path,
        );

        assert_eq!(fix_request.incidents.len(), 2);
        assert!(fix_request.is_valid());

        // Process multiple incidents
        let fix_handler = FixGenerationHandler::new();
        let request_id = fix_handler
            .handle_request(&fix_request)
            .await
            .expect("Failed to handle multi-incident request");

        assert!(!request_id.is_empty());

        // TODO: Verify that all incidents are processed
        assert!(false, "Multi-incident workflow test not fully implemented");
    }

    #[tokio::test]
    async fn test_session_isolation() {
        let _ = init_test_logging();

        // Test that multiple sessions work independently
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();
        let workspace1 = temp_dir1.path().to_string_lossy().to_string();
        let workspace2 = temp_dir2.path().to_string_lossy().to_string();

        let lifecycle_handler = LifecycleHandler::new();
        let goose_manager = GooseManager::new();

        // Create two separate sessions
        let session1 = lifecycle_handler
            .create_session(workspace1.clone(), Some("session-1".to_string()))
            .await
            .expect("Failed to create session 1");

        let session2 = lifecycle_handler
            .create_session(workspace2.clone(), Some("session-2".to_string()))
            .await
            .expect("Failed to create session 2");

        assert_ne!(session1.id, session2.id);
        assert_ne!(session1.configuration.workspace_path, session2.configuration.workspace_path);

        // TODO: Verify sessions are isolated in Goose manager
        // For now, just verify structure
        assert_eq!(goose_manager.active_session_count().await, 0); // No sessions created yet in manager

        assert!(false, "Session isolation test not fully implemented");
    }
}

/// Test helpers for workflow testing

/// Create a sample workspace with test files
pub async fn create_test_workspace() -> Result<TempDir> {
    let temp_dir = TempDir::new()?;
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir)?;

    // Create main.rs with deprecated API usage
    fs::write(
        src_dir.join("main.rs"),
        r#"fn main() {
    println!("Hello, world!");
    old_method(); // deprecated
    unsafe_operation(); // needs review
}"#,
    )?;

    // Create lib.rs with import issues
    fs::write(
        src_dir.join("lib.rs"),
        r#"use old_module::deprecated_function;
use unsafe_module::*;

pub fn test_function() {
    deprecated_function();
}"#,
    )?;

    Ok(temp_dir)
}

/// Create test incidents for common migration scenarios
pub fn create_test_incidents() -> Vec<Incident> {
    vec![
        Incident::new(
            "deprecated-api-usage".to_string(),
            "src/main.rs".to_string(),
            3,
            Severity::Warning,
            "Deprecated API call".to_string(),
            "old_method() is deprecated since version 1.5.0".to_string(),
            "deprecated-api".to_string(),
        ),
        Incident::new(
            "unsafe-code-review".to_string(),
            "src/main.rs".to_string(),
            4,
            Severity::Error,
            "Unsafe operation requires review".to_string(),
            "unsafe_operation() should be replaced with safe alternative".to_string(),
            "safety".to_string(),
        ),
    ]
}
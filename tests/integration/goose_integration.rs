use anyhow::Result;
use kaiak::config::init_test_logging;
use kaiak::models::AiSession;
use kaiak::goose::{GooseManager, GooseSessionWrapper, AgentManager};
use tempfile::TempDir;

/// Integration tests for Goose agent initialization and lifecycle
/// Tests User Story 1: "runs the Goose AI agent with customized prompts and/or tools"

#[cfg(test)]
mod goose_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_goose_agent_manager_initialization() {
        let _ = init_test_logging();

        // Test that AgentManager can be created and initialized
        let agent_manager = AgentManager::new().await;
        assert!(agent_manager.is_ok(), "AgentManager creation should succeed");

        let manager = agent_manager.unwrap();
        // TODO: Once Goose integration is complete, test actual agent initialization

        // For now, just verify the manager structure exists
        assert!(false, "Goose agent manager test not fully implemented - waiting for Goose integration");
    }

    #[tokio::test]
    async fn test_goose_session_wrapper_lifecycle() {
        let _ = init_test_logging();

        let temp_dir = TempDir::new().unwrap();
        let workspace_path = temp_dir.path().to_string_lossy().to_string();

        // Create an AI session for testing
        let ai_session = AiSession::new(workspace_path, Some("test-goose-session".to_string()));

        // Test GooseSessionWrapper creation
        let session_wrapper = GooseSessionWrapper::new(&ai_session).await;
        assert!(session_wrapper.is_ok(), "Session wrapper creation should succeed");

        let mut wrapper = session_wrapper.unwrap();
        assert_eq!(wrapper.session_id, ai_session.id);
        assert!(!wrapper.is_ready(), "New session should not be ready initially");

        // Test session initialization
        let init_result = wrapper.initialize().await;
        assert!(init_result.is_ok(), "Session initialization should succeed");
        assert!(wrapper.is_ready(), "Initialized session should be ready");

        // Test session cleanup
        let cleanup_result = wrapper.cleanup().await;
        assert!(cleanup_result.is_ok(), "Session cleanup should succeed");

        // TODO: Test actual Goose session integration once available
        assert!(false, "Goose session wrapper test not fully implemented");
    }

    #[tokio::test]
    async fn test_goose_manager_session_management() {
        let _ = init_test_logging();

        let goose_manager = GooseManager::new();
        assert_eq!(goose_manager.active_session_count().await, 0);

        let temp_dir = TempDir::new().unwrap();
        let workspace_path = temp_dir.path().to_string_lossy().to_string();

        let ai_session = AiSession::new(workspace_path, Some("test-manager-session".to_string()));

        // Test session creation through manager
        let session_result = goose_manager
            .get_or_create_session(ai_session.id.clone(), &ai_session)
            .await;

        assert!(session_result.is_ok(), "Session creation through manager should succeed");

        // Verify session count increased
        assert_eq!(goose_manager.active_session_count().await, 1);

        // Test getting existing session
        let existing_session = goose_manager
            .get_or_create_session(ai_session.id.clone(), &ai_session)
            .await;

        assert!(existing_session.is_ok(), "Getting existing session should succeed");
        assert_eq!(goose_manager.active_session_count().await, 1); // Should not create duplicate

        // Test session removal
        let remove_result = goose_manager.remove_session(&ai_session.id).await;
        assert!(remove_result.is_ok(), "Session removal should succeed");
        assert_eq!(goose_manager.active_session_count().await, 0);

        // TODO: Test actual Goose session integration
        assert!(false, "Goose manager test not fully implemented");
    }

    #[tokio::test]
    async fn test_agent_fix_request_processing() {
        let _ = init_test_logging();

        let agent_manager = AgentManager::new().await.unwrap();

        // Create a test fix generation request
        let temp_dir = TempDir::new().unwrap();
        let workspace_path = temp_dir.path().to_string_lossy().to_string();

        let ai_session = AiSession::new(workspace_path.clone(), Some("test-agent-processing".to_string()));

        let incident = kaiak::models::Incident::new(
            "test-rule".to_string(),
            "src/main.rs".to_string(),
            42,
            kaiak::models::Severity::Warning,
            "Test incident".to_string(),
            "This is a test incident for agent processing".to_string(),
            "test".to_string(),
        );

        let fix_request = kaiak::models::FixGenerationRequest::new(
            ai_session.id,
            vec![incident],
            workspace_path,
        );

        // Test that agent can process fix request
        let processing_result = agent_manager.process_fix_request(&fix_request).await;
        assert!(processing_result.is_ok(), "Agent should be able to process fix requests");

        let request_id = processing_result.unwrap();
        assert!(!request_id.is_empty(), "Processing should return a request ID");

        // Test request status checking
        let status_result = agent_manager.get_request_status(&request_id).await;
        assert!(status_result.is_ok(), "Should be able to get request status");

        let status = status_result.unwrap();
        assert!(!status.is_empty(), "Status should not be empty");

        // TODO: Test actual Goose agent processing once integration is complete
        assert!(false, "Agent processing test not fully implemented");
    }

    #[tokio::test]
    async fn test_concurrent_session_handling() {
        let _ = init_test_logging();

        let goose_manager = GooseManager::new();
        let mut session_ids = Vec::new();

        // Create multiple sessions concurrently
        for i in 0..5 {
            let temp_dir = TempDir::new().unwrap();
            let workspace_path = temp_dir.path().to_string_lossy().to_string();

            let ai_session = AiSession::new(
                workspace_path,
                Some(format!("concurrent-session-{}", i)),
            );

            let session_result = goose_manager
                .get_or_create_session(ai_session.id.clone(), &ai_session)
                .await;

            assert!(session_result.is_ok(), "Concurrent session creation should succeed");
            session_ids.push(ai_session.id);
        }

        // Verify all sessions were created
        assert_eq!(goose_manager.active_session_count().await, 5);

        // Cleanup all sessions
        for session_id in session_ids {
            let remove_result = goose_manager.remove_session(&session_id).await;
            assert!(remove_result.is_ok(), "Session cleanup should succeed");
        }

        assert_eq!(goose_manager.active_session_count().await, 0);

        // TODO: Test actual concurrent Goose session handling
        assert!(false, "Concurrent session test not fully implemented");
    }

    #[tokio::test]
    async fn test_agent_prompt_integration() {
        let _ = init_test_logging();

        // Test that prompt building works with Goose agent integration
        use kaiak::goose::PromptBuilder;

        let system_prompt = PromptBuilder::system_prompt();
        assert!(system_prompt.contains("migration assistant"));
        assert!(system_prompt.len() > 100); // Should be substantial

        // Create test request for prompt generation
        let temp_dir = TempDir::new().unwrap();
        let workspace_path = temp_dir.path().to_string_lossy().to_string();

        let ai_session = AiSession::new(workspace_path.clone(), Some("prompt-test".to_string()));

        let incident = kaiak::models::Incident::new(
            "deprecated-api".to_string(),
            "src/main.rs".to_string(),
            42,
            kaiak::models::Severity::Warning,
            "Deprecated API usage".to_string(),
            "old_method() is deprecated".to_string(),
            "deprecated".to_string(),
        );

        let fix_request = kaiak::models::FixGenerationRequest::new(
            ai_session.id,
            vec![incident],
            workspace_path,
        );

        let fix_prompt = PromptBuilder::fix_generation_prompt(&fix_request);
        assert!(fix_prompt.contains("src/main.rs"));
        assert!(fix_prompt.contains("deprecated-api"));
        assert!(fix_prompt.contains("line 42"));

        // TODO: Test actual prompt integration with Goose agent
        assert!(false, "Prompt integration test not fully implemented");
    }
}

/// Test utilities for Goose integration testing

/// Create a mock incident for testing
pub fn create_mock_incident(rule_id: &str, file_path: &str, line: u32) -> kaiak::models::Incident {
    kaiak::models::Incident::new(
        rule_id.to_string(),
        file_path.to_string(),
        line,
        kaiak::models::Severity::Warning,
        format!("Mock incident for {}", rule_id),
        format!("This is a mock incident for testing {}", rule_id),
        "mock".to_string(),
    )
}

/// Create a test workspace structure for Goose testing
pub async fn create_goose_test_workspace() -> Result<TempDir> {
    let temp_dir = TempDir::new()?;
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir_all(&src_dir)?;

    // Create files that would benefit from AI assistance
    std::fs::write(
        src_dir.join("legacy.rs"),
        r#"// This file has deprecated API usage that needs migration
use old_crate::DeprecatedStruct;
use std::mem::transmute;

fn legacy_function() {
    let data = DeprecatedStruct::old_constructor();
    let raw_ptr = unsafe { transmute(data) };
    // More legacy code that needs migration...
}"#,
    )?;

    std::fs::write(
        src_dir.join("modern.rs"),
        r#"// This file shows what the migrated code should look like
use new_crate::ModernStruct;

fn modern_function() {
    let data = ModernStruct::new();
    let safe_reference = data.as_ref();
    // Modern, safe code patterns
}"#,
    )?;

    Ok(temp_dir)
}
// T050: Integration test for error recovery and graceful degradation
// Tests system behavior under error conditions and recovery mechanisms
use kaiak::{
    handlers::{
        lifecycle::LifecycleHandler,
        fix_generation::FixGenerationHandler,
        streaming::StreamingHandler,
        interactions::InteractionHandler,
    },
    models::{
        session::{Session, SessionStatus},
        request::FixGenerationRequest,
        incident::Incident,
        messages::{Id, StreamMessage, StreamMessageType},
        interaction::{UserInteraction, InteractionType, InteractionStatus},
    },
    goose::{SessionManager, agent::AgentManager},
    config::Settings,
    error::{KaiakError, Result},
};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::time::{sleep, Duration, timeout};
use uuid::Uuid;

/// Test system recovery from agent initialization failures
#[tokio::test]
async fn test_agent_initialization_failure_recovery() {
    // T050: Test recovery when Goose agent fails to initialize

    let temp_dir = TempDir::new().unwrap();
    let workspace_path = "/nonexistent/path"; // Invalid path to trigger failure

    let settings = Settings::new().unwrap();
    let agent_manager = AgentManager::new().await.unwrap();
    let session_manager = SessionManager::new(agent_manager.clone()).await;
    let lifecycle_handler = LifecycleHandler::new(session_manager.clone()).await;

    let session_id = Id::new();
    let session = Session {
        id: session_id.clone(),
        goose_session_id: Uuid::new_v4().to_string(),
        status: SessionStatus::Created,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        configuration: Default::default(),
        active_request_id: None,
        message_count: 0,
        error_count: 0,
    };

    // Create session
    lifecycle_handler.create_session(session).await.unwrap();

    // Try to initialize with invalid workspace - should fail gracefully
    let init_result = lifecycle_handler.initialize_session(&session_id, workspace_path).await;
    assert!(init_result.is_err(), "Initialization should fail with invalid workspace");

    // Session should be in error state, not crashed
    let status = lifecycle_handler.get_session_status(&session_id).await.unwrap();
    assert_eq!(status.status, SessionStatus::Error);
    assert!(status.error_count > 0);

    // Test recovery - reinitialize with valid workspace
    let valid_workspace = temp_dir.path().to_string_lossy().to_string();
    let recovery_result = lifecycle_handler.restart_session(&session_id).await.unwrap();
    let reinit_result = lifecycle_handler.initialize_session(&session_id, &valid_workspace).await;
    assert!(reinit_result.is_ok(), "Recovery should succeed with valid workspace");

    // Session should be ready after recovery
    let recovered_status = lifecycle_handler.get_session_status(&session_id).await.unwrap();
    assert_eq!(recovered_status.status, SessionStatus::Ready);

    // Clean up
    lifecycle_handler.terminate_session(&session_id).await.unwrap();
}

/// Test graceful degradation when streaming fails
#[tokio::test]
async fn test_streaming_failure_degradation() {
    // T050: Test system behavior when streaming components fail

    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path().to_string_lossy().to_string();

    let settings = Settings::new().unwrap();
    let agent_manager = AgentManager::new().await.unwrap();
    let session_manager = SessionManager::new(agent_manager.clone()).await;
    let lifecycle_handler = LifecycleHandler::new(session_manager.clone()).await;
    let streaming_handler = StreamingHandler::new().await;

    let session_id = Id::new();
    let session = Session {
        id: session_id.clone(),
        goose_session_id: Uuid::new_v4().to_string(),
        status: SessionStatus::Created,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        configuration: Default::default(),
        active_request_id: None,
        message_count: 0,
        error_count: 0,
    };

    // Create and initialize session
    lifecycle_handler.create_session(session).await.unwrap();
    lifecycle_handler.initialize_session(&session_id, &workspace_path).await.unwrap();

    // Create fix generation handler
    let fix_handler = FixGenerationHandler::new(
        session_manager.clone(),
        agent_manager.clone()
    ).await;

    // Test streaming failure - attempt to send to non-existent stream
    let test_message = StreamMessage {
        id: Id::new(),
        session_id: session_id.clone(),
        message_type: StreamMessageType::Progress,
        timestamp: chrono::Utc::now(),
        content: serde_json::json!({
            "percentage": 50,
            "phase": "testing",
            "description": "Testing streaming failure"
        }),
        metadata: None,
    };

    // This should fail gracefully without crashing the session
    let stream_result = streaming_handler.send_message(&session_id, test_message).await;
    assert!(stream_result.is_err(), "Streaming to non-existent stream should fail");

    // Session should remain operational despite streaming failure
    let status = lifecycle_handler.get_session_status(&session_id).await.unwrap();
    assert_ne!(status.status, SessionStatus::Error); // Should not be in error state
    assert!(status.status == SessionStatus::Ready || status.status == SessionStatus::Processing);

    // System should continue to function for other operations
    let health_check = lifecycle_handler.check_session_health(&session_id).await;
    assert!(health_check.is_ok(), "Health check should still work");

    // Clean up
    lifecycle_handler.terminate_session(&session_id).await.unwrap();
}

/// Test timeout handling and recovery
#[tokio::test]
async fn test_timeout_handling_recovery() {
    // T050: Test system behavior with operation timeouts

    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path().to_string_lossy().to_string();

    let settings = Settings::new().unwrap();
    let agent_manager = AgentManager::new().await.unwrap();
    let session_manager = SessionManager::new(agent_manager.clone()).await;
    let lifecycle_handler = LifecycleHandler::new(session_manager.clone()).await;
    let interaction_handler = InteractionHandler::new(session_manager.clone()).await;

    let session_id = Id::new();
    let session = Session {
        id: session_id.clone(),
        goose_session_id: Uuid::new_v4().to_string(),
        status: SessionStatus::Created,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        configuration: Default::default(),
        active_request_id: None,
        message_count: 0,
        error_count: 0,
    };

    // Create and initialize session
    lifecycle_handler.create_session(session).await.unwrap();
    lifecycle_handler.initialize_session(&session_id, &workspace_path).await.unwrap();

    // Create user interaction with short timeout
    let interaction = UserInteraction {
        id: Id::new(),
        session_id: session_id.clone(),
        interaction_type: InteractionType::Approval,
        prompt: "Test timeout scenario".to_string(),
        request_data: serde_json::json!({"default_choice": false}),
        response_data: None,
        status: InteractionStatus::Pending,
        timeout_seconds: 1, // Very short timeout
        created_at: chrono::Utc::now(),
        responded_at: None,
    };

    let create_result = interaction_handler.create_interaction(interaction.clone()).await;
    assert!(create_result.is_ok());

    // Wait for timeout to occur
    sleep(Duration::from_secs(2)).await;

    // Check that interaction was handled gracefully
    let expired_interaction = interaction_handler.get_interaction(&interaction.id).await;
    assert!(
        expired_interaction.is_err() ||
        expired_interaction.unwrap().status == InteractionStatus::Timeout,
        "Interaction should be expired or removed"
    );

    // System should remain operational after timeout
    let status = lifecycle_handler.get_session_status(&session_id).await.unwrap();
    assert_ne!(status.status, SessionStatus::Error);

    // Clean up
    lifecycle_handler.terminate_session(&session_id).await.unwrap();
}

/// Test concurrent error handling
#[tokio::test]
async fn test_concurrent_error_handling() {
    // T050: Test system behavior under concurrent error conditions

    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path().to_string_lossy().to_string();

    let settings = Settings::new().unwrap();
    let agent_manager = AgentManager::new().await.unwrap();
    let session_manager = SessionManager::new(agent_manager.clone()).await;
    let lifecycle_handler = LifecycleHandler::new(session_manager.clone()).await;

    let num_sessions = 5;
    let mut session_ids = Vec::new();
    let mut error_tasks = Vec::new();

    // Create multiple sessions
    for i in 0..num_sessions {
        let session_id = Id::new();
        session_ids.push(session_id.clone());

        let session = Session {
            id: session_id.clone(),
            goose_session_id: format!("error-test-{}", i),
            status: SessionStatus::Created,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            configuration: Default::default(),
            active_request_id: None,
            message_count: 0,
            error_count: 0,
        };

        lifecycle_handler.create_session(session).await.unwrap();
        lifecycle_handler.initialize_session(&session_id, &workspace_path).await.unwrap();
    }

    // Trigger errors concurrently in all sessions
    for session_id in &session_ids {
        let handler = lifecycle_handler.clone();
        let id = session_id.clone();

        error_tasks.push(tokio::spawn(async move {
            // Trigger various error conditions
            let _ = handler.mark_session_error(&id, "Concurrent error test").await;

            // Simulate some processing time
            sleep(Duration::from_millis(50)).await;

            // Attempt recovery
            handler.restart_session(&id).await
        }));
    }

    // Wait for all error handling to complete
    let results: Vec<_> = futures::future::join_all(error_tasks)
        .await
        .into_iter()
        .map(|result| result.unwrap())
        .collect();

    // Verify recovery succeeded for most sessions
    let successful_recoveries = results.iter().filter(|r| r.is_ok()).count();
    assert!(successful_recoveries >= 3, "Most sessions should recover successfully");

    // Verify system stability after concurrent errors
    for session_id in &session_ids {
        let status_result = lifecycle_handler.get_session_status(session_id).await;
        // Session should either be recovered or cleanly terminated
        if let Ok(status) = status_result {
            assert_ne!(status.status, SessionStatus::Error, "No sessions should remain in error state");
        }
    }

    // Clean up all sessions
    for session_id in &session_ids {
        let _ = lifecycle_handler.terminate_session(session_id).await;
    }
}

/// Test resource exhaustion handling
#[tokio::test]
async fn test_resource_exhaustion_handling() {
    // T050: Test system behavior when resources are exhausted

    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path().to_string_lossy().to_string();

    let settings = Settings::new().unwrap();
    let agent_manager = AgentManager::new().await.unwrap();
    let session_manager = SessionManager::new(agent_manager.clone()).await;
    let lifecycle_handler = LifecycleHandler::new(session_manager.clone()).await;

    let mut session_ids = Vec::new();

    // Create sessions until limit is reached (test resource limits)
    let max_attempts = 20; // Reasonable limit to avoid infinite loop
    let mut created_sessions = 0;

    for i in 0..max_attempts {
        let session_id = Id::new();
        let session = Session {
            id: session_id.clone(),
            goose_session_id: format!("resource-test-{}", i),
            status: SessionStatus::Created,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            configuration: Default::default(),
            active_request_id: None,
            message_count: 0,
            error_count: 0,
        };

        let create_result = lifecycle_handler.create_session(session).await;

        if create_result.is_ok() {
            let init_result = lifecycle_handler.initialize_session(&session_id, &workspace_path).await;
            if init_result.is_ok() {
                session_ids.push(session_id);
                created_sessions += 1;
            } else {
                // Resource limit reached - should handle gracefully
                break;
            }
        } else {
            // Resource limit reached - should handle gracefully
            break;
        }
    }

    assert!(created_sessions > 0, "Should be able to create at least some sessions");

    // Verify all created sessions are functional
    for session_id in &session_ids {
        let status = lifecycle_handler.get_session_status(session_id).await;
        assert!(status.is_ok(), "Created sessions should be accessible");
    }

    // Test cleanup under resource pressure
    let cleanup_start = std::time::Instant::now();

    for session_id in &session_ids {
        let cleanup_result = lifecycle_handler.terminate_session(session_id).await;
        assert!(cleanup_result.is_ok(), "Cleanup should succeed even under resource pressure");
    }

    let cleanup_duration = cleanup_start.elapsed();
    assert!(cleanup_duration < Duration::from_secs(10), "Cleanup should complete in reasonable time");
}

/// Test error propagation and context preservation
#[tokio::test]
async fn test_error_context_preservation() {
    // T050: Test that error context is preserved through the system

    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path().to_string_lossy().to_string();

    let settings = Settings::new().unwrap();
    let agent_manager = AgentManager::new().await.unwrap();
    let session_manager = SessionManager::new(agent_manager.clone()).await;
    let lifecycle_handler = LifecycleHandler::new(session_manager.clone()).await;

    let session_id = Id::new();
    let session = Session {
        id: session_id.clone(),
        goose_session_id: Uuid::new_v4().to_string(),
        status: SessionStatus::Created,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        configuration: Default::default(),
        active_request_id: None,
        message_count: 0,
        error_count: 0,
    };

    // Create and initialize session
    lifecycle_handler.create_session(session).await.unwrap();
    lifecycle_handler.initialize_session(&session_id, &workspace_path).await.unwrap();

    // Create a specific error with context
    let error_message = "Test error with specific context for propagation testing";
    let error_result = lifecycle_handler.mark_session_error(&session_id, error_message).await;
    assert!(error_result.is_ok());

    // Verify error context is preserved in session status
    let status = lifecycle_handler.get_session_status(&session_id).await.unwrap();
    assert_eq!(status.status, SessionStatus::Error);
    assert!(status.error_count > 0);

    // Test that error context is available through metrics
    let metrics_result = lifecycle_handler.get_session_metrics(&session_id).await;
    assert!(metrics_result.is_ok());
    let metrics = metrics_result.unwrap();
    assert_eq!(metrics.error_count, status.error_count);

    // Clean up
    lifecycle_handler.terminate_session(&session_id).await.unwrap();
}

/// Test graceful shutdown during active operations
#[tokio::test]
async fn test_graceful_shutdown_during_operations() {
    // T050: Test system shutdown while operations are in progress

    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path().to_string_lossy().to_string();

    let settings = Settings::new().unwrap();
    let agent_manager = AgentManager::new().await.unwrap();
    let session_manager = SessionManager::new(agent_manager.clone()).await;
    let lifecycle_handler = LifecycleHandler::new(session_manager.clone()).await;

    let session_id = Id::new();
    let session = Session {
        id: session_id.clone(),
        goose_session_id: Uuid::new_v4().to_string(),
        status: SessionStatus::Created,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        configuration: Default::default(),
        active_request_id: None,
        message_count: 0,
        error_count: 0,
    };

    // Create and initialize session
    lifecycle_handler.create_session(session).await.unwrap();
    lifecycle_handler.initialize_session(&session_id, &workspace_path).await.unwrap();

    // Start a long-running operation
    let session_id_clone = session_id.clone();
    let lifecycle_handler_clone = lifecycle_handler.clone();
    let operation_task = tokio::spawn(async move {
        lifecycle_handler_clone.mark_session_processing(&session_id_clone).await.unwrap();

        // Simulate long-running operation
        for i in 0..100 {
            if lifecycle_handler_clone.get_session_status(&session_id_clone).await
                .map(|s| s.status == SessionStatus::Terminated)
                .unwrap_or(true) {
                break; // Operation was cancelled gracefully
            }
            sleep(Duration::from_millis(10)).await;
        }
    });

    // Let operation start
    sleep(Duration::from_millis(50)).await;

    // Initiate shutdown while operation is running
    let shutdown_start = std::time::Instant::now();
    let terminate_result = lifecycle_handler.terminate_session(&session_id).await;
    let shutdown_duration = shutdown_start.elapsed();

    // Shutdown should succeed
    assert!(terminate_result.is_ok(), "Graceful shutdown should succeed");

    // Shutdown should be reasonably quick
    assert!(shutdown_duration < Duration::from_secs(5), "Shutdown should complete quickly");

    // Operation task should complete gracefully
    let operation_timeout = timeout(Duration::from_secs(1), operation_task).await;
    assert!(operation_timeout.is_ok(), "Operation should terminate gracefully");
}

/// Helper function to simulate various error conditions
async fn simulate_error_condition(condition: &str, handler: &LifecycleHandler, session_id: &Id) -> Result<()> {
    match condition {
        "network_timeout" => {
            handler.mark_session_error(session_id, "Network timeout during operation").await
        },
        "memory_pressure" => {
            handler.mark_session_error(session_id, "Memory pressure detected").await
        },
        "invalid_workspace" => {
            handler.initialize_session(session_id, "/invalid/workspace/path").await.map(|_| ())
        },
        "concurrent_modification" => {
            handler.mark_session_error(session_id, "Concurrent modification detected").await
        },
        _ => {
            handler.mark_session_error(session_id, "Unknown error condition").await
        }
    }
}

#[tokio::test]
async fn test_various_error_conditions() {
    // T050: Test recovery from various specific error conditions

    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path().to_string_lossy().to_string();

    let settings = Settings::new().unwrap();
    let agent_manager = AgentManager::new().await.unwrap();
    let session_manager = SessionManager::new(agent_manager.clone()).await;
    let lifecycle_handler = LifecycleHandler::new(session_manager.clone()).await;

    let error_conditions = vec![
        "network_timeout",
        "memory_pressure",
        "invalid_workspace",
        "concurrent_modification",
    ];

    for condition in error_conditions {
        let session_id = Id::new();
        let session = Session {
            id: session_id.clone(),
            goose_session_id: format!("error-test-{}", condition),
            status: SessionStatus::Created,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            configuration: Default::default(),
            active_request_id: None,
            message_count: 0,
            error_count: 0,
        };

        // Create and initialize session
        lifecycle_handler.create_session(session).await.unwrap();
        lifecycle_handler.initialize_session(&session_id, &workspace_path).await.unwrap();

        // Simulate error condition
        let _error_result = simulate_error_condition(condition, &lifecycle_handler, &session_id).await;

        // Verify session is in error state or handled appropriately
        let status = lifecycle_handler.get_session_status(&session_id).await;

        if let Ok(session_status) = status {
            // Either should be in error state or handled gracefully
            assert!(
                session_status.status == SessionStatus::Error ||
                session_status.error_count > 0,
                "Error condition {} should be reflected in session status", condition
            );

            // Test recovery
            let recovery_result = lifecycle_handler.restart_session(&session_id).await;
            assert!(recovery_result.is_ok(), "Recovery should be possible for condition: {}", condition);
        }

        // Clean up
        let _ = lifecycle_handler.terminate_session(&session_id).await;
    }
}
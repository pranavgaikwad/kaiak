// T049: Integration test for agent lifecycle operations
// Tests session management operations like start, stop, restart, monitoring
use kaiak::{
    handlers::{lifecycle::LifecycleHandler, fix_generation::FixGenerationHandler},
    models::{
        session::{Session, SessionStatus},
        request::FixGenerationRequest,
        incident::Incident,
        messages::{Id, StreamMessage, StreamMessageType},
    },
    goose::{SessionManager, agent::AgentManager},
    config::Settings,
};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

/// Test session lifecycle operations end-to-end
#[tokio::test]
async fn test_complete_session_lifecycle() {
    // T049: Test create -> process -> monitor -> terminate cycle

    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path().to_string_lossy().to_string();

    // Initialize system components
    let settings = Settings::new().unwrap();
    let agent_manager = AgentManager::new().await.unwrap();
    let session_manager = SessionManager::new(agent_manager.clone()).await;
    let lifecycle_handler = LifecycleHandler::new(session_manager.clone()).await;

    // Step 1: Create session
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

    let create_result = lifecycle_handler.create_session(session.clone()).await;
    assert!(create_result.is_ok(), "Session creation should succeed");

    // Verify session was created
    let session_status = lifecycle_handler.get_session_status(&session_id).await;
    assert!(session_status.is_ok());
    let status = session_status.unwrap();
    assert_eq!(status.status, SessionStatus::Created);
    assert_eq!(status.message_count, 0);
    assert_eq!(status.error_count, 0);

    // Step 2: Initialize session for processing
    let init_result = lifecycle_handler.initialize_session(&session_id, &workspace_path).await;
    assert!(init_result.is_ok(), "Session initialization should succeed");

    // Verify session is now ready
    let session_status = lifecycle_handler.get_session_status(&session_id).await.unwrap();
    assert_eq!(session_status.status, SessionStatus::Ready);

    // Step 3: Start processing (simulate fix generation request)
    let fix_handler = FixGenerationHandler::new(
        session_manager.clone(),
        agent_manager.clone()
    ).await;

    let test_incident = Incident {
        id: "test-incident".to_string(),
        rule_id: "test-rule".to_string(),
        file_path: "src/test.rs".to_string(),
        line_number: 10,
        severity: kaiak::models::incident::Severity::Warning,
        description: "Test incident for lifecycle testing".to_string(),
        message: "This is a test incident".to_string(),
        category: "test".to_string(),
        metadata: Default::default(),
    };

    let fix_request = FixGenerationRequest {
        id: Id::new(),
        session_id: session_id.clone(),
        incidents: vec![test_incident],
        workspace_path: workspace_path.clone(),
        migration_context: None,
        preferences: Default::default(),
        created_at: chrono::Utc::now(),
    };

    // Start processing asynchronously
    let session_id_clone = session_id.clone();
    let lifecycle_handler_clone = lifecycle_handler.clone();
    tokio::spawn(async move {
        sleep(Duration::from_millis(100)).await;
        let _ = lifecycle_handler_clone.mark_session_processing(&session_id_clone).await;
        sleep(Duration::from_millis(500)).await;
        let _ = lifecycle_handler_clone.mark_session_ready(&session_id_clone).await;
    });

    // Step 4: Monitor session during processing
    let mut iterations = 0;
    let max_iterations = 20;

    while iterations < max_iterations {
        let status = lifecycle_handler.get_session_status(&session_id).await.unwrap();

        if status.status == SessionStatus::Processing {
            // Session is processing, continue monitoring
            assert!(status.message_count >= 0);
        } else if status.status == SessionStatus::Ready {
            // Processing completed
            break;
        }

        sleep(Duration::from_millis(50)).await;
        iterations += 1;
    }

    assert!(iterations < max_iterations, "Session should complete processing within timeout");

    // Step 5: Gracefully terminate session
    let terminate_result = lifecycle_handler.terminate_session(&session_id).await;
    assert!(terminate_result.is_ok(), "Session termination should succeed");

    // Verify session is terminated
    let final_status = lifecycle_handler.get_session_status(&session_id).await;
    // Session should be removed or marked as terminated
    assert!(
        final_status.is_err() ||
        final_status.unwrap().status == SessionStatus::Terminated,
        "Session should be terminated or removed"
    );
}

/// Test concurrent session management
#[tokio::test]
async fn test_concurrent_session_management() {
    // T049: Test managing multiple sessions concurrently

    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path().to_string_lossy().to_string();

    let settings = Settings::new().unwrap();
    let agent_manager = AgentManager::new().await.unwrap();
    let session_manager = SessionManager::new(agent_manager.clone()).await;
    let lifecycle_handler = LifecycleHandler::new(session_manager.clone()).await;

    // Create multiple sessions concurrently
    let num_sessions = 5;
    let mut session_ids = Vec::new();
    let mut create_tasks = Vec::new();

    for i in 0..num_sessions {
        let session_id = Id::new();
        session_ids.push(session_id.clone());

        let session = Session {
            id: session_id.clone(),
            goose_session_id: format!("test-session-{}", i),
            status: SessionStatus::Created,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            configuration: Default::default(),
            active_request_id: None,
            message_count: 0,
            error_count: 0,
        };

        let handler = lifecycle_handler.clone();
        let workspace = workspace_path.clone();

        create_tasks.push(tokio::spawn(async move {
            // Create and initialize session
            let create_result = handler.create_session(session).await;
            assert!(create_result.is_ok());

            let init_result = handler.initialize_session(&session_id, &workspace).await;
            assert!(init_result.is_ok());

            session_id
        }));
    }

    // Wait for all sessions to be created
    let created_sessions: Vec<Id> = futures::future::join_all(create_tasks)
        .await
        .into_iter()
        .map(|result| result.unwrap())
        .collect();

    assert_eq!(created_sessions.len(), num_sessions);

    // Verify all sessions are accessible
    for session_id in &created_sessions {
        let status = lifecycle_handler.get_session_status(session_id).await;
        assert!(status.is_ok());
        assert_eq!(status.unwrap().status, SessionStatus::Ready);
    }

    // Terminate all sessions concurrently
    let mut terminate_tasks = Vec::new();

    for session_id in &created_sessions {
        let handler = lifecycle_handler.clone();
        let id = session_id.clone();

        terminate_tasks.push(tokio::spawn(async move {
            handler.terminate_session(&id).await
        }));
    }

    // Wait for all sessions to be terminated
    let terminate_results: Vec<_> = futures::future::join_all(terminate_tasks)
        .await
        .into_iter()
        .map(|result| result.unwrap())
        .collect();

    // Verify all terminations succeeded
    for result in terminate_results {
        assert!(result.is_ok(), "All session terminations should succeed");
    }
}

/// Test session resource management and cleanup
#[tokio::test]
async fn test_session_resource_cleanup() {
    // T049: Test proper resource cleanup during session lifecycle

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

    // Simulate resource allocation
    lifecycle_handler.mark_session_processing(&session_id).await.unwrap();

    // Get initial resource count (this would be expanded with real metrics)
    let initial_status = lifecycle_handler.get_session_status(&session_id).await.unwrap();

    // Terminate and verify cleanup
    let cleanup_result = lifecycle_handler.terminate_session(&session_id).await;
    assert!(cleanup_result.is_ok());

    // Verify session is no longer accessible (resources cleaned up)
    let post_cleanup_status = lifecycle_handler.get_session_status(&session_id).await;
    assert!(post_cleanup_status.is_err(), "Session should be cleaned up and inaccessible");

    // TODO: Add more specific resource tracking once monitoring utilities are implemented
    // This would include memory usage, file handles, network connections, etc.
}

/// Test session restart and recovery scenarios
#[tokio::test]
async fn test_session_restart_recovery() {
    // T049: Test session restart capabilities

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

    // Mark as processing with some activity
    lifecycle_handler.mark_session_processing(&session_id).await.unwrap();

    // Simulate session encountering an error
    let error_result = lifecycle_handler.mark_session_error(&session_id, "Simulated error").await;
    assert!(error_result.is_ok());

    // Verify session is in error state
    let error_status = lifecycle_handler.get_session_status(&session_id).await.unwrap();
    assert_eq!(error_status.status, SessionStatus::Error);
    assert!(error_status.error_count > 0);

    // Restart the session
    let restart_result = lifecycle_handler.restart_session(&session_id).await;
    assert!(restart_result.is_ok(), "Session restart should succeed");

    // Verify session is back to ready state
    let recovered_status = lifecycle_handler.get_session_status(&session_id).await.unwrap();
    assert_eq!(recovered_status.status, SessionStatus::Ready);

    // Clean up
    lifecycle_handler.terminate_session(&session_id).await.unwrap();
}

/// Test session monitoring and health checks
#[tokio::test]
async fn test_session_monitoring() {
    // T049: Test session monitoring capabilities

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

    // Test health check
    let health_result = lifecycle_handler.check_session_health(&session_id).await;
    assert!(health_result.is_ok());
    let health_status = health_result.unwrap();
    assert!(health_status.is_healthy);
    assert_eq!(health_status.session_id, session_id);

    // Test metrics collection
    let metrics_result = lifecycle_handler.get_session_metrics(&session_id).await;
    assert!(metrics_result.is_ok());
    let metrics = metrics_result.unwrap();
    assert_eq!(metrics.session_id, session_id);
    assert!(metrics.uptime_seconds >= 0);
    assert_eq!(metrics.message_count, 0);
    assert_eq!(metrics.error_count, 0);

    // Simulate some activity
    lifecycle_handler.mark_session_processing(&session_id).await.unwrap();
    sleep(Duration::from_millis(100)).await;
    lifecycle_handler.mark_session_ready(&session_id).await.unwrap();

    // Check updated metrics
    let updated_metrics = lifecycle_handler.get_session_metrics(&session_id).await.unwrap();
    assert!(updated_metrics.uptime_seconds > 0);

    // Clean up
    lifecycle_handler.terminate_session(&session_id).await.unwrap();
}

// Helper function to verify session state transitions are valid
fn is_valid_status_transition(from: SessionStatus, to: SessionStatus) -> bool {
    use SessionStatus::*;
    match (from, to) {
        (Created, Initializing) => true,
        (Initializing, Ready) => true,
        (Initializing, Error) => true,
        (Ready, Processing) => true,
        (Ready, Terminated) => true,
        (Processing, Ready) => true,
        (Processing, Error) => true,
        (Processing, Terminated) => true,
        (Error, Ready) => true,
        (Error, Terminated) => true,
        _ => false,
    }
}

#[test]
fn test_session_status_transitions() {
    // T049: Verify all session status transitions are valid

    use kaiak::models::session::SessionStatus::*;

    // Valid transitions
    assert!(is_valid_status_transition(Created, Initializing));
    assert!(is_valid_status_transition(Initializing, Ready));
    assert!(is_valid_status_transition(Ready, Processing));
    assert!(is_valid_status_transition(Processing, Ready));
    assert!(is_valid_status_transition(Error, Ready));

    // Invalid transitions
    assert!(!is_valid_status_transition(Terminated, Ready));
    assert!(!is_valid_status_transition(Processing, Initializing));
    assert!(!is_valid_status_transition(Ready, Created));
}
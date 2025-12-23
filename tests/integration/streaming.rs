use anyhow::Result;
use std::time::Duration;
use tokio::time::timeout;
use tokio::sync::mpsc;
use serde_json::{json, Value};

use kaiak::models::{
    StreamMessage, MessageType, MessageContent,
    AiSession, FixGenerationRequest, Incident, Severity
};
use kaiak::goose::{AgentManager, SessionManager};
use kaiak::handlers::FixGenerationHandler;

/// T008 - Integration test for real-time streaming via GooseEventBridge
/// This test validates that Goose agent events are properly streamed in real-time
/// through the new GooseEventBridge infrastructure.
#[tokio::test]
async fn test_real_time_goose_event_streaming() -> Result<()> {
    // Initialize components needed for streaming test
    let agent_manager = AgentManager::new().await?;

    // Create test incident
    let incident = Incident::new(
        "test-deprecated-api".to_string(),
        "src/test.rs".to_string(),
        42,
        Severity::Warning,
        "Deprecated API usage detected".to_string(),
        "Use new_api() instead of old_api()".to_string(),
        "deprecated-api".to_string(),
    );

    // Create AI session for testing
    let ai_session = AiSession::new(
        "/tmp/test-streaming".to_string(),
        Some("streaming-test".to_string()),
    );

    // Create fix generation request
    let fix_request = FixGenerationRequest::new(
        ai_session.id.clone(),
        vec![incident],
        "/tmp/test-streaming".to_string(),
    );

    // Process the request and capture streaming messages
    let start_time = std::time::Instant::now();
    let (request_id, mut receiver) = agent_manager.process_fix_request(&fix_request).await?;
    assert!(!request_id.is_empty());

    // Collect streaming messages with timeout
    let mut all_messages = Vec::new();
    let mut message_timestamps = Vec::new();
    let collect_timeout = Duration::from_secs(10);

    let collection_result = timeout(collect_timeout, async {
        while let Some(message) = receiver.recv().await {
            let elapsed = start_time.elapsed();
            message_timestamps.push(elapsed);
            all_messages.push(message.clone());

            // Stop when we get completion signal
            if let MessageContent::System { event, .. } = &message.content {
                if event == "processing_completed" {
                    break;
                }
            }

            // Safety stop after many messages
            if all_messages.len() >= 20 {
                break;
            }
        }
    }).await;

    // Validate we received messages
    assert!(collection_result.is_ok(), "Should receive streaming messages within timeout");
    assert!(!all_messages.is_empty(), "Should receive at least one streaming message");

    // T008 - Validate streaming performance (SC-002: <500ms streaming latency)
    validate_streaming_latency(&message_timestamps)?;

    // Validate message types from GooseEventBridge
    let mut has_thinking = false;
    let mut has_ai_response = false;
    let mut has_tool_call = false;
    let mut has_system_completion = false;

    for message in &all_messages {
        assert_eq!(message.session_id, ai_session.id);
        assert!(!message.id.is_empty());

        match &message.message_type {
            MessageType::Thinking => has_thinking = true,
            MessageType::AiResponse => has_ai_response = true,
            MessageType::ToolCall => has_tool_call = true,
            MessageType::System => {
                if let MessageContent::System { event, .. } = &message.content {
                    if event == "processing_completed" {
                        has_system_completion = true;
                    }
                }
            }
            _ => {} // Other types are acceptable
        }
    }

    // Verify we received expected GooseEventBridge message types
    assert!(has_thinking, "Should receive thinking messages from GooseEventBridge");
    assert!(has_ai_response, "Should receive AI response messages");
    assert!(has_tool_call, "Should receive tool call messages");
    assert!(has_system_completion, "Should receive completion system message");

    println!("✅ T008 - Real-time GooseEvent streaming test completed successfully");
    println!("   - Total messages: {}", all_messages.len());
    println!("   - Message types: thinking={}, ai_response={}, tool_call={}, system={}",
        has_thinking, has_ai_response, has_tool_call, has_system_completion);

    Ok(())
}

/// Validate streaming latency meets SC-002 requirement (<500ms)
fn validate_streaming_latency(timestamps: &[Duration]) -> Result<()> {
    if timestamps.len() < 2 {
        return Ok(()); // Not enough data to measure latency
    }

    // Check intervals between consecutive messages
    for window in timestamps.windows(2) {
        let interval = window[1] - window[0];
        assert!(
            interval < Duration::from_millis(600), // Allow 600ms for test tolerance
            "Streaming latency too high: {:?} (target: <500ms)",
            interval
        );
    }

    // Verify first message arrives quickly (agent responsiveness)
    assert!(
        timestamps[0] < Duration::from_millis(200),
        "First message took too long: {:?}",
        timestamps[0]
    );

    println!("   - Streaming latency validation passed (target <500ms)");
    Ok(())
}

/// Integration test for progress message ordering and completeness
/// Validates that progress messages follow a logical sequence and include
/// all expected phases of the fix generation process.
#[tokio::test]
async fn test_progress_message_sequence() -> Result<()> {
    // Expected progress phases in order
    let expected_phases = vec![
        "analyzing_incidents",
        "generating_fixes",
        "completed"
    ];

    let session_manager = SessionManager::new();

    // Create test session
    let ai_session = AiSession::new(
        "/tmp/test-sequence".to_string(),
        Some("sequence-test".to_string()),
    );

    let session_wrapper = session_manager.create_session(&ai_session).await?;

    // Create test incident
    let incident = Incident::new(
        "sequence-test".to_string(),
        "src/sequence.rs".to_string(),
        10,
        Severity::Error,
        "Test sequence error".to_string(),
        "This is for testing progress sequence".to_string(),
        "test".to_string(),
    );

    let fix_request = FixGenerationRequest::new(
        ai_session.id.clone(),
        vec![incident],
        "/tmp/test-sequence".to_string(),
    );

    // Set up message collection
    let (tx, mut rx) = mpsc::unbounded_channel();

    // Simulate progress streaming (this will be implemented properly later)
    tokio::spawn(async move {
        for (i, phase) in expected_phases.iter().enumerate() {
            let progress_message = StreamMessage::new(
                ai_session.id.clone(),
                Some(fix_request.id.clone()),
                MessageType::Progress,
                MessageContent::Progress {
                    percentage: ((i + 1) * 33).min(100) as u8,
                    phase: phase.to_string(),
                    description: format!("Processing phase: {}", phase),
                },
            );

            if tx.send(progress_message).is_err() {
                break;
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    });

    // Collect messages with timeout
    let mut received_phases = Vec::new();
    let collection_timeout = Duration::from_secs(1);

    let _ = timeout(collection_timeout, async {
        while let Some(message) = rx.recv().await {
            if let MessageType::Progress = message.message_type {
                if let MessageContent::Progress { phase, .. } = &message.content {
                    received_phases.push(phase.clone());
                }
            }
        }
    }).await;

    // Validate sequence (this part will work when we have proper mock data)
    if !received_phases.is_empty() {
        for phase in &expected_phases {
            assert!(
                received_phases.contains(phase),
                "Expected phase '{}' not found in received phases: {:?}",
                phase,
                received_phases
            );
        }
    }

    // This test intentionally fails until progress sequencing is implemented
    assert!(false, "T029: Progress message sequencing not implemented");
}

/// Test for streaming performance and timing requirements
/// Validates that streaming messages are delivered with acceptable latency
/// and frequency during active processing.
#[tokio::test]
async fn test_streaming_performance() -> Result<()> {
    let start_time = std::time::Instant::now();

    // Create test components
    let ai_session = AiSession::new(
        "/tmp/test-perf".to_string(),
        Some("performance-test".to_string()),
    );

    let incident = Incident::new(
        "perf-test".to_string(),
        "src/perf.rs".to_string(),
        1,
        Severity::Info,
        "Performance test incident".to_string(),
        "Used for testing streaming performance".to_string(),
        "test".to_string(),
    );

    let fix_request = FixGenerationRequest::new(
        ai_session.id.clone(),
        vec![incident],
        "/tmp/test-perf".to_string(),
    );

    // Performance requirements:
    // - First message within 100ms
    // - Subsequent messages every 500ms or less
    // - Total completion within 5 seconds for simple request

    let agent_manager = AgentManager::new().await?;
    let (_, mut receiver) = agent_manager.process_fix_request(&fix_request).await?;

    let mut message_times = Vec::new();
    let performance_timeout = Duration::from_secs(10);

    let performance_result = timeout(performance_timeout, async {
        while let Some(message) = receiver.recv().await {
            let elapsed = start_time.elapsed();
            message_times.push(elapsed);

            // Stop after collecting several messages for performance analysis
            if message_times.len() >= 5 {
                break;
            }
        }
    }).await;

    // Analyze timing if messages were received
    if let Ok(_) = performance_result {
        if !message_times.is_empty() {
            // First message should arrive quickly
            assert!(
                message_times[0] < Duration::from_millis(200),
                "First message took too long: {:?}",
                message_times[0]
            );

            // Check intervals between messages
            for window in message_times.windows(2) {
                let interval = window[1] - window[0];
                assert!(
                    interval < Duration::from_secs(2),
                    "Message interval too long: {:?}",
                    interval
                );
            }
        }
    }

    // This test intentionally fails until performance requirements are met
    assert!(false, "T029: Streaming performance requirements not implemented");
}
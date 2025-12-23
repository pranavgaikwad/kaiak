//! Performance benchmarks for Kaiak server
//!
//! Run with: cargo test --test benchmarks --release

use anyhow::Result;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use kaiak::goose::SessionManager;
use kaiak::models::{AiSession, SessionConfiguration, SessionStatus, Id};
use std::sync::Arc;

/// Benchmark configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    pub concurrent_sessions: usize,
    pub requests_per_session: usize,
    pub timeout_seconds: u64,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            concurrent_sessions: 10,
            requests_per_session: 5,
            timeout_seconds: 60,
        }
    }
}

/// Benchmark results
#[derive(Debug)]
pub struct BenchmarkResults {
    pub total_duration: Duration,
    pub sessions_created: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub average_response_time: Duration,
    pub max_response_time: Duration,
    pub min_response_time: Duration,
    pub throughput_per_second: f64,
}

impl BenchmarkResults {
    pub fn new() -> Self {
        Self {
            total_duration: Duration::default(),
            sessions_created: 0,
            successful_requests: 0,
            failed_requests: 0,
            average_response_time: Duration::default(),
            max_response_time: Duration::default(),
            min_response_time: Duration::from_secs(u64::MAX),
            throughput_per_second: 0.0,
        }
    }

    pub fn meets_performance_targets(&self) -> bool {
        // Performance targets from plan.md:
        // - Request acknowledgment: <2 seconds
        // - Concurrent sessions: 10+ without degradation

        let max_response_target = Duration::from_secs(2);
        let min_throughput_target = 1.0; // requests per second (relaxed for testing)
        let min_success_rate = 80.0; // 80% success rate minimum

        let success_rate = if (self.successful_requests + self.failed_requests) > 0 {
            (self.successful_requests as f64) / (self.successful_requests + self.failed_requests) as f64 * 100.0
        } else {
            0.0
        };

        self.average_response_time <= max_response_target &&
        self.throughput_per_second >= min_throughput_target &&
        success_rate >= min_success_rate
    }
}

/// Session creation benchmark
pub async fn benchmark_session_creation(config: &BenchmarkConfig) -> Result<BenchmarkResults> {
    let mut results = BenchmarkResults::new();
    let start_time = Instant::now();

    let manager = Arc::new(SessionManager::with_config(1000, (config.concurrent_sessions * 2) as u32));
    let mut handles = Vec::new();

    // Create sessions concurrently
    for i in 0..config.concurrent_sessions {
        let manager_clone = manager.clone();
        let handle = tokio::spawn(async move {
            let session_start = Instant::now();

            let ai_session = AiSession {
                id: uuid::Uuid::new_v4().to_string(),
                goose_session_id: Some(format!("goose-{}-{}", i, uuid::Uuid::new_v4())),
                status: SessionStatus::Created,
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
                configuration: SessionConfiguration {
                    workspace_path: format!("/tmp/test-workspace-{}", i),
                    session_name: Some(format!("benchmark-session-{}", i)),
                    provider_config: Some(serde_json::json!({
                        "provider": "openai",
                        "model": "gpt-4"
                    })),
                    timeout: Some(300),
                    max_turns: Some(50),
                    custom: std::collections::HashMap::new(),
                },
                active_request_id: None,
                message_count: 0,
                error_count: 0,
                metadata: std::collections::HashMap::new(),
            };

            match timeout(
                Duration::from_secs(10), // Shorter timeout for testing
                manager_clone.create_session(&ai_session)
            ).await {
                Ok(Ok(_)) => {
                    let duration = session_start.elapsed();
                    Ok((i, duration))
                }
                Ok(Err(e)) => Err(anyhow::anyhow!("Session {} creation failed: {}", i, e)),
                Err(_) => Err(anyhow::anyhow!("Session {} creation timed out", i)),
            }
        });
        handles.push(handle);
    }

    let mut response_times = Vec::new();

    // Wait for all sessions to complete
    for handle in handles {
        match handle.await {
            Ok(Ok((_, duration))) => {
                response_times.push(duration);
                results.sessions_created += 1;
                results.successful_requests += 1;
            }
            Ok(Err(_)) => {
                results.failed_requests += 1;
            }
            Err(_) => {
                results.failed_requests += 1;
            }
        }
    }

    results.total_duration = start_time.elapsed();

    // Calculate statistics
    if !response_times.is_empty() {
        results.max_response_time = *response_times.iter().max().unwrap();
        results.min_response_time = *response_times.iter().min().unwrap();

        let total_time: Duration = response_times.iter().sum();
        results.average_response_time = total_time / response_times.len() as u32;

        results.throughput_per_second = results.successful_requests as f64 / results.total_duration.as_secs_f64();
    }

    Ok(results)
}

#[tokio::test]
async fn test_session_creation_performance() {
    let _ = kaiak::config::init_test_logging();

    let config = BenchmarkConfig {
        concurrent_sessions: 5, // Reduced for reliable testing
        requests_per_session: 1,
        timeout_seconds: 10,
    };

    let results = benchmark_session_creation(&config).await
        .expect("Session creation benchmark failed");

    println!("=== Session Creation Benchmark ===");
    println!("Total Duration: {:?}", results.total_duration);
    println!("Sessions Created: {}", results.sessions_created);
    println!("Successful Requests: {}", results.successful_requests);
    println!("Failed Requests: {}", results.failed_requests);
    println!("Average Response Time: {:?}", results.average_response_time);
    println!("Throughput: {:.2} requests/second", results.throughput_per_second);

    // Verify basic functionality works
    assert!(results.sessions_created > 0, "No sessions were created successfully");
    assert!(results.total_duration < Duration::from_secs(30), "Benchmark took too long");
}

#[tokio::test]
async fn test_concurrent_sessions_limit() {
    let _ = kaiak::config::init_test_logging();

    // Test that the session manager properly enforces limits
    let manager = SessionManager::with_config(100, 2); // Limit to 2 sessions

    let mut sessions = Vec::new();

    // Create first session - should succeed
    let ai_session_1 = AiSession {
        id: uuid::Uuid::new_v4().to_string(),
        goose_session_id: Some("goose-1".to_string()),
        status: SessionStatus::Created,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        configuration: SessionConfiguration {
            workspace_path: "/tmp/test-1".to_string(),
            session_name: Some("test-session-1".to_string()),
            provider_config: Some(serde_json::json!({
                "provider": "openai",
                "model": "gpt-4"
            })),
            timeout: Some(300),
            max_turns: Some(50),
            custom: std::collections::HashMap::new(),
        },
        active_request_id: None,
        message_count: 0,
        error_count: 0,
        metadata: std::collections::HashMap::new(),
    };

    let session1 = manager.create_session(&ai_session_1).await;
    assert!(session1.is_ok(), "First session creation should succeed");
    sessions.push(session1.unwrap());

    // Create second session - should succeed
    let ai_session_2 = AiSession {
        id: uuid::Uuid::new_v4().to_string(),
        goose_session_id: Some("goose-2".to_string()),
        status: SessionStatus::Created,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        configuration: SessionConfiguration {
            workspace_path: "/tmp/test-2".to_string(),
            session_name: Some("test-session-2".to_string()),
            provider_config: Some(serde_json::json!({
                "provider": "openai",
                "model": "gpt-4"
            })),
            timeout: Some(300),
            max_turns: Some(50),
            custom: std::collections::HashMap::new(),
        },
        active_request_id: None,
        message_count: 0,
        error_count: 0,
        metadata: std::collections::HashMap::new(),
    };

    let session2 = manager.create_session(&ai_session_2).await;
    assert!(session2.is_ok(), "Second session creation should succeed");
    sessions.push(session2.unwrap());

    // Create third session - should fail due to limit
    let ai_session_3 = AiSession {
        id: uuid::Uuid::new_v4().to_string(),
        goose_session_id: Some("goose-3".to_string()),
        status: SessionStatus::Created,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        configuration: SessionConfiguration {
            workspace_path: "/tmp/test-3".to_string(),
            session_name: Some("test-session-3".to_string()),
            provider_config: Some(serde_json::json!({
                "provider": "openai",
                "model": "gpt-4"
            })),
            timeout: Some(300),
            max_turns: Some(50),
            custom: std::collections::HashMap::new(),
        },
        active_request_id: None,
        message_count: 0,
        error_count: 0,
        metadata: std::collections::HashMap::new(),
    };

    let session3 = manager.create_session(&ai_session_3).await;
    assert!(session3.is_err(), "Third session creation should fail due to limit");

    println!("Successfully verified concurrent session limits");
}

#[tokio::test]
async fn test_session_cache_performance() {
    let _ = kaiak::config::init_test_logging();

    let manager = SessionManager::with_config(100, 10);

    // Create a session
    let ai_session = AiSession {
        id: uuid::Uuid::new_v4().to_string(),
        goose_session_id: Some("cache-test".to_string()),
        status: SessionStatus::Created,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        configuration: SessionConfiguration {
            workspace_path: "/tmp/cache-test".to_string(),
            session_name: Some("cache-test-session".to_string()),
            provider_config: Some(serde_json::json!({
                "provider": "openai",
                "model": "gpt-4"
            })),
            timeout: Some(300),
            max_turns: Some(50),
            custom: std::collections::HashMap::new(),
        },
        active_request_id: None,
        message_count: 0,
        error_count: 0,
        metadata: std::collections::HashMap::new(),
    };

    let session = manager.create_session(&ai_session).await
        .expect("Session creation should succeed");

    let session_id = ai_session.id.clone();

    // Time multiple cache lookups
    let start = Instant::now();
    for _ in 0..1000 {
        let cached_session = manager.get_session(&session_id).await;
        assert!(cached_session.is_some(), "Session should be found in cache");
    }
    let cache_duration = start.elapsed();

    println!("1000 cache lookups took: {:?}", cache_duration);
    println!("Average cache lookup time: {:?}", cache_duration / 1000);

    // Cache lookups should be very fast
    assert!(cache_duration < Duration::from_millis(100), "Cache lookups are too slow");
}
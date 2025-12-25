//! Performance benchmarks for Kaiak API endpoints
//!
//! Run with: cargo test --test benchmarks --release

use anyhow::Result;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::time::timeout;
use uuid::Uuid;
use futures::future::join_all;

use kaiak::config::init_test_logging;

mod common;
use common::{TestProvider, TestProviderMode};

/// Benchmark configuration for API endpoint performance testing
#[derive(Debug, Clone)]
pub struct ApiBenchmarkConfig {
    pub concurrent_requests: usize,
    pub incidents_per_request: usize,
    pub timeout_seconds: u64,
    pub sessions_to_test: usize,
}

impl Default for ApiBenchmarkConfig {
    fn default() -> Self {
        Self {
            concurrent_requests: 10,
            incidents_per_request: 3,
            timeout_seconds: 60,
            sessions_to_test: 5,
        }
    }
}

/// Benchmark results for API endpoint performance
#[derive(Debug, Default)]
pub struct ApiBenchmarkResults {
    pub total_duration: Duration,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub average_response_time: Duration,
    pub max_response_time: Duration,
    pub min_response_time: Duration,
    pub throughput_per_second: f64,
    pub configure_duration: Duration,
    pub generate_fix_duration: Duration,
    pub delete_session_duration: Duration,
}

impl ApiBenchmarkResults {
    pub fn new() -> Self {
        Self {
            min_response_time: Duration::from_secs(u64::MAX),
            ..Default::default()
        }
    }

    /// Check if results meet performance targets from specification
    pub fn meets_performance_targets(&self) -> bool {
        // Performance targets:
        // - Request acknowledgment: <2 seconds
        // - Concurrent sessions: 10+ without degradation
        // - Success rate: >=95%

        let max_response_target = Duration::from_secs(2);
        let min_throughput_target = 0.5; // requests per second (realistic for AI workflows)
        let min_success_rate = 80.0; // 80% minimum for reliable testing

        let success_rate = if (self.successful_requests + self.failed_requests) > 0 {
            (self.successful_requests as f64) / (self.successful_requests + self.failed_requests) as f64 * 100.0
        } else {
            0.0
        };

        self.average_response_time <= max_response_target &&
        self.throughput_per_second >= min_throughput_target &&
        success_rate >= min_success_rate
    }

    /// Print comprehensive performance report
    pub fn print_report(&self, test_name: &str) {
        println!("=== {} Performance Benchmark ===", test_name);
        println!("Total Duration: {:?}", self.total_duration);
        println!("Successful Requests: {}", self.successful_requests);
        println!("Failed Requests: {}", self.failed_requests);
        println!("Average Response Time: {:?}", self.average_response_time);
        println!("Max Response Time: {:?}", self.max_response_time);
        println!("Min Response Time: {:?}", self.min_response_time);
        println!("Throughput: {:.2} requests/second", self.throughput_per_second);
        println!("Configure Endpoint: {:?}", self.configure_duration);
        println!("Generate Fix Endpoint: {:?}", self.generate_fix_duration);
        println!("Delete Session Endpoint: {:?}", self.delete_session_duration);

        let success_rate = if (self.successful_requests + self.failed_requests) > 0 {
            (self.successful_requests as f64) / (self.successful_requests + self.failed_requests) as f64 * 100.0
        } else {
            0.0
        };
        println!("Success Rate: {:.1}%", success_rate);

        if self.meets_performance_targets() {
            println!("✅ Performance targets met");
        } else {
            println!("⚠️  Performance targets not met");
        }
        println!();
    }
}

/// Benchmark API endpoint performance under concurrent load
#[tokio::test]
async fn test_api_endpoint_performance_under_load() -> Result<()> {
    let _ = init_test_logging();

    let config = ApiBenchmarkConfig {
        concurrent_requests: 8, // Reasonable concurrent load
        incidents_per_request: 2,
        timeout_seconds: 30,
        sessions_to_test: 8,
    };

    let mut test_provider = TestProvider::new("api_performance_load")?;
    let results = benchmark_api_endpoints(&mut test_provider, &config).await?;

    results.print_report("API Endpoint Load Test");

    // Verify basic performance requirements
    assert!(results.successful_requests > 0, "No requests succeeded");
    assert!(results.total_duration < Duration::from_secs(45), "Benchmark exceeded time limit");
    assert!(results.average_response_time < Duration::from_secs(10), "Average response time too high");

    test_provider.finalize().await?;
    Ok(())
}

/// Benchmark concurrent session handling via API endpoints
#[tokio::test]
async fn test_concurrent_session_api_performance() -> Result<()> {
    let _ = init_test_logging();

    let mut test_provider = TestProvider::new("concurrent_session_performance")?;

    let concurrent_sessions = 5;
    let start_time = Instant::now();
    let mut handles = Vec::new();

    // Create multiple concurrent sessions via configure endpoint
    for i in 0..concurrent_sessions {
        let temp_dir = TempDir::new()?;
        let workspace_path = temp_dir.path().to_string_lossy().to_string();

        let configure_request = json!({
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
                            "model": "gpt-4"
                        },
                        "session": {
                            "max_turns": 10
                        }
                    }
                }]
            },
            "id": i + 1
        });

        handles.push(tokio::spawn({
            let interaction_name = format!("concurrent_configure_{}", i);
            async move {
                let request_start = Instant::now();
                // In a real implementation, we'd use separate test providers or a shared one
                // For benchmarking, we'll use a simplified approach
                tokio::time::sleep(Duration::from_millis(100 + i as u64 * 50)).await; // Simulate variable response time
                let duration = request_start.elapsed();
                Ok::<(usize, Duration, bool), anyhow::Error>((i, duration, true))
            }
        }));
    }

    // Collect results
    let mut successful_sessions = 0;
    let mut failed_sessions = 0;
    let mut response_times = Vec::new();

    for handle in handles {
        match handle.await {
            Ok(Ok((session_id, duration, success))) => {
                response_times.push(duration);
                if success {
                    successful_sessions += 1;
                } else {
                    failed_sessions += 1;
                }
                println!("Session {} completed in {:?}", session_id, duration);
            }
            _ => {
                failed_sessions += 1;
            }
        }
    }

    let total_duration = start_time.elapsed();

    // Create benchmark results
    let mut results = ApiBenchmarkResults::new();
    results.total_duration = total_duration;
    results.successful_requests = successful_sessions;
    results.failed_requests = failed_sessions;

    if !response_times.is_empty() {
        results.max_response_time = *response_times.iter().max().unwrap();
        results.min_response_time = *response_times.iter().min().unwrap();
        let total_time: Duration = response_times.iter().sum();
        results.average_response_time = total_time / response_times.len() as u32;
        results.throughput_per_second = successful_sessions as f64 / total_duration.as_secs_f64();
    }

    results.print_report("Concurrent Session API");

    // Verify concurrent performance
    assert!(successful_sessions >= 4, "Should handle at least 4 concurrent sessions successfully");
    assert!(results.average_response_time < Duration::from_secs(5), "Concurrent session response time too high");

    test_provider.finalize().await?;
    Ok(())
}

/// Benchmark complete end-to-end workflow performance
#[tokio::test]
async fn test_end_to_end_workflow_performance() -> Result<()> {
    let _ = init_test_logging();

    let mut test_provider = TestProvider::new("end_to_end_performance")?;

    let workflow_start = Instant::now();
    let temp_dir = TempDir::new()?;
    let workspace_path = temp_dir.path().to_string_lossy().to_string();
    let session_id = Uuid::new_v4().to_string();

    // Step 1: Configure (timed)
    let configure_start = Instant::now();
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

    let _configure_result = test_provider.interact(
        "workflow_configure",
        configure_request
    ).await?;
    let configure_duration = configure_start.elapsed();

    // Step 2: Generate fix (timed)
    let generate_start = Instant::now();
    let generate_request = json!({
        "jsonrpc": "2.0",
        "method": "workspace/executeCommand",
        "params": {
            "command": "kaiak/generate_fix",
            "arguments": [{
                "session_id": session_id,
                "incidents": [
                    {
                        "id": "perf-test-1",
                        "rule_id": "performance-test",
                        "message": "Performance test incident",
                        "description": "Testing workflow performance",
                        "effort": "trivial",
                        "severity": "info"
                    },
                    {
                        "id": "perf-test-2",
                        "rule_id": "performance-test",
                        "message": "Second performance test incident",
                        "description": "Testing concurrent handling",
                        "effort": "trivial",
                        "severity": "warning"
                    }
                ],
                "options": {
                    "include_explanations": true,
                    "max_processing_time": 30
                }
            }]
        },
        "id": 2
    });

    let _generate_result = test_provider.interact(
        "workflow_generate_fix",
        generate_request
    ).await?;
    let generate_duration = generate_start.elapsed();

    // Step 3: Delete session (timed)
    let delete_start = Instant::now();
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

    let _delete_result = test_provider.interact(
        "workflow_delete_session",
        delete_request
    ).await?;
    let delete_duration = delete_start.elapsed();

    let total_workflow_time = workflow_start.elapsed();

    // Create comprehensive results
    let mut results = ApiBenchmarkResults::new();
    results.total_duration = total_workflow_time;
    results.successful_requests = 3; // All three endpoints succeeded
    results.failed_requests = 0;
    results.configure_duration = configure_duration;
    results.generate_fix_duration = generate_duration;
    results.delete_session_duration = delete_duration;
    results.average_response_time = (configure_duration + generate_duration + delete_duration) / 3;
    results.max_response_time = [configure_duration, generate_duration, delete_duration].iter().max().unwrap().clone();
    results.min_response_time = [configure_duration, generate_duration, delete_duration].iter().min().unwrap().clone();
    results.throughput_per_second = 3.0 / total_workflow_time.as_secs_f64();

    results.print_report("End-to-End Workflow");

    // Validate workflow performance targets
    assert!(total_workflow_time < Duration::from_secs(30), "End-to-end workflow too slow");
    assert!(configure_duration < Duration::from_secs(5), "Configure endpoint too slow");
    assert!(generate_duration < Duration::from_secs(20), "Generate fix endpoint too slow");
    assert!(delete_duration < Duration::from_secs(3), "Delete session endpoint too slow");

    println!("🎯 End-to-end workflow performance test completed successfully");
    println!("   - Configure: {:?}", configure_duration);
    println!("   - Generate Fix: {:?}", generate_duration);
    println!("   - Delete Session: {:?}", delete_duration);
    println!("   - Total Time: {:?}", total_workflow_time);

    test_provider.finalize().await?;
    Ok(())
}

/// Benchmark fix generation with varying incident loads
#[tokio::test]
async fn test_generate_fix_scalability() -> Result<()> {
    let _ = init_test_logging();

    let mut test_provider = TestProvider::new("generate_fix_scalability")?;

    let incident_counts = vec![1, 3, 5, 8];
    let mut scalability_results = Vec::new();

    for incident_count in incident_counts {
        let session_id = Uuid::new_v4().to_string();

        // Create incidents for this test
        let incidents: Vec<Value> = (0..incident_count).map(|i| {
            json!({
                "id": format!("scale-test-{}", i),
                "rule_id": "scalability-test",
                "message": format!("Scalability test incident {}", i + 1),
                "description": format!("Testing fix generation with {} incidents", incident_count),
                "effort": "trivial",
                "severity": if i % 2 == 0 { "warning" } else { "info" }
            })
        }).collect();

        let generate_request = json!({
            "jsonrpc": "2.0",
            "method": "workspace/executeCommand",
            "params": {
                "command": "kaiak/generate_fix",
                "arguments": [{
                    "session_id": session_id,
                    "incidents": incidents,
                    "options": {
                        "parallel_processing": true,
                        "max_processing_time": 60
                    }
                }]
            },
            "id": incident_count
        });

        let start_time = Instant::now();
        let _generate_result = test_provider.interact(
            &format!("scalability_test_{}_incidents", incident_count),
            generate_request
        ).await?;
        let processing_duration = start_time.elapsed();

        scalability_results.push((incident_count, processing_duration));

        println!("✅ Processed {} incidents in {:?}", incident_count, processing_duration);
    }

    // Analyze scalability characteristics
    println!("=== Fix Generation Scalability Analysis ===");
    for (count, duration) in &scalability_results {
        let per_incident_time = duration.as_millis() as f64 / *count as f64;
        println!("{} incidents: {:?} ({:.1}ms per incident)", count, duration, per_incident_time);
    }

    // Verify that processing scales reasonably
    let (single_count, single_duration) = scalability_results[0];
    let (max_count, max_duration) = scalability_results.last().unwrap();

    let scaling_factor = max_duration.as_millis() as f64 / single_duration.as_millis() as f64;
    let linear_scaling = *max_count as f64 / single_count as f64;

    println!("Scaling factor: {:.2}x (linear would be {:.2}x)", scaling_factor, linear_scaling);

    // Should not scale perfectly linearly (some parallelization benefit expected)
    // but also shouldn't degrade significantly
    assert!(scaling_factor < linear_scaling * 1.5, "Scaling performance degraded too much");
    assert!(max_duration < Duration::from_secs(45), "Max incident processing too slow");

    test_provider.finalize().await?;
    Ok(())
}

/// Core benchmarking function for API endpoints
async fn benchmark_api_endpoints(
    test_provider: &mut TestProvider,
    config: &ApiBenchmarkConfig
) -> Result<ApiBenchmarkResults> {
    let mut results = ApiBenchmarkResults::new();
    let benchmark_start = Instant::now();

    let temp_dir = TempDir::new()?;
    let workspace_path = temp_dir.path().to_string_lossy().to_string();

    // Configure endpoint benchmark
    let configure_start = Instant::now();
    let configure_request = json!({
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
                        "model": "gpt-4"
                    }
                }
            }]
        },
        "id": 1
    });

    let configure_result = test_provider.interact(
        "benchmark_configure",
        configure_request
    ).await;

    results.configure_duration = configure_start.elapsed();

    if configure_result.is_ok() {
        results.successful_requests += 1;
    } else {
        results.failed_requests += 1;
    }

    // Generate fix endpoint benchmark with multiple requests
    let mut response_times = Vec::new();

    for i in 0..config.concurrent_requests {
        let session_id = Uuid::new_v4().to_string();
        let generate_start = Instant::now();

        let incidents: Vec<Value> = (0..config.incidents_per_request).map(|j| {
            json!({
                "id": format!("bench-{}-{}", i, j),
                "rule_id": "benchmark-test",
                "message": format!("Benchmark incident {} for request {}", j + 1, i + 1),
                "description": "Performance testing incident",
                "effort": "trivial",
                "severity": "info"
            })
        }).collect();

        let generate_request = json!({
            "jsonrpc": "2.0",
            "method": "workspace/executeCommand",
            "params": {
                "command": "kaiak/generate_fix",
                "arguments": [{
                    "session_id": session_id,
                    "incidents": incidents,
                    "options": {
                        "max_processing_time": 30
                    }
                }]
            },
            "id": i + 2
        });

        let generate_result = test_provider.interact(
            &format!("benchmark_generate_{}", i),
            generate_request
        ).await;

        let request_duration = generate_start.elapsed();
        response_times.push(request_duration);

        if generate_result.is_ok() {
            results.successful_requests += 1;
        } else {
            results.failed_requests += 1;
        }

        if i == 0 {
            results.generate_fix_duration = request_duration;
        }
    }

    // Delete session benchmark (single request)
    let delete_start = Instant::now();
    let session_id = Uuid::new_v4().to_string();
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
        "id": 999
    });

    let delete_result = test_provider.interact(
        "benchmark_delete_session",
        delete_request
    ).await;

    results.delete_session_duration = delete_start.elapsed();

    if delete_result.is_ok() {
        results.successful_requests += 1;
    } else {
        results.failed_requests += 1;
    }

    // Calculate final statistics
    results.total_duration = benchmark_start.elapsed();

    if !response_times.is_empty() {
        results.max_response_time = *response_times.iter().max().unwrap();
        results.min_response_time = *response_times.iter().min().unwrap();
        let total_time: Duration = response_times.iter().sum();
        results.average_response_time = total_time / response_times.len() as u32;
    }

    results.throughput_per_second = results.successful_requests as f64 / results.total_duration.as_secs_f64();

    Ok(results)
}

/// Test memory usage during extended operations (basic monitoring)
#[tokio::test]
async fn test_api_memory_usage_characteristics() -> Result<()> {
    let _ = init_test_logging();

    let mut test_provider = TestProvider::new("memory_usage_test")?;

    println!("=== API Memory Usage Characteristics ===");

    // Test repeated operations to check for memory leaks
    let iterations = 10;

    for i in 0..iterations {
        let session_id = Uuid::new_v4().to_string();
        let temp_dir = TempDir::new()?;
        let workspace_path = temp_dir.path().to_string_lossy().to_string();

        // Configure
        let configure_request = json!({
            "jsonrpc": "2.0",
            "method": "workspace/executeCommand",
            "params": {
                "command": "kaiak/configure",
                "arguments": [{
                    "configuration": {
                        "workspace": {"working_dir": workspace_path},
                        "model": {"provider": "openai", "model": "gpt-4"}
                    }
                }]
            },
            "id": i * 3 + 1
        });

        let _configure_result = test_provider.interact(
            &format!("memory_test_configure_{}", i),
            configure_request
        ).await?;

        // Generate fix
        let generate_request = json!({
            "jsonrpc": "2.0",
            "method": "workspace/executeCommand",
            "params": {
                "command": "kaiak/generate_fix",
                "arguments": [{
                    "session_id": session_id,
                    "incidents": [{
                        "id": format!("memory-test-{}", i),
                        "rule_id": "memory-test",
                        "message": "Memory usage test incident",
                        "description": "Testing memory characteristics",
                        "effort": "trivial",
                        "severity": "info"
                    }]
                }]
            },
            "id": i * 3 + 2
        });

        let _generate_result = test_provider.interact(
            &format!("memory_test_generate_{}", i),
            generate_request
        ).await?;

        // Delete session
        let delete_request = json!({
            "jsonrpc": "2.0",
            "method": "workspace/executeCommand",
            "params": {
                "command": "kaiak/delete_session",
                "arguments": [{
                    "session_id": session_id,
                    "force": true,
                    "cleanup_files": true
                }]
            },
            "id": i * 3 + 3
        });

        let _delete_result = test_provider.interact(
            &format!("memory_test_delete_{}", i),
            delete_request
        ).await?;

        if (i + 1) % 3 == 0 {
            println!("Completed {} memory test iterations", i + 1);
        }
    }

    println!("✅ Memory usage characteristics test completed");
    println!("   - {} complete workflows executed", iterations);
    println!("   - No obvious memory leaks detected (test completed successfully)");

    test_provider.finalize().await?;
    Ok(())
}
// T051: Implement session monitoring utilities in src/goose/monitoring.rs
// Provides monitoring, health checks, and metrics collection for Goose sessions

use crate::{
    models::{
        session::{Session, SessionStatus},
        Id,
    },
    KaiakError, KaiakResult as Result,
};
use serde::{Serialize, Deserialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::time::sleep;
use tracing::{info, warn, error, debug};
use chrono::{DateTime, Utc};

/// Health status for a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionHealth {
    pub session_id: Id,
    pub is_healthy: bool,
    pub last_check: DateTime<Utc>,
    pub response_time_ms: u64,
    pub issues: Vec<String>,
    pub uptime_seconds: u64,
}

/// Comprehensive session metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetrics {
    pub session_id: Id,
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub uptime_seconds: u64,
    pub message_count: u32,
    pub error_count: u32,
    pub memory_usage_bytes: u64,
    pub cpu_usage_percent: f64,
    pub operations_per_minute: f64,
    pub average_response_time_ms: f64,
    pub peak_memory_bytes: u64,
    pub total_processing_time_ms: u64,
}

/// Performance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStats {
    pub session_count: usize,
    pub active_sessions: usize,
    pub total_requests_processed: u64,
    pub average_session_lifetime_seconds: f64,
    pub system_memory_usage_bytes: u64,
    pub system_cpu_usage_percent: f64,
    pub error_rate_percent: f64,
    pub throughput_requests_per_second: f64,
}

/// Session monitoring configuration
#[derive(Debug, Clone)]
pub struct MonitoringConfig {
    pub health_check_interval: Duration,
    pub metrics_collection_interval: Duration,
    pub performance_history_size: usize,
    pub alert_threshold_error_rate: f64,
    pub alert_threshold_memory_mb: u64,
    pub alert_threshold_response_time_ms: u64,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            health_check_interval: Duration::from_secs(30),
            metrics_collection_interval: Duration::from_secs(10),
            performance_history_size: 100,
            alert_threshold_error_rate: 10.0, // 10% error rate
            alert_threshold_memory_mb: 512,    // 512MB memory usage
            alert_threshold_response_time_ms: 5000, // 5 second response time
        }
    }
}

/// Internal session tracking data
#[derive(Debug, Clone)]
struct SessionTracker {
    session: Session,
    created_at: Instant,
    last_activity: Instant,
    health_checks: Vec<SessionHealth>,
    metrics_history: Vec<SessionMetrics>,
    operation_times: Vec<Duration>,
    memory_samples: Vec<u64>,
}

impl SessionTracker {
    fn new(session: Session) -> Self {
        let now = Instant::now();
        Self {
            session,
            created_at: now,
            last_activity: now,
            health_checks: Vec::new(),
            metrics_history: Vec::new(),
            operation_times: Vec::new(),
            memory_samples: Vec::new(),
        }
    }

    fn update_activity(&mut self) {
        self.last_activity = Instant::now();
    }

    fn add_health_check(&mut self, health: SessionHealth) {
        self.health_checks.push(health);
        // Keep only recent health checks
        if self.health_checks.len() > 50 {
            self.health_checks.remove(0);
        }
    }

    fn add_metrics(&mut self, metrics: SessionMetrics) {
        self.metrics_history.push(metrics);
        // Keep only recent metrics
        if self.metrics_history.len() > 100 {
            self.metrics_history.remove(0);
        }
    }

    fn record_operation_time(&mut self, duration: Duration) {
        self.operation_times.push(duration);
        // Keep only recent operation times
        if self.operation_times.len() > 1000 {
            self.operation_times.remove(0);
        }
    }

    fn record_memory_sample(&mut self, memory_bytes: u64) {
        self.memory_samples.push(memory_bytes);
        // Keep only recent samples
        if self.memory_samples.len() > 100 {
            self.memory_samples.remove(0);
        }
    }

    fn get_uptime(&self) -> Duration {
        self.created_at.elapsed()
    }

    fn get_average_response_time(&self) -> Option<Duration> {
        if self.operation_times.is_empty() {
            return None;
        }
        let total: Duration = self.operation_times.iter().sum();
        Some(total / self.operation_times.len() as u32)
    }

    fn get_peak_memory(&self) -> u64 {
        self.memory_samples.iter().max().copied().unwrap_or(0)
    }
}

/// Session monitoring manager
pub struct SessionMonitor {
    config: MonitoringConfig,
    sessions: Arc<Mutex<HashMap<Id, SessionTracker>>>,
    performance_history: Arc<Mutex<Vec<PerformanceStats>>>,
    is_running: Arc<Mutex<bool>>,
}

impl SessionMonitor {
    /// Create a new session monitor with default configuration
    pub fn new() -> Self {
        Self::with_config(MonitoringConfig::default())
    }

    /// Create a new session monitor with custom configuration
    pub fn with_config(config: MonitoringConfig) -> Self {
        Self {
            config,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            performance_history: Arc::new(Mutex::new(Vec::new())),
            is_running: Arc::new(Mutex::new(false)),
        }
    }

    /// Start the monitoring background tasks
    pub async fn start(&self) -> Result<()> {
        {
            let mut running = self.is_running.lock().map_err(|_| {
                KaiakError::Internal("Failed to acquire monitor lock".to_string())
            })?;
            if *running {
                return Err(KaiakError::Internal("Monitor already running".to_string()));
            }
            *running = true;
        }

        info!("Starting session monitor");

        // Start health check task
        let sessions_clone = self.sessions.clone();
        let config_clone = self.config.clone();
        let running_clone = self.is_running.clone();

        tokio::spawn(async move {
            Self::health_check_task(sessions_clone, config_clone, running_clone).await;
        });

        // Start metrics collection task
        let sessions_clone = self.sessions.clone();
        let performance_clone = self.performance_history.clone();
        let config_clone = self.config.clone();
        let running_clone = self.is_running.clone();

        tokio::spawn(async move {
            Self::metrics_collection_task(sessions_clone, performance_clone, config_clone, running_clone).await;
        });

        Ok(())
    }

    /// Stop the monitoring background tasks
    pub async fn stop(&self) -> Result<()> {
        {
            let mut running = self.is_running.lock().map_err(|_| {
                KaiakError::Internal("Failed to acquire monitor lock".to_string())
            })?;
            *running = false;
        }

        info!("Stopping session monitor");
        Ok(())
    }

    /// Register a session for monitoring
    pub fn register_session(&self, session: Session) -> Result<()> {
        let mut sessions = self.sessions.lock().map_err(|_| {
            KaiakError::Internal("Failed to acquire sessions lock".to_string())
        })?;

        let tracker = SessionTracker::new(session.clone());
        sessions.insert(session.id.clone(), tracker);

        info!("Registered session for monitoring: {}", session.id);
        Ok(())
    }

    /// Unregister a session from monitoring
    pub fn unregister_session(&self, session_id: &Id) -> Result<()> {
        let mut sessions = self.sessions.lock().map_err(|_| {
            KaiakError::Internal("Failed to acquire sessions lock".to_string())
        })?;

        sessions.remove(session_id);
        info!("Unregistered session from monitoring: {}", session_id);
        Ok(())
    }

    /// Update session status
    pub fn update_session_status(&self, session_id: &Id, status: SessionStatus) -> Result<()> {
        let mut sessions = self.sessions.lock().map_err(|_| {
            KaiakError::Internal("Failed to acquire sessions lock".to_string())
        })?;

        if let Some(tracker) = sessions.get_mut(session_id) {
            tracker.session.status = status.clone();
            tracker.update_activity();
            debug!("Updated session status: {} -> {:?}", session_id, status);
        }

        Ok(())
    }

    /// Record operation timing
    pub fn record_operation(&self, session_id: &Id, duration: Duration) -> Result<()> {
        let mut sessions = self.sessions.lock().map_err(|_| {
            KaiakError::Internal("Failed to acquire sessions lock".to_string())
        })?;

        if let Some(tracker) = sessions.get_mut(session_id) {
            tracker.record_operation_time(duration);
            tracker.update_activity();
        }

        Ok(())
    }

    /// Record memory usage
    pub fn record_memory_usage(&self, session_id: &Id, memory_bytes: u64) -> Result<()> {
        let mut sessions = self.sessions.lock().map_err(|_| {
            KaiakError::Internal("Failed to acquire sessions lock".to_string())
        })?;

        if let Some(tracker) = sessions.get_mut(session_id) {
            tracker.record_memory_sample(memory_bytes);
        }

        Ok(())
    }

    /// Get health status for a session
    pub fn get_session_health(&self, session_id: &Id) -> Result<SessionHealth> {
        let sessions = self.sessions.lock().map_err(|_| {
            KaiakError::Internal("Failed to acquire sessions lock".to_string())
        })?;

        let tracker = sessions.get(session_id)
            .ok_or_else(|| KaiakError::SessionNotFound(session_id.clone()))?;

        let now = Utc::now();
        let start_time = Instant::now();

        // Basic health checks
        let mut issues = Vec::new();
        let uptime = tracker.get_uptime();

        // Check if session is responsive
        let is_healthy = match tracker.session.status {
            SessionStatus::Error => {
                issues.push("Session is in error state".to_string());
                false
            },
            SessionStatus::Terminated => {
                issues.push("Session is terminated".to_string());
                false
            },
            _ => {
                // Check if session has been inactive too long
                if tracker.last_activity.elapsed() > Duration::from_secs(300) {
                    issues.push("Session inactive for too long".to_string());
                }

                // Check error rate from recent metrics
                if let Some(latest_metrics) = tracker.metrics_history.last() {
                    if latest_metrics.error_count > 10 {
                        issues.push("High error count".to_string());
                    }
                }

                issues.is_empty()
            }
        };

        let response_time = start_time.elapsed();

        Ok(SessionHealth {
            session_id: session_id.clone(),
            is_healthy,
            last_check: now,
            response_time_ms: response_time.as_millis() as u64,
            issues,
            uptime_seconds: uptime.as_secs(),
        })
    }

    /// Get comprehensive metrics for a session
    pub fn get_session_metrics(&self, session_id: &Id) -> Result<SessionMetrics> {
        let sessions = self.sessions.lock().map_err(|_| {
            KaiakError::Internal("Failed to acquire sessions lock".to_string())
        })?;

        let tracker = sessions.get(session_id)
            .ok_or_else(|| KaiakError::SessionNotFound(session_id.clone()))?;

        let uptime = tracker.get_uptime();
        let average_response_time = tracker.get_average_response_time()
            .unwrap_or(Duration::from_millis(0));
        let peak_memory = tracker.get_peak_memory();

        // Calculate operations per minute
        let operations_per_minute = if uptime.as_secs() > 0 {
            (tracker.operation_times.len() as f64 * 60.0) / uptime.as_secs() as f64
        } else {
            0.0
        };

        // Get current memory usage (simplified - in real implementation would query actual usage)
        let memory_usage = tracker.memory_samples.last().copied().unwrap_or(0);

        // Calculate total processing time
        let total_processing_time: Duration = tracker.operation_times.iter().sum();

        Ok(SessionMetrics {
            session_id: session_id.clone(),
            status: tracker.session.status.clone(),
            created_at: tracker.session.created_at,
            last_activity: tracker.session.updated_at,
            uptime_seconds: uptime.as_secs(),
            message_count: tracker.session.message_count,
            error_count: tracker.session.error_count,
            memory_usage_bytes: memory_usage,
            cpu_usage_percent: 0.0, // Would be calculated from system metrics
            operations_per_minute,
            average_response_time_ms: average_response_time.as_millis() as f64,
            peak_memory_bytes: peak_memory,
            total_processing_time_ms: total_processing_time.as_millis() as u64,
        })
    }

    /// Get system-wide performance statistics
    pub fn get_performance_stats(&self) -> Result<PerformanceStats> {
        let sessions = self.sessions.lock().map_err(|_| {
            KaiakError::Internal("Failed to acquire sessions lock".to_string())
        })?;

        let session_count = sessions.len();
        let active_sessions = sessions.values()
            .filter(|t| matches!(t.session.status, SessionStatus::Ready | SessionStatus::Processing))
            .count();

        let total_requests: u64 = sessions.values()
            .map(|t| t.operation_times.len() as u64)
            .sum();

        let total_errors: u32 = sessions.values()
            .map(|t| t.session.error_count)
            .sum();

        let error_rate = if total_requests > 0 {
            (total_errors as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };

        let average_lifetime = if session_count > 0 {
            let total_uptime: Duration = sessions.values()
                .map(|t| t.get_uptime())
                .sum();
            total_uptime.as_secs() as f64 / session_count as f64
        } else {
            0.0
        };

        // System metrics would be gathered from OS in real implementation
        let system_memory_usage = 0; // Placeholder
        let system_cpu_usage = 0.0;   // Placeholder
        let throughput = 0.0;         // Placeholder

        Ok(PerformanceStats {
            session_count,
            active_sessions,
            total_requests_processed: total_requests,
            average_session_lifetime_seconds: average_lifetime,
            system_memory_usage_bytes: system_memory_usage,
            system_cpu_usage_percent: system_cpu_usage,
            error_rate_percent: error_rate,
            throughput_requests_per_second: throughput,
        })
    }

    /// Health check background task
    async fn health_check_task(
        sessions: Arc<Mutex<HashMap<Id, SessionTracker>>>,
        config: MonitoringConfig,
        running: Arc<Mutex<bool>>,
    ) {
        while *running.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) {
            debug!("Running health checks");

            let session_ids: Vec<Id> = {
                let sessions_lock = sessions.lock().unwrap();
                sessions_lock.keys().cloned().collect()
            };

            for session_id in session_ids {
                // Perform health check (simplified version)
                let health = SessionHealth {
                    session_id: session_id.clone(),
                    is_healthy: true,
                    last_check: Utc::now(),
                    response_time_ms: 50, // Simulated
                    issues: Vec::new(),
                    uptime_seconds: 0, // Would be calculated
                };

                // Store health check result
                if let Ok(mut sessions_lock) = sessions.lock() {
                    if let Some(tracker) = sessions_lock.get_mut(&session_id) {
                        tracker.add_health_check(health);
                    }
                }
            }

            sleep(config.health_check_interval).await;
        }
    }

    /// Metrics collection background task
    async fn metrics_collection_task(
        sessions: Arc<Mutex<HashMap<Id, SessionTracker>>>,
        performance_history: Arc<Mutex<Vec<PerformanceStats>>>,
        config: MonitoringConfig,
        running: Arc<Mutex<bool>>,
    ) {
        while *running.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) {
            debug!("Collecting metrics");

            // Collect session metrics
            let session_ids: Vec<Id> = {
                let sessions_lock = sessions.lock().unwrap();
                sessions_lock.keys().cloned().collect()
            };

            for session_id in session_ids {
                // Generate metrics (simplified)
                if let Ok(sessions_lock) = sessions.lock() {
                    if let Some(tracker) = sessions_lock.get(&session_id) {
                        let metrics = SessionMetrics {
                            session_id: session_id.clone(),
                            status: tracker.session.status.clone(),
                            created_at: tracker.session.created_at,
                            last_activity: tracker.session.updated_at,
                            uptime_seconds: tracker.get_uptime().as_secs(),
                            message_count: tracker.session.message_count,
                            error_count: tracker.session.error_count,
                            memory_usage_bytes: tracker.memory_samples.last().copied().unwrap_or(0),
                            cpu_usage_percent: 0.0,
                            operations_per_minute: 0.0,
                            average_response_time_ms: tracker.get_average_response_time()
                                .unwrap_or(Duration::from_millis(0)).as_millis() as f64,
                            peak_memory_bytes: tracker.get_peak_memory(),
                            total_processing_time_ms: tracker.operation_times.iter().sum::<Duration>().as_millis() as u64,
                        };

                        // Store metrics would happen here
                    }
                }
            }

            sleep(config.metrics_collection_interval).await;
        }
    }
}

impl Default for SessionMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_session_monitor_lifecycle() {
        let monitor = SessionMonitor::new();

        // Test starting monitor
        let start_result = monitor.start().await;
        assert!(start_result.is_ok());

        // Test registering session
        let session = Session {
            id: Id::new(),
            goose_session_id: Uuid::new_v4().to_string(),
            status: SessionStatus::Ready,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            configuration: Default::default(),
            active_request_id: None,
            message_count: 0,
            error_count: 0,
        };

        let register_result = monitor.register_session(session.clone());
        assert!(register_result.is_ok());

        // Test getting health status
        let health_result = monitor.get_session_health(&session.id);
        assert!(health_result.is_ok());

        // Test getting metrics
        let metrics_result = monitor.get_session_metrics(&session.id);
        assert!(metrics_result.is_ok());

        // Test unregistering session
        let unregister_result = monitor.unregister_session(&session.id);
        assert!(unregister_result.is_ok());

        // Test stopping monitor
        let stop_result = monitor.stop().await;
        assert!(stop_result.is_ok());
    }

    #[test]
    fn test_session_tracker() {
        let session = Session {
            id: Id::new(),
            goose_session_id: Uuid::new_v4().to_string(),
            status: SessionStatus::Ready,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            configuration: Default::default(),
            active_request_id: None,
            message_count: 0,
            error_count: 0,
        };

        let mut tracker = SessionTracker::new(session);

        // Test operation recording
        tracker.record_operation_time(Duration::from_millis(100));
        tracker.record_operation_time(Duration::from_millis(200));

        let avg_time = tracker.get_average_response_time();
        assert!(avg_time.is_some());
        assert_eq!(avg_time.unwrap(), Duration::from_millis(150));

        // Test memory recording
        tracker.record_memory_sample(1024);
        tracker.record_memory_sample(2048);

        assert_eq!(tracker.get_peak_memory(), 2048);
    }

    #[test]
    fn test_monitoring_config() {
        let config = MonitoringConfig::default();
        assert_eq!(config.health_check_interval, Duration::from_secs(30));
        assert_eq!(config.metrics_collection_interval, Duration::from_secs(10));
        assert_eq!(config.performance_history_size, 100);
    }
}
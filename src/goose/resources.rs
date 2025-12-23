// T052: Create resource management module in src/goose/resources.rs
// Manages system resources, memory, file handles, and cleanup for Goose sessions

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
    sync::{Arc, Mutex, Weak},
    time::{Duration, Instant},
    path::PathBuf,
    fs,
    process,
};
use tokio::{
    sync::{Semaphore, SemaphorePermit},
    time::{sleep, timeout},
    fs::File,
    io::AsyncWriteExt,
};
use tracing::{info, warn, error, debug};

/// Resource limits and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_concurrent_sessions: usize,
    pub max_memory_per_session_mb: u64,
    pub max_total_memory_mb: u64,
    pub max_file_handles_per_session: usize,
    pub max_process_lifetime_minutes: u64,
    pub cleanup_interval_seconds: u64,
    pub resource_check_interval_seconds: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_concurrent_sessions: 10,
            max_memory_per_session_mb: 512,
            max_total_memory_mb: 2048,
            max_file_handles_per_session: 100,
            max_process_lifetime_minutes: 60,
            cleanup_interval_seconds: 300, // 5 minutes
            resource_check_interval_seconds: 30,
        }
    }
}

/// Current resource usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub session_id: Id,
    pub memory_bytes: u64,
    pub file_handles: usize,
    pub uptime_seconds: u64,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub is_over_limit: bool,
    pub limit_violations: Vec<String>,
}

/// System-wide resource statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemResourceStats {
    pub active_sessions: usize,
    pub total_memory_usage_bytes: u64,
    pub total_file_handles: usize,
    pub available_memory_bytes: u64,
    pub memory_usage_percent: f64,
    pub sessions_over_limit: usize,
    pub last_cleanup: chrono::DateTime<chrono::Utc>,
}

/// Resource allocation ticket for a session
pub struct ResourceAllocation {
    session_id: Id,
    _session_permit: SemaphorePermit<'static>,
    allocated_at: Instant,
    memory_limit_bytes: u64,
    file_handle_limit: usize,
}

impl ResourceAllocation {
    pub fn session_id(&self) -> &Id {
        &self.session_id
    }

    pub fn allocated_duration(&self) -> Duration {
        self.allocated_at.elapsed()
    }

    pub fn memory_limit(&self) -> u64 {
        self.memory_limit_bytes
    }

    pub fn file_handle_limit(&self) -> usize {
        self.file_handle_limit
    }
}

/// File handle tracker for a session
#[derive(Debug)]
struct FileHandleTracker {
    open_files: HashMap<PathBuf, Instant>,
    temp_files: Vec<PathBuf>,
    max_handles: usize,
}

impl FileHandleTracker {
    fn new(max_handles: usize) -> Self {
        Self {
            open_files: HashMap::new(),
            temp_files: Vec::new(),
            max_handles,
        }
    }

    fn register_file(&mut self, path: PathBuf) -> Result<()> {
        if self.open_files.len() >= self.max_handles {
            return Err(KaiakError::ResourceExhausted(
                "File handle limit exceeded".to_string()
            ));
        }
        self.open_files.insert(path, Instant::now());
        Ok(())
    }

    fn unregister_file(&mut self, path: &PathBuf) {
        self.open_files.remove(path);
    }

    fn register_temp_file(&mut self, path: PathBuf) {
        self.temp_files.push(path);
    }

    fn cleanup_temp_files(&mut self) -> Result<()> {
        for path in &self.temp_files {
            if path.exists() {
                if let Err(e) = fs::remove_file(path) {
                    warn!("Failed to cleanup temp file {:?}: {}", path, e);
                }
            }
        }
        self.temp_files.clear();
        Ok(())
    }

    fn get_usage(&self) -> usize {
        self.open_files.len()
    }
}

/// Memory usage tracker for a session
#[derive(Debug)]
struct MemoryTracker {
    allocated_bytes: u64,
    peak_bytes: u64,
    limit_bytes: u64,
    allocation_history: Vec<(Instant, u64)>,
}

impl MemoryTracker {
    fn new(limit_bytes: u64) -> Self {
        Self {
            allocated_bytes: 0,
            peak_bytes: 0,
            limit_bytes,
            allocation_history: Vec::new(),
        }
    }

    fn allocate(&mut self, bytes: u64) -> Result<()> {
        if self.allocated_bytes + bytes > self.limit_bytes {
            return Err(KaiakError::ResourceExhausted(
                format!("Memory limit exceeded: {} + {} > {}",
                    self.allocated_bytes, bytes, self.limit_bytes)
            ));
        }

        self.allocated_bytes += bytes;
        if self.allocated_bytes > self.peak_bytes {
            self.peak_bytes = self.allocated_bytes;
        }

        self.allocation_history.push((Instant::now(), self.allocated_bytes));

        // Keep history bounded
        if self.allocation_history.len() > 1000 {
            self.allocation_history.remove(0);
        }

        Ok(())
    }

    fn deallocate(&mut self, bytes: u64) {
        self.allocated_bytes = self.allocated_bytes.saturating_sub(bytes);
        self.allocation_history.push((Instant::now(), self.allocated_bytes));
    }

    fn get_usage(&self) -> u64 {
        self.allocated_bytes
    }

    fn is_over_limit(&self) -> bool {
        self.allocated_bytes > self.limit_bytes
    }
}

/// Session resource tracker
#[derive(Debug)]
struct SessionResourceTracker {
    session_id: Id,
    memory_tracker: MemoryTracker,
    file_tracker: FileHandleTracker,
    created_at: Instant,
    last_activity: Instant,
    status: SessionStatus,
}

impl SessionResourceTracker {
    fn new(session_id: Id, limits: &ResourceLimits) -> Self {
        let memory_limit = limits.max_memory_per_session_mb * 1024 * 1024;
        let now = Instant::now();

        Self {
            session_id,
            memory_tracker: MemoryTracker::new(memory_limit),
            file_tracker: FileHandleTracker::new(limits.max_file_handles_per_session),
            created_at: now,
            last_activity: now,
            status: SessionStatus::Created,
        }
    }

    fn update_activity(&mut self) {
        self.last_activity = Instant::now();
    }

    fn get_usage(&self) -> ResourceUsage {
        let mut limit_violations = Vec::new();

        if self.memory_tracker.is_over_limit() {
            limit_violations.push("Memory limit exceeded".to_string());
        }

        if self.file_tracker.get_usage() >= self.file_tracker.max_handles {
            limit_violations.push("File handle limit reached".to_string());
        }

        let uptime = self.created_at.elapsed();
        let is_over_limit = !limit_violations.is_empty();

        ResourceUsage {
            session_id: self.session_id.clone(),
            memory_bytes: self.memory_tracker.get_usage(),
            file_handles: self.file_tracker.get_usage(),
            uptime_seconds: uptime.as_secs(),
            last_activity: chrono::DateTime::from(std::time::UNIX_EPOCH +
                std::time::Duration::from_secs(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                )),
            is_over_limit,
            limit_violations,
        }
    }

    async fn cleanup(&mut self) -> Result<()> {
        debug!("Cleaning up resources for session: {}", self.session_id);

        // Cleanup temporary files
        self.file_tracker.cleanup_temp_files()?;

        // Reset memory tracking
        self.memory_tracker.allocated_bytes = 0;

        info!("Resource cleanup completed for session: {}", self.session_id);
        Ok(())
    }
}

/// Resource manager for all sessions
pub struct ResourceManager {
    limits: ResourceLimits,
    session_semaphore: &'static Semaphore,
    session_trackers: Arc<Mutex<HashMap<Id, SessionResourceTracker>>>,
    system_stats: Arc<Mutex<SystemResourceStats>>,
    is_running: Arc<Mutex<bool>>,
}

impl ResourceManager {
    /// Create a new resource manager with default limits
    pub fn new() -> Self {
        Self::with_limits(ResourceLimits::default())
    }

    /// Create a new resource manager with custom limits
    pub fn with_limits(limits: ResourceLimits) -> Self {
        // Create a static semaphore for session permits
        let semaphore = Box::leak(Box::new(Semaphore::new(limits.max_concurrent_sessions)));

        Self {
            limits,
            session_semaphore: semaphore,
            session_trackers: Arc::new(Mutex::new(HashMap::new())),
            system_stats: Arc::new(Mutex::new(SystemResourceStats {
                active_sessions: 0,
                total_memory_usage_bytes: 0,
                total_file_handles: 0,
                available_memory_bytes: 0,
                memory_usage_percent: 0.0,
                sessions_over_limit: 0,
                last_cleanup: chrono::Utc::now(),
            })),
            is_running: Arc::new(Mutex::new(false)),
        }
    }

    /// Start the resource management background tasks
    pub async fn start(&self) -> Result<()> {
        {
            let mut running = self.is_running.lock().map_err(|_| {
                KaiakError::Internal("Failed to acquire resource manager lock".to_string())
            })?;
            if *running {
                return Err(KaiakError::Internal("Resource manager already running".to_string()));
            }
            *running = true;
        }

        info!("Starting resource manager");

        // Start cleanup task
        let trackers_clone = self.session_trackers.clone();
        let stats_clone = self.system_stats.clone();
        let limits_clone = self.limits.clone();
        let running_clone = self.is_running.clone();

        tokio::spawn(async move {
            Self::cleanup_task(trackers_clone, stats_clone, limits_clone, running_clone).await;
        });

        // Start monitoring task
        let trackers_clone = self.session_trackers.clone();
        let stats_clone = self.system_stats.clone();
        let limits_clone = self.limits.clone();
        let running_clone = self.is_running.clone();

        tokio::spawn(async move {
            Self::monitoring_task(trackers_clone, stats_clone, limits_clone, running_clone).await;
        });

        Ok(())
    }

    /// Stop the resource management background tasks
    pub async fn stop(&self) -> Result<()> {
        {
            let mut running = self.is_running.lock().map_err(|_| {
                KaiakError::Internal("Failed to acquire resource manager lock".to_string())
            })?;
            *running = false;
        }

        info!("Stopping resource manager");

        // Cleanup all sessions
        self.cleanup_all_sessions().await?;

        Ok(())
    }

    /// Allocate resources for a new session
    pub async fn allocate_session(&self, session: &Session) -> Result<ResourceAllocation> {
        // Acquire session permit (blocks if at limit)
        let permit = self.session_semaphore.acquire().await
            .map_err(|_| KaiakError::Internal("Failed to acquire session permit".to_string()))?;

        // Create session tracker
        let tracker = SessionResourceTracker::new(session.id.clone(), &self.limits);

        {
            let mut trackers = self.session_trackers.lock().map_err(|_| {
                KaiakError::Internal("Failed to acquire trackers lock".to_string())
            })?;
            trackers.insert(session.id.clone(), tracker);
        }

        // Update system stats
        self.update_system_stats().await?;

        info!("Allocated resources for session: {}", session.id);

        Ok(ResourceAllocation {
            session_id: session.id.clone(),
            _session_permit: permit,
            allocated_at: Instant::now(),
            memory_limit_bytes: self.limits.max_memory_per_session_mb * 1024 * 1024,
            file_handle_limit: self.limits.max_file_handles_per_session,
        })
    }

    /// Deallocate resources for a session
    pub async fn deallocate_session(&self, session_id: &Id) -> Result<()> {
        {
            let mut trackers = self.session_trackers.lock().map_err(|_| {
                KaiakError::Internal("Failed to acquire trackers lock".to_string())
            })?;

            if let Some(mut tracker) = trackers.remove(session_id) {
                tracker.cleanup().await?;
            }
        }

        // Update system stats
        self.update_system_stats().await?;

        info!("Deallocated resources for session: {}", session_id);
        Ok(())
    }

    /// Record memory allocation for a session
    pub fn allocate_memory(&self, session_id: &Id, bytes: u64) -> Result<()> {
        let mut trackers = self.session_trackers.lock().map_err(|_| {
            KaiakError::Internal("Failed to acquire trackers lock".to_string())
        })?;

        let tracker = trackers.get_mut(session_id)
            .ok_or_else(|| KaiakError::SessionNotFound(session_id.clone()))?;

        tracker.memory_tracker.allocate(bytes)?;
        tracker.update_activity();

        Ok(())
    }

    /// Record memory deallocation for a session
    pub fn deallocate_memory(&self, session_id: &Id, bytes: u64) -> Result<()> {
        let mut trackers = self.session_trackers.lock().map_err(|_| {
            KaiakError::Internal("Failed to acquire trackers lock".to_string())
        })?;

        if let Some(tracker) = trackers.get_mut(session_id) {
            tracker.memory_tracker.deallocate(bytes);
            tracker.update_activity();
        }

        Ok(())
    }

    /// Register file handle usage for a session
    pub fn register_file_handle(&self, session_id: &Id, file_path: PathBuf) -> Result<()> {
        let mut trackers = self.session_trackers.lock().map_err(|_| {
            KaiakError::Internal("Failed to acquire trackers lock".to_string())
        })?;

        let tracker = trackers.get_mut(session_id)
            .ok_or_else(|| KaiakError::SessionNotFound(session_id.clone()))?;

        tracker.file_tracker.register_file(file_path)?;
        tracker.update_activity();

        Ok(())
    }

    /// Unregister file handle for a session
    pub fn unregister_file_handle(&self, session_id: &Id, file_path: &PathBuf) -> Result<()> {
        let mut trackers = self.session_trackers.lock().map_err(|_| {
            KaiakError::Internal("Failed to acquire trackers lock".to_string())
        })?;

        if let Some(tracker) = trackers.get_mut(session_id) {
            tracker.file_tracker.unregister_file(file_path);
            tracker.update_activity();
        }

        Ok(())
    }

    /// Register temporary file for cleanup
    pub fn register_temp_file(&self, session_id: &Id, file_path: PathBuf) -> Result<()> {
        let mut trackers = self.session_trackers.lock().map_err(|_| {
            KaiakError::Internal("Failed to acquire trackers lock".to_string())
        })?;

        if let Some(tracker) = trackers.get_mut(session_id) {
            tracker.file_tracker.register_temp_file(file_path);
            tracker.update_activity();
        }

        Ok(())
    }

    /// Get resource usage for a specific session
    pub fn get_session_usage(&self, session_id: &Id) -> Result<ResourceUsage> {
        let trackers = self.session_trackers.lock().map_err(|_| {
            KaiakError::Internal("Failed to acquire trackers lock".to_string())
        })?;

        let tracker = trackers.get(session_id)
            .ok_or_else(|| KaiakError::SessionNotFound(session_id.clone()))?;

        Ok(tracker.get_usage())
    }

    /// Get system-wide resource statistics
    pub async fn get_system_stats(&self) -> Result<SystemResourceStats> {
        let stats = self.system_stats.lock().map_err(|_| {
            KaiakError::Internal("Failed to acquire stats lock".to_string())
        })?;

        Ok(stats.clone())
    }

    /// Update session status
    pub fn update_session_status(&self, session_id: &Id, status: SessionStatus) -> Result<()> {
        let mut trackers = self.session_trackers.lock().map_err(|_| {
            KaiakError::Internal("Failed to acquire trackers lock".to_string())
        })?;

        if let Some(tracker) = trackers.get_mut(session_id) {
            tracker.status = status;
            tracker.update_activity();
        }

        Ok(())
    }

    /// Check if system is under resource pressure
    pub async fn is_under_pressure(&self) -> Result<bool> {
        let stats = self.get_system_stats().await?;

        let pressure_indicators = [
            stats.memory_usage_percent > 85.0,
            stats.sessions_over_limit > 0,
            stats.active_sessions >= self.limits.max_concurrent_sessions,
        ];

        Ok(pressure_indicators.iter().any(|&x| x))
    }

    /// Force cleanup of sessions over resource limits
    pub async fn force_cleanup_over_limit(&self) -> Result<Vec<Id>> {
        let mut cleaned_sessions = Vec::new();

        let session_ids: Vec<Id> = {
            let trackers = self.session_trackers.lock().map_err(|_| {
                KaiakError::Internal("Failed to acquire trackers lock".to_string())
            })?;
            trackers.keys().cloned().collect()
        };

        for session_id in session_ids {
            let usage = self.get_session_usage(&session_id)?;
            if usage.is_over_limit {
                warn!("Force cleaning up session over limit: {}", session_id);
                self.deallocate_session(&session_id).await?;
                cleaned_sessions.push(session_id);
            }
        }

        Ok(cleaned_sessions)
    }

    /// Cleanup all sessions
    async fn cleanup_all_sessions(&self) -> Result<()> {
        let session_ids: Vec<Id> = {
            let trackers = self.session_trackers.lock().map_err(|_| {
                KaiakError::Internal("Failed to acquire trackers lock".to_string())
            })?;
            trackers.keys().cloned().collect()
        };

        for session_id in session_ids {
            if let Err(e) = self.deallocate_session(&session_id).await {
                error!("Failed to cleanup session {}: {}", session_id, e);
            }
        }

        Ok(())
    }

    /// Update system statistics
    async fn update_system_stats(&self) -> Result<()> {
        let (active_sessions, total_memory, total_handles, sessions_over_limit) = {
            let trackers = self.session_trackers.lock().map_err(|_| {
                KaiakError::Internal("Failed to acquire trackers lock".to_string())
            })?;

            let active = trackers.len();
            let memory: u64 = trackers.values()
                .map(|t| t.memory_tracker.get_usage())
                .sum();
            let handles: usize = trackers.values()
                .map(|t| t.file_tracker.get_usage())
                .sum();
            let over_limit = trackers.values()
                .filter(|t| t.get_usage().is_over_limit)
                .count();

            (active, memory, handles, over_limit)
        };

        let mut stats = self.system_stats.lock().map_err(|_| {
            KaiakError::Internal("Failed to acquire stats lock".to_string())
        })?;

        stats.active_sessions = active_sessions;
        stats.total_memory_usage_bytes = total_memory;
        stats.total_file_handles = total_handles;
        stats.sessions_over_limit = sessions_over_limit;

        // Calculate memory usage percentage (simplified)
        let total_limit = (self.limits.max_total_memory_mb * 1024 * 1024) as f64;
        stats.memory_usage_percent = if total_limit > 0.0 {
            (total_memory as f64 / total_limit) * 100.0
        } else {
            0.0
        };

        stats.available_memory_bytes = (total_limit as u64).saturating_sub(total_memory);

        Ok(())
    }

    /// Background cleanup task
    async fn cleanup_task(
        trackers: Arc<Mutex<HashMap<Id, SessionResourceTracker>>>,
        stats: Arc<Mutex<SystemResourceStats>>,
        limits: ResourceLimits,
        running: Arc<Mutex<bool>>,
    ) {
        let cleanup_interval = Duration::from_secs(limits.cleanup_interval_seconds);

        while *running.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) {
            debug!("Running resource cleanup task");

            // Identify sessions that need cleanup
            let sessions_to_cleanup: Vec<Id> = {
                let trackers_lock = trackers.lock().unwrap();
                let max_lifetime = Duration::from_secs(limits.max_process_lifetime_minutes * 60);

                trackers_lock.iter()
                    .filter(|(_, tracker)| {
                        tracker.created_at.elapsed() > max_lifetime ||
                        tracker.get_usage().is_over_limit
                    })
                    .map(|(id, _)| id.clone())
                    .collect()
            };

            // Cleanup identified sessions
            for session_id in sessions_to_cleanup {
                warn!("Cleaning up session due to resource limits: {}", session_id);

                let mut trackers_lock = trackers.lock().unwrap();
                if let Some(mut tracker) = trackers_lock.remove(&session_id) {
                    if let Err(e) = tracker.cleanup().await {
                        error!("Failed to cleanup session {}: {}", session_id, e);
                    }
                }
            }

            // Update cleanup timestamp
            if let Ok(mut stats_lock) = stats.lock() {
                stats_lock.last_cleanup = chrono::Utc::now();
            }

            sleep(cleanup_interval).await;
        }
    }

    /// Background monitoring task
    async fn monitoring_task(
        trackers: Arc<Mutex<HashMap<Id, SessionResourceTracker>>>,
        stats: Arc<Mutex<SystemResourceStats>>,
        limits: ResourceLimits,
        running: Arc<Mutex<bool>>,
    ) {
        let check_interval = Duration::from_secs(limits.resource_check_interval_seconds);

        while *running.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) {
            debug!("Running resource monitoring task");

            // Update system statistics
            let (active_sessions, total_memory, total_handles, sessions_over_limit) = {
                let trackers_lock = trackers.lock().unwrap();

                let active = trackers_lock.len();
                let memory: u64 = trackers_lock.values()
                    .map(|t| t.memory_tracker.get_usage())
                    .sum();
                let handles: usize = trackers_lock.values()
                    .map(|t| t.file_tracker.get_usage())
                    .sum();
                let over_limit = trackers_lock.values()
                    .filter(|t| t.get_usage().is_over_limit)
                    .count();

                (active, memory, handles, over_limit)
            };

            // Update stats
            if let Ok(mut stats_lock) = stats.lock() {
                stats_lock.active_sessions = active_sessions;
                stats_lock.total_memory_usage_bytes = total_memory;
                stats_lock.total_file_handles = total_handles;
                stats_lock.sessions_over_limit = sessions_over_limit;

                let total_limit = (limits.max_total_memory_mb * 1024 * 1024) as f64;
                stats_lock.memory_usage_percent = if total_limit > 0.0 {
                    (total_memory as f64 / total_limit) * 100.0
                } else {
                    0.0
                };
                stats_lock.available_memory_bytes = (total_limit as u64).saturating_sub(total_memory);
            }

            // Log warnings if resource usage is high
            if sessions_over_limit > 0 {
                warn!("Sessions over resource limits: {}", sessions_over_limit);
            }

            let total_limit_mb = limits.max_total_memory_mb;
            let usage_mb = total_memory / (1024 * 1024);
            if usage_mb > total_limit_mb * 8 / 10 { // 80% usage
                warn!("High memory usage: {}MB / {}MB", usage_mb, total_limit_mb);
            }

            sleep(check_interval).await;
        }
    }
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::session::Session;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_resource_manager_lifecycle() {
        let manager = ResourceManager::new();

        // Start manager
        let start_result = manager.start().await;
        assert!(start_result.is_ok());

        // Create test session
        let session = Session {
            id: Id::new(),
            goose_session_id: Uuid::new_v4().to_string(),
            status: SessionStatus::Created,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            configuration: Default::default(),
            active_request_id: None,
            message_count: 0,
            error_count: 0,
        };

        // Allocate resources
        let allocation = manager.allocate_session(&session).await;
        assert!(allocation.is_ok());

        // Test resource usage
        let usage = manager.get_session_usage(&session.id);
        assert!(usage.is_ok());

        // Test memory allocation
        let mem_alloc = manager.allocate_memory(&session.id, 1024);
        assert!(mem_alloc.is_ok());

        // Deallocate session
        let dealloc_result = manager.deallocate_session(&session.id).await;
        assert!(dealloc_result.is_ok());

        // Stop manager
        let stop_result = manager.stop().await;
        assert!(stop_result.is_ok());
    }

    #[test]
    fn test_memory_tracker() {
        let mut tracker = MemoryTracker::new(1024);

        // Test normal allocation
        let alloc_result = tracker.allocate(512);
        assert!(alloc_result.is_ok());
        assert_eq!(tracker.get_usage(), 512);

        // Test over-limit allocation
        let over_limit = tracker.allocate(1024);
        assert!(over_limit.is_err());

        // Test deallocation
        tracker.deallocate(256);
        assert_eq!(tracker.get_usage(), 256);
    }

    #[test]
    fn test_file_handle_tracker() {
        let mut tracker = FileHandleTracker::new(2);

        // Test normal registration
        let file1 = PathBuf::from("/tmp/test1");
        let result1 = tracker.register_file(file1.clone());
        assert!(result1.is_ok());

        let file2 = PathBuf::from("/tmp/test2");
        let result2 = tracker.register_file(file2.clone());
        assert!(result2.is_ok());

        // Test limit exceeded
        let file3 = PathBuf::from("/tmp/test3");
        let result3 = tracker.register_file(file3);
        assert!(result3.is_err());

        // Test unregistration
        tracker.unregister_file(&file1);
        assert_eq!(tracker.get_usage(), 1);
    }

    #[test]
    fn test_resource_limits_default() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.max_concurrent_sessions, 10);
        assert_eq!(limits.max_memory_per_session_mb, 512);
        assert!(limits.cleanup_interval_seconds > 0);
    }
}
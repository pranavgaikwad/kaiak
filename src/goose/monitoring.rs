// T051: Implement session monitoring utilities in src/goose/monitoring.rs
// Provides monitoring, health checks, and metrics collection for Goose sessions

use crate::{
    models::{
        session::{Session, SessionStatus},
        messages::{StreamMessage, MessageType, MessageContent, ToolOperation, ToolResult},
        Id,
    },
    KaiakError, KaiakResult as Result,
};
use serde::{Serialize, Deserialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::time::sleep;
use tracing::{info, debug};
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

// ==================== T005 & T006: Goose Event Bridge Implementation ====================

/// Placeholder for Goose agent events - will be replaced with real goose::AgentEvent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GooseAgentEvent {
    /// AI message response from the agent
    Message {
        content: String,
        partial: bool,
        confidence: Option<f32>,
    },
    /// Tool call executed by the agent
    ToolCall {
        id: String,
        tool_name: String,
        parameters: serde_json::Value,
        status: ToolExecutionStatus,
    },
    /// Tool call result
    ToolResult {
        call_id: String,
        success: bool,
        result: Option<serde_json::Value>,
        error: Option<String>,
        execution_time_ms: u64,
    },
    /// Agent thinking process
    Thinking {
        text: String,
    },
    /// Request for user interaction/approval
    InteractionRequest {
        interaction_id: String,
        interaction_type: String,
        prompt: String,
        timeout_seconds: Option<u32>,
    },
    /// File modification proposal
    FileModification {
        proposal_id: String,
        file_path: String,
        change_type: String,
        original_content: String,
        proposed_content: String,
        description: String,
        confidence: f32,
    },
    /// Error during agent processing
    Error {
        error_code: String,
        message: String,
        details: Option<String>,
        recoverable: bool,
    },
    /// System events (session state changes, etc.)
    System {
        event: String,
        status: String,
        metadata: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolExecutionStatus {
    Starting,
    InProgress,
    Completed,
    Failed,
}

/// Callback trait for streaming messages during processing
/// Used by GooseEventBridge to send converted events
pub trait MessageCallback: Send + Sync {
    fn on_message(&self, message: StreamMessage) -> Result<()>;
}

/// T005 - Goose Event Bridge
/// Bridges Goose agent events to Kaiak streaming system
pub struct GooseEventBridge {
    session_id: String,
    request_id: Option<String>,
    message_callback: Option<Arc<dyn MessageCallback>>,
    sequence_number: Arc<Mutex<u32>>,
    active: Arc<Mutex<bool>>,
}

impl GooseEventBridge {
    /// Create new event bridge for a session
    pub fn new(session_id: String, request_id: Option<String>) -> Self {
        Self {
            session_id,
            request_id,
            message_callback: None,
            sequence_number: Arc::new(Mutex::new(0)),
            active: Arc::new(Mutex::new(false)),
        }
    }

    /// Set the message callback for streaming
    pub fn set_message_callback(&mut self, callback: Arc<dyn MessageCallback>) {
        self.message_callback = Some(callback);
    }

    /// Start listening for Goose agent events
    pub async fn start_event_subscription(&self) -> Result<()> {
        {
            let mut active = self.active.lock().map_err(|_| {
                KaiakError::Internal("Failed to acquire active lock".to_string())
            })?;
            if *active {
                return Ok(()); // Already active
            }
            *active = true;
        }

        info!("Started Goose event subscription for session: {}", self.session_id);
        Ok(())
    }

    /// Stop listening for Goose agent events
    pub async fn stop_event_subscription(&self) -> Result<()> {
        {
            let mut active = self.active.lock().map_err(|_| {
                KaiakError::Internal("Failed to acquire active lock".to_string())
            })?;
            *active = false;
        }

        info!("Stopped Goose event subscription for session: {}", self.session_id);
        Ok(())
    }

    /// Subscribe to Goose agent events (placeholder for actual implementation)
    /// This would subscribe to real goose::Agent events in actual implementation
    pub async fn subscribe_to_goose_events(&self) -> Result<()> {
        // Validate that callback is set
        if self.message_callback.is_none() {
            return Err(KaiakError::Internal("Message callback not set".to_string()));
        }

        // In real implementation, this would:
        // let mut event_stream = agent.subscribe_events();
        // while let Some(event) = event_stream.next().await {
        //     let stream_message = self.convert_goose_event(event).await?;
        //     if let Some(callback) = &self.message_callback {
        //         callback.on_message(stream_message)?;
        //     }
        // }

        info!("Subscribed to Goose events for session: {}", self.session_id);
        Ok(())
    }

    /// T006 - Convert Goose agent events to Kaiak streaming messages
    /// T013 - Enhanced with feature gap detection and logging
    pub async fn convert_goose_event(&self, event: GooseAgentEvent) -> Result<StreamMessage> {
        let sequence_num = self.get_next_sequence();

        let content = match event {
            GooseAgentEvent::Message { content, partial, confidence } => {
                // T013 - Check for rich content capabilities that might be missing
                if content.contains("```diff") || content.contains("@@ ") {
                    self.log_message_format_gap(
                        "ai_response",
                        "Rich diff content in text response - could be enhanced with structured diff display"
                    );
                }
                if content.contains("![") || content.contains("<img") {
                    self.log_message_format_gap(
                        "ai_response",
                        "Image content in text response - requires rich content display support"
                    );
                }

                MessageContent::AiResponse {
                    text: content,
                    partial,
                    confidence,
                }
            }
            GooseAgentEvent::ToolCall { id, tool_name, parameters, status } => {
                let operation = match status {
                    ToolExecutionStatus::Starting => ToolOperation::Start,
                    ToolExecutionStatus::InProgress => ToolOperation::Progress,
                    ToolExecutionStatus::Completed => ToolOperation::Complete,
                    ToolExecutionStatus::Failed => ToolOperation::Error,
                };

                // T013 - Log advanced tool call features that aren't fully supported
                if parameters.get("dependencies").is_some() {
                    self.log_event_conversion_gap(
                        "tool_call",
                        "Tool call dependencies not tracked in current implementation"
                    );
                }
                if parameters.get("approval_requirements").is_some() {
                    self.log_event_conversion_gap(
                        "tool_call",
                        "Custom approval requirements not fully processed"
                    );
                }
                if parameters.get("estimated_duration_ms").is_some() {
                    self.log_message_format_gap(
                        "tool_call",
                        "Tool execution time estimates not displayed to user"
                    );
                }

                MessageContent::ToolCall {
                    tool_name,
                    operation,
                    parameters,
                    result: None, // Will be filled in by ToolResult event
                }
            }
            GooseAgentEvent::ToolResult { call_id, success, result, error, execution_time_ms } => {
                let tool_result = ToolResult {
                    success,
                    data: result.clone(),
                    error,
                    execution_time_ms,
                    output_size_bytes: result.as_ref().map(|r| r.to_string().len() as u64),
                };

                // T013 - Check for rich tool result content that can't be displayed properly
                if let Some(result_data) = &result {
                    if result_data.get("file_tree").is_some() {
                        self.log_ide_enhancement_need(
                            "file_tree_display",
                            "Tool result contains file tree structure - needs interactive file browser"
                        );
                    }
                    if result_data.get("code_diff").is_some() {
                        self.log_ide_enhancement_need(
                            "code_diff_display",
                            "Tool result contains code diff - needs syntax-highlighted diff view"
                        );
                    }
                    if result_data.get("interactive_component").is_some() {
                        self.log_unsupported_feature(
                            "interactive_tool_results",
                            "Tool result requires interactive component that cannot be rendered"
                        );
                    }
                }

                MessageContent::ToolCall {
                    tool_name: format!("tool-{}", call_id),
                    operation: if success { ToolOperation::Complete } else { ToolOperation::Error },
                    parameters: serde_json::Value::Null,
                    result: Some(tool_result),
                }
            }
            GooseAgentEvent::Thinking { text } => {
                MessageContent::Thinking { text }
            }
            GooseAgentEvent::InteractionRequest { interaction_id, interaction_type, prompt, timeout_seconds } => {
                // T013 - Log advanced interaction types that may need enhanced UI
                if interaction_type.contains("inline_edit") {
                    self.log_ide_enhancement_need(
                        "inline_editing",
                        "Inline edit interaction requires IDE integration with live preview"
                    );
                }
                if interaction_type.contains("multi_choice") {
                    self.log_message_format_gap(
                        "interaction",
                        "Multi-choice interactions limited to simple approval dialogs"
                    );
                }
                if interaction_type.contains("code_completion") {
                    self.log_unsupported_feature(
                        "code_completion_interaction",
                        "Code completion interactions not supported in current UI"
                    );
                }

                MessageContent::UserInteraction {
                    interaction_id,
                    interaction_type,
                    prompt,
                    proposal_id: None,
                    timeout: timeout_seconds,
                }
            }
            GooseAgentEvent::FileModification {
                proposal_id,
                file_path,
                change_type,
                original_content,
                proposed_content,
                description,
                confidence,
            } => {
                // T013 - Log advanced file modification features
                if change_type.contains("semantic_edit") {
                    self.log_unsupported_feature(
                        "semantic_file_editing",
                        "Semantic-aware file editing not supported - using basic text replacement"
                    );
                }
                if original_content.len() > 10000 || proposed_content.len() > 10000 {
                    self.log_message_format_gap(
                        "file_modification",
                        "Large file modifications may need streaming diff display for better UX"
                    );
                }

                MessageContent::FileModification {
                    proposal_id,
                    file_path,
                    change_type,
                    description,
                    original_content,
                    proposed_content,
                    confidence,
                }
            }
            GooseAgentEvent::Error { error_code, message, details, recoverable } => {
                MessageContent::Error {
                    error_code,
                    message,
                    details,
                    recoverable,
                }
            }
            GooseAgentEvent::System { event, status, metadata } => {
                // T013 - Log advanced system event features
                if event.contains("session_branch") {
                    self.log_unsupported_feature(
                        "session_branching",
                        "Session branching events not supported in current implementation"
                    );
                }
                if event.contains("model_switch") {
                    self.log_unsupported_feature(
                        "runtime_model_switching",
                        "Runtime model switching not implemented"
                    );
                }
                if metadata.get("performance_metrics").is_some() {
                    self.log_message_format_gap(
                        "system_event",
                        "Performance metrics in system events not displayed in UI"
                    );
                }

                MessageContent::System {
                    event,
                    request_id: self.request_id.clone(),
                    status,
                    summary: Some(metadata),
                }
            }
        };

        let message_type = match &content {
            MessageContent::Progress { .. } => MessageType::Progress,
            MessageContent::AiResponse { .. } => MessageType::AiResponse,
            MessageContent::ToolCall { .. } => MessageType::ToolCall,
            MessageContent::Thinking { .. } => MessageType::Thinking,
            MessageContent::UserInteraction { .. } => MessageType::UserInteraction,
            MessageContent::FileModification { .. } => MessageType::FileModification,
            MessageContent::Error { .. } => MessageType::Error,
            MessageContent::System { .. } => MessageType::System,
        };

        Ok(StreamMessage::new(
            self.session_id.clone(),
            self.request_id.clone(),
            message_type,
            content,
        ))
    }

    /// Process a Goose event and send to callback
    pub async fn handle_goose_event(&self, event: GooseAgentEvent) -> Result<()> {
        let stream_message = self.convert_goose_event(event).await?;

        if let Some(callback) = &self.message_callback {
            callback.on_message(stream_message)?;
        } else {
            debug!("No callback set for event bridge, dropping message");
        }

        Ok(())
    }

    /// Get next sequence number for message ordering
    fn get_next_sequence(&self) -> u32 {
        let mut seq = self.sequence_number.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        *seq += 1;
        *seq
    }

    /// T013 - Log unsupported Goose features for comprehensive gap documentation (FR-010/FR-011)
    pub fn log_unsupported_feature(&self, feature: &str, details: &str) {
        use tracing::warn;

        warn!(
            target: "feature_gap",
            session_id = %self.session_id,
            feature = %feature,
            details = %details,
            "Unsupported Goose feature detected"
        );

        // Log structured data for automated collection
        let gap_data = serde_json::json!({
            "session_id": self.session_id,
            "feature": feature,
            "details": details,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "category": Self::categorize_feature_gap(feature),
            "impact_level": Self::assess_feature_impact(feature),
            "recommendation": Self::get_feature_recommendation(feature)
        });

        // In full implementation, this would be sent to centralized gap tracking service
        debug!(target: "feature_gap_data", gap_data = %gap_data, "Feature gap data collected");
    }

    /// T013 - Categorize feature gaps for systematic analysis
    fn categorize_feature_gap(feature: &str) -> &'static str {
        match feature {
            f if f.contains("plugin") || f.contains("extension") => "plugin_system",
            f if f.contains("model") || f.contains("provider") => "model_management",
            f if f.contains("session") || f.contains("persistence") => "session_management",
            f if f.contains("tool") || f.contains("call") => "tool_execution",
            f if f.contains("diff") || f.contains("content") || f.contains("rich") => "content_display",
            f if f.contains("interactive") || f.contains("inline") || f.contains("editor") => "interactive_features",
            f if f.contains("debug") || f.contains("breakpoint") => "debugging_integration",
            f if f.contains("auth") || f.contains("user") || f.contains("permission") => "enterprise_features",
            _ => "uncategorized"
        }
    }

    /// T013 - Assess the impact level of missing features
    fn assess_feature_impact(feature: &str) -> &'static str {
        match feature {
            // High impact - core functionality limitations
            f if f.contains("session_branching") || f.contains("model_switching") => "high",
            f if f.contains("rich_content") || f.contains("code_diff") => "high",
            f if f.contains("interactive_edit") || f.contains("inline_preview") => "high",

            // Medium impact - enhanced functionality
            f if f.contains("plugin") || f.contains("custom_tool") => "medium",
            f if f.contains("persistence") || f.contains("long_term") => "medium",
            f if f.contains("collaboration") || f.contains("multi_user") => "medium",

            // Low impact - nice-to-have features
            f if f.contains("advanced") || f.contains("optimization") => "low",
            f if f.contains("analytics") || f.contains("metrics") => "low",

            _ => "unknown"
        }
    }

    /// T013 - Provide implementation recommendations for missing features
    fn get_feature_recommendation(feature: &str) -> &'static str {
        match feature {
            f if f.contains("session_branching") => "Implement SessionOperation enum with Branch/Merge support",
            f if f.contains("model_switching") => "Extend SessionConfiguration with runtime model switching",
            f if f.contains("rich_content") => "Develop rich content event types in GooseEventBridge",
            f if f.contains("interactive_edit") => "Create IDE-specific event extensions with bi-directional sync",
            f if f.contains("plugin") => "Design plugin loading and custom tool registration system",
            f if f.contains("persistence") => "Implement database-backed session storage",
            f if f.contains("collaboration") => "Add multi-user session support with permissions",
            f if f.contains("debug") => "Integrate with language server protocol for debugging",
            _ => "Analyze requirements and design appropriate implementation strategy"
        }
    }

    /// T013 - Log specific event conversion gaps encountered during processing
    pub fn log_event_conversion_gap(&self, goose_event_type: &str, missing_capability: &str) {
        self.log_unsupported_feature(
            &format!("event_conversion_{}", goose_event_type),
            &format!("Cannot fully convert Goose {} event: {}", goose_event_type, missing_capability)
        );
    }

    /// T013 - Log message format incompatibilities
    pub fn log_message_format_gap(&self, message_type: &str, limitation: &str) {
        self.log_unsupported_feature(
            &format!("message_format_{}", message_type),
            &format!("Message format limitation for {}: {}", message_type, limitation)
        );
    }

    /// T013 - Log IDE enhancement requirements based on usage patterns
    pub fn log_ide_enhancement_need(&self, enhancement_type: &str, context: &str) {
        self.log_unsupported_feature(
            &format!("ide_enhancement_{}", enhancement_type),
            &format!("IDE enhancement needed for {}: {}", enhancement_type, context)
        );
    }

    /// T015 - Record performance metrics for success criteria validation
    pub fn record_performance_metric(&self, metric_type: &str, value: f64, threshold: f64, passed: bool) {
        use tracing::{info, warn};

        if passed {
            info!(
                target: "performance_metrics",
                metric_type = %metric_type,
                value = %value,
                threshold = %threshold,
                result = "passed",
                "Performance metric within threshold"
            );
        } else {
            warn!(
                target: "performance_metrics",
                metric_type = %metric_type,
                value = %value,
                threshold = %threshold,
                result = "failed",
                "Performance metric exceeded threshold"
            );
        }

        // Log structured data for performance analysis
        let metric_data = serde_json::json!({
            "session_id": self.session_id,
            "metric_type": metric_type,
            "value": value,
            "threshold": threshold,
            "passed": passed,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "success_criteria": Self::map_metric_to_success_criteria(metric_type)
        });

        debug!(target: "performance_data", metric_data = %metric_data, "Performance metric recorded");
    }

    /// T015 - Map performance metrics to success criteria
    fn map_metric_to_success_criteria(metric_type: &str) -> &'static str {
        match metric_type {
            "processing_time_ms" => "SC-001: Processing time <30s",
            "streaming_latency_ms" => "SC-002: Streaming latency <500ms",
            "test_success_rate" => "SC-003: 95% test success rate",
            "tool_call_capture_rate" => "SC-004: 100% tool call capture",
            "error_handling_coverage" => "SC-005: 100% error handling",
            "goose_compatibility" => "SC-006: Goose compatibility demonstrated",
            _ => "Unknown success criteria"
        }
    }

    /// T015 - Record streaming latency measurement
    pub fn record_streaming_latency(&self, latency_ms: u64) {
        const LATENCY_THRESHOLD: u64 = 500; // SC-002: <500ms
        let passed = latency_ms < LATENCY_THRESHOLD;

        self.record_performance_metric(
            "streaming_latency_ms",
            latency_ms as f64,
            LATENCY_THRESHOLD as f64,
            passed
        );
    }

    /// T015 - Record processing time measurement
    pub fn record_processing_time(&self, processing_time_ms: u64) {
        const PROCESSING_THRESHOLD: u64 = 30_000; // SC-001: <30s
        let passed = processing_time_ms < PROCESSING_THRESHOLD;

        self.record_performance_metric(
            "processing_time_ms",
            processing_time_ms as f64,
            PROCESSING_THRESHOLD as f64,
            passed
        );
    }

    /// T015 - Record test success rate measurement
    pub fn record_test_success_rate(&self, success_rate: f64) {
        const SUCCESS_RATE_THRESHOLD: f64 = 0.95; // SC-003: 95%
        let passed = success_rate >= SUCCESS_RATE_THRESHOLD;

        self.record_performance_metric(
            "test_success_rate",
            success_rate,
            SUCCESS_RATE_THRESHOLD,
            passed
        );
    }

    /// T015 - Record tool call capture rate
    pub fn record_tool_call_capture_rate(&self, capture_rate: f64) {
        const CAPTURE_RATE_THRESHOLD: f64 = 1.0; // SC-004: 100%
        let passed = capture_rate >= CAPTURE_RATE_THRESHOLD;

        self.record_performance_metric(
            "tool_call_capture_rate",
            capture_rate,
            CAPTURE_RATE_THRESHOLD,
            passed
        );
    }
}

/// Implementation of MessageCallback for GooseEventBridge
pub struct StreamingMessageHandler {
    tx: tokio::sync::mpsc::UnboundedSender<StreamMessage>,
}

impl StreamingMessageHandler {
    pub fn new(tx: tokio::sync::mpsc::UnboundedSender<StreamMessage>) -> Self {
        Self { tx }
    }
}

impl MessageCallback for StreamingMessageHandler {
    fn on_message(&self, message: StreamMessage) -> Result<()> {
        self.tx.send(message).map_err(|_| {
            KaiakError::Internal("Failed to send message through channel".to_string())
        })?;
        Ok(())
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
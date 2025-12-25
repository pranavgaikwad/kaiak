# Data Model: Agent API Refactor for Goose Integration

**Date**: 2025-12-24
**Feature**: 003-agent-api-refactor

## Overview

This document defines the data models for the refactored agent API that integrates with Goose AI framework while maintaining compatibility with existing JSON-RPC protocol and transport layers.

## Core Entities

### 1. Agent Configuration

Structured JSON configuration object with nested sections for comprehensive agent setup.

```rust
use goose::agents::SessionConfig as GooseSessionConfig;
use goose::providers::ModelConfig as GooseModelConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfiguration {
    pub workspace: WorkspaceConfig,
    pub model: GooseModelConfig,  // Re-use Goose's model configuration
    pub tools: ToolConfig,
    pub session: GooseSessionConfig,  // Re-use Goose's session configuration
    pub permissions: PermissionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub working_dir: PathBuf,
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    pub enabled_extensions: Vec<String>,
    pub custom_tools: Vec<CustomToolConfig>,
    pub planning_mode: bool,
    pub max_tool_calls: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionConfig {
    pub tool_permissions: HashMap<String, ToolPermission>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolPermission {
    Allow,       // Always allow this tool
    Deny,        // Always deny this tool
    Approve,     // Require user approval for this tool
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomToolConfig {
    pub name: String,
    pub extension_type: ExtensionType,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionType {
    Stdio,
    Sse,
    Platform,
    Frontend,
}
```

### 2. Agent Session

Goose-managed session instance with metadata for tracking and lifecycle management.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub id: String,                    // Client-generated UUID
    pub goose_session_id: String,     // Goose internal session ID
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub configuration: AgentConfiguration,
    pub metrics: SessionMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Creating,     // Session being created
    Ready,        // Session ready for requests
    Processing,   // Agent actively processing
    Waiting,      // Waiting for user interaction
    Completed,    // Processing completed successfully
    Error,        // Session in error state
    Terminated,   // Session terminated by user/system
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetrics {
    pub message_count: u32,
    pub tool_calls_count: u32,
    pub tokens_used: Option<u32>,
    pub processing_time: Option<u64>, // milliseconds
    pub interaction_count: u32,
}
```

### 3. Migration Incident

Simplified input data representing code issues requiring agent processing and resolution.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationIncident {
    pub id: String,
    pub rule_id: String,
    pub message: String,
    pub description: String,
    pub effort: String,
    pub severity: IncidentSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IncidentSeverity {
    Info,
    Warning,
    Error,
    Critical,
}
```

### 4. Agent Event

Real-time notifications from Goose agents mapped to Kaiak's streaming system.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEventNotification {
    pub session_id: String,
    pub request_id: Option<String>,
    pub message_id: String,
    pub timestamp: DateTime<Utc>,
    pub event_type: AgentEventType,
    pub content: AgentEventContent,
    pub metadata: EventMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEventType {
    Progress,
    AiResponse,
    ToolCall,
    Thinking,
    UserInteraction,
    FileModification,
    Error,
    System,
    ModelChange,
    HistoryCompacted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEventContent {
    Progress {
        percentage: u8,
        phase: String,
        description: String,
        current_step: Option<String>,
        total_steps: Option<u32>,
    },
    AiResponse {
        text: String,
        partial: bool,
        confidence: Option<f32>,
        tokens: Option<u32>,
    },
    ToolCall {
        tool_name: String,
        operation: String,
        parameters: serde_json::Value,
        status: ToolCallStatus,
        result: Option<ToolCallResult>,
    },
    Thinking {
        internal_monologue: String,
        reasoning_type: String,
        confidence: Option<f32>,
    },
    UserInteraction {
        interaction_id: String,
        interaction_type: UserInteractionType,
        prompt: String,
        options: Option<Vec<String>>,
        default_response: Option<String>,
        timeout: Option<u32>,
    },
    FileModification {
        proposal_id: String,
        file_path: PathBuf,
        operation: FileOperation,
        diff: Option<String>,
        requires_approval: bool,
    },
    Error {
        error_code: String,
        message: String,
        details: Option<String>,
        recoverable: bool,
        suggested_action: Option<String>,
    },
    System {
        message: String,
        level: SystemEventLevel,
        component: String,
    },
    ModelChange {
        old_model: String,
        new_model: String,
        reason: String,
    },
    HistoryCompacted {
        original_length: u32,
        compacted_length: u32,
        tokens_saved: Option<u32>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallStatus {
    Pending,
    Approved,
    Denied,
    Executing,
    Completed,
    Failed,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub execution_time: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserInteractionType {
    Confirmation,
    Choice,
    TextInput,
    FileApproval,
    ToolPermission,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileOperation {
    Create,
    Modify,
    Delete,
    Rename,
    Copy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SystemEventLevel {
    Debug,
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    pub sequence_number: u64,
    pub correlation_id: Option<String>,
    pub trace_id: Option<String>,
    pub processing_duration: Option<u64>,
}
```

### 5. User Interaction

Requests for client input during agent processing with comprehensive response handling.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInteractionRequest {
    pub session_id: String,
    pub interaction_id: String,
    pub request_id: Option<String>,
    pub interaction_type: UserInteractionType,
    pub prompt: String,
    pub context: InteractionContext,
    pub response_options: ResponseOptions,
    pub timeout: Option<u32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionContext {
    pub file_path: Option<PathBuf>,
    pub code_snippet: Option<String>,
    pub incident_id: Option<String>,
    pub tool_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseOptions {
    Confirmation {
        default_response: bool,
    },
    Choice {
        options: Vec<String>,
        allow_custom: bool,
        default_index: Option<usize>,
    },
    TextInput {
        placeholder: Option<String>,
        validation_pattern: Option<String>,
        max_length: Option<u32>,
    },
    FileApproval {
        proposal_id: String,
        diff_preview: String,
        allow_edit: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInteractionResponse {
    pub session_id: String,
    pub interaction_id: String,
    pub response_type: UserResponseType,
    pub response_data: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserResponseType {
    Approved,
    Denied,
    Custom,
    Timeout,
    Cancelled,
}
```

## Request/Response Models

### 1. Configure Endpoint

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigureRequest {
    pub configuration: AgentConfiguration,
    pub reset_existing: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigureResponse {
    pub status: String,
    pub message: Option<String>,
    pub configuration_applied: AgentConfiguration,
    pub warnings: Vec<String>,
    pub timestamp: DateTime<Utc>,
}
```

### 2. Generate Fix Endpoint

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateFixRequest {
    pub session_id: String,
    pub incidents: Vec<MigrationIncident>,
    pub migration_context: Option<MigrationContext>,
    pub options: GenerationOptions,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MigrationContext {
    pub source_technology: String,
    pub target_technology: String,
    pub migration_hints: Vec<String>,
    pub constraints: Vec<String>,
    pub preferences: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerationOptions {
    pub auto_apply_safe_fixes: bool,
    pub max_processing_time: Option<u32>,
    pub parallel_processing: bool,
    pub include_explanations: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateFixResponse {
    pub request_id: String,
    pub session_id: String,
    pub status: RequestStatus,
    pub incident_count: usize,
    pub completed_at: DateTime<Utc>,  // When agent finished processing
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestStatus {
    Completed,  // Agent completed successfully
    Failed,     // Agent failed to process
    Cancelled,  // Processing was cancelled
}
```

### 3. Delete Session Endpoint

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteSessionRequest {
    pub session_id: String,
    pub force: Option<bool>,
    pub cleanup_files: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteSessionResponse {
    pub session_id: String,
    pub status: String,
    pub cleanup_summary: CleanupSummary,
    pub deleted_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CleanupSummary {
    pub session_removed: bool,
    pub messages_cleaned: u32,
    pub temp_files_removed: u32,
    pub errors: Vec<String>,
}
```

## State Transitions

### Session State Machine

```
Creating → Ready → Processing → (Waiting | Completed | Error)
                 ↘             ↗
                   Terminated
```

**State Descriptions**:
- **Creating**: Session being initialized with Goose SessionManager
- **Ready**: Session created, agent configured, ready for requests
- **Processing**: Agent actively processing incidents with Goose
- **Waiting**: Agent paused, awaiting user interaction response
- **Completed**: Processing finished successfully
- **Error**: Processing failed or session in error state
- **Terminated**: Session ended by user request or system cleanup

### Request State Machine

```
Processing → (Completed | Failed | Cancelled)
```

## Validation Rules

### 1. Agent Configuration Validation

```rust
impl AgentConfiguration {
    pub fn validate(&self) -> Result<(), ValidationError> {
        // Workspace validation
        if !self.workspace.working_dir.exists() {
            return Err(ValidationError::InvalidWorkspace("Directory does not exist".into()));
        }

        // Tool validation
        for tool in &self.tools.custom_tools {
            if tool.name.is_empty() {
                return Err(ValidationError::InvalidTool("Tool name required".into()));
            }
        }

        Ok(())
    }
}
```

### 2. Session ID Validation

```rust
impl AgentSession {
    pub fn validate_session_id(session_id: &str) -> Result<(), ValidationError> {
        use uuid::Uuid;

        Uuid::parse_str(session_id)
            .map_err(|_| ValidationError::InvalidSessionId("Must be valid UUID".into()))?;

        Ok(())
    }
}
```

### 3. Migration Incident Validation

```rust
impl MigrationIncident {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.id.is_empty() || self.rule_id.is_empty() {
            return Err(ValidationError::InvalidIncident("ID and rule_id required".into()));
        }

        if self.message.is_empty() || self.description.is_empty() {
            return Err(ValidationError::InvalidIncident("Message and description required".into()));
        }

        Ok(())
    }
}
```

## Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Invalid workspace: {0}")]
    InvalidWorkspace(String),

    #[error("Invalid tool configuration: {0}")]
    InvalidTool(String),

    #[error("Invalid session ID: {0}")]
    InvalidSessionId(String),

    #[error("Invalid incident: {0}")]
    InvalidIncident(String),

    #[error("Invalid interaction: {0}")]
    InvalidInteraction(String),
}

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("Configuration error: {0}")]
    Configuration(#[from] ValidationError),

    #[error("Session error: {0}")]
    Session(String),

    #[error("Goose integration error: {0}")]
    GooseIntegration(String),

    #[error("Tool execution error: {0}")]
    ToolExecution(String),

    #[error("User interaction timeout: {0}")]
    InteractionTimeout(String),

    #[error("File operation error: {0}")]
    FileOperation(String),
}
```

## Relationships

### Entity Relationship Diagram

```
AgentConfiguration ──1:N── AgentSession
                          │
AgentSession ──1:N── UserInteractionRequest
           │
           └──1:N── AgentEventNotification
                   │
                   └── AgentEventContent
                       │
                       ├── ToolCallResult
                       ├── FileModification
                       └── UserInteraction

MigrationIncident ──N:1── GenerateFixRequest
                         │
                         └──1:1── AgentSession
```

This data model provides a comprehensive foundation for the agent API refactor while maintaining compatibility with existing JSON-RPC infrastructure and enabling seamless Goose integration.
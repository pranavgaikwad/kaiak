# Research Findings: Agent Implementation

**Feature**: Agent Implementation
**Date**: 2025-12-23
**Purpose**: Resolve technical unknowns and establish implementation patterns for Goose Agent integration

---

## Overview

This research consolidates findings from investigating Goose Agent framework integration patterns, model provider mocking capabilities, and message compatibility between Goose and IDE extensions.

---

## 1. Model Provider Mocking for CI/CD Testing

### Decision: Record-Replay Testing Pattern
**Rationale**: Goose provides a sophisticated `TestProvider` that records real API interactions and replays them deterministically. This enables comprehensive integration testing without API keys in CI/PR environments.

**Implementation Pattern**:
```rust
let test_provider = if replay_mode {
    TestProvider::new_replaying("tests/recordings/test_case.json")?
} else {
    let real_provider = create_real_provider().await?;
    TestProvider::new_recording(real_provider, "tests/recordings/test_case.json")
};
```

**Key Features**:
- **Record-and-replay**: Wraps real providers during recording, replays without API calls
- **Hash-based matching**: Uses SHA256 of message content for deterministic request matching
- **CI safety guards**: Prevents recording in CI environments, requires pre-committed recordings
- **Multi-provider support**: Works with any provider implementing the `Provider` trait

**Alternatives Considered**:
- **Static mock responses**: Rejected because they don't capture real LLM behavior patterns
- **API proxy mocking**: Rejected due to complexity and maintenance overhead
- **Live API testing in CI**: Rejected due to cost, rate limits, and credential management

---

## 2. Goose Agent Integration API Patterns

### Decision: AgentManager with Per-Session Agent Pattern
**Rationale**: Goose 2025 architecture uses per-session agents managed by `AgentManager`, eliminating shared state issues and enabling session isolation. This aligns perfectly with Kaiak's existing `GooseManager` and `GooseSessionWrapper` design.

**Implementation Pattern**:
```rust
// Wire actual Goose Agent in GooseSessionWrapper::initialize()
pub async fn initialize(&mut self) -> Result<()> {
    let agent = Agent::new(
        &self.workspace_path,
        self.configuration.provider.clone(),
        self.configuration.model.clone(),
    ).await?;

    self.goose_agent = Some(agent);
    self.status = SessionStatus::Ready;
    Ok(())
}

// Subscribe to Goose events and bridge to Kaiak's StreamMessage format
let mut event_stream = agent.subscribe_events();
while let Some(event) = event_stream.next().await {
    match event {
        AgentEvent::Message(msg) => self.handle_message(msg).await?,
        AgentEvent::ToolCall(tool) => self.handle_tool_call(tool).await?,
        // ... other events
    }
}
```

**Key Features**:
- **Session Isolation**: Each session gets dedicated Agent, ExtensionManager, ToolMonitor
- **Event Streaming**: AgentEvent enum provides Message, McpNotification, ModelChange, HistoryReplaced
- **Tool Interception**: Integrate with existing `create_safe_tool_call()` approval workflow
- **Provider Configuration**: Map SessionConfiguration to Goose's 20+ built-in providers
- **Error Recovery**: Result types throughout with graceful degradation

**Alternatives Considered**:
- **Shared Agent**: Rejected due to session interference and lock contention
- **HTTP API integration**: Rejected as direct library integration provides better performance and control
- **CLI wrapper**: Rejected due to complexity and limited streaming capabilities

---

## 3. Goose-IDE Message Compatibility Analysis

### Decision: Document Feature Gaps for Incremental Enhancement
**Rationale**: Goose provides sophisticated capabilities far beyond typical IDE extensions. Rather than limiting Goose features, document gaps to guide future IDE enhancement roadmap.

**Key Compatibility Findings**:

**Goose Advanced Features Not in Typical IDEs**:
1. **Multi-Step Orchestration**: SubAgent task execution, Recipe composition, parallel processing
2. **Rich Event Streaming**: Tool notifications, task events, token usage, conversation replacements
3. **Session Persistence**: SQLite-backed full history, extension state, session types
4. **Context Management**: Auto-compaction, intelligent summarization, threshold triggering
5. **MCP 2025 Features**: Sampling, elicitation, progress notifications, resource discovery
6. **Structured Workflows**: JSON schema validation, retry logic, success checks
7. **Permission System**: Asynchronous approval, smart modes, security inspection

**Message Format Differences**:
```rust
// Goose MessageContent (rich, extensible)
pub enum MessageContent {
    Text(TextContent),
    Image(ImageContent),
    ToolRequest(ToolRequest),
    ToolResponse(ToolResponse),
    ActionRequired(ActionRequired),     // Multi-type approval system
    Thinking(ThinkingContent),          // Cryptographically signed
    SystemNotification(SystemNotificationContent),
    // ... 8 total types
}

// Typical IDE (simple)
enum MessageType { Text, Error, Progress, ToolCall }
```

**Implementation Pattern**:
```rust
// Document differences during integration
pub struct GooseToIdeMapper {
    unsupported_features: Vec<String>,
    message_conversions: HashMap<GooseEvent, IdeMessage>,
    feature_gaps: Vec<FeatureGap>,
}

impl GooseToIdeMapper {
    pub fn convert_event(&mut self, event: AgentEvent) -> Option<StreamMessage> {
        match event {
            AgentEvent::Message(msg) => self.convert_message(msg),
            AgentEvent::McpNotification(_) => {
                self.log_unsupported("MCP notifications not supported by IDE");
                None // Drop advanced notifications
            }
            // ... handle other events
        }
    }
}
```

**Alternatives Considered**:
- **Restrict Goose features**: Rejected as it limits future enhancement potential
- **Create IDE-specific Goose build**: Rejected due to maintenance complexity
- **Ignore advanced features**: Rejected as FR-010/FR-011 require documentation

---

## Implementation Guidelines

Based on current findings:

### Testing Strategy
1. **Create recording files**: Use real providers during development to generate test recordings
2. **Commit recordings**: All JSON recording files must be version controlled for CI
3. **CI safety**: Implement guards to prevent recording mode in automated environments
4. **Test organization**: Structure recordings by provider and test scenario

### CI/PR Integration
```yaml
# Recommended CI pattern
- name: Run Integration Tests
  run: |
    # Use only replay mode in CI
    GOOSE_REPLAY_MODE=true cargo test integration
```

### Directory Structure
```text
tests/
├── integration/
│   ├── recordings/          # Committed test recordings
│   │   ├── anthropic/
│   │   ├── openai/
│   │   └── test_fixtures/
│   └── goose_integration.rs # Main integration test
└── fixtures/                # Sample incidents, test workspace
```

---

## Next Steps

1. ✅ **Research Complete**: All technical unknowns resolved with implementation patterns identified
2. **Phase 1 - Design & Contracts**: Create data models and API contracts based on research findings
3. **Implementation Focus**: Wire actual Goose Agent into existing skeleton with proper event streaming
4. **Testing Infrastructure**: Implement TestProvider recording/replay for CI/PR testing

---

**Research Status**: ✅ **COMPLETE** - All technical decisions resolved and implementation patterns established.
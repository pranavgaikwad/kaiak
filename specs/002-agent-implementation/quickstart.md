# Quickstart Guide: Agent Implementation

**Feature**: Agent Implementation
**Date**: 2025-12-23
**Purpose**: Step-by-step guide for implementing Goose Agent integration in Kaiak

---

## Overview

This guide covers implementing the Goose Agent integration feature in Kaiak, enabling standalone agent processing capabilities with comprehensive end-to-end testing.

**Key Deliverables**:
- Complete Goose Agent integration in existing skeleton
- Comprehensive integration test with recording/replay capability
- Real-time streaming of agent events
- Tool call interception with approval workflow
- Documentation of Goose-IDE feature gaps

---

## Prerequisites

### Environment Setup

1. **Rust Toolchain**: Rust 1.75+ (stable)
   ```bash
   rustup update stable
   rustc --version  # Should be 1.75+
   ```

2. **Dependencies**: Already in `Cargo.toml`
   ```toml
   goose = { git = "https://github.com/block/goose.git" }
   tower-lsp = "0.20"
   tokio = { version = "1.0", features = ["full"] }
   # ... other existing deps
   ```

3. **Goose Project Access**: Clone for testing infrastructure reference
   ```bash
   # Already available at /home/pranav/Projects/goose/
   ```

### Development Environment

- **Testing**: Model provider access for recording (development) or existing recordings (CI/PR)
- **Workspace**: Sample workspace with test incidents
- **IDE**: Rust-capable editor with LSP support

---

## Implementation Steps

### Phase 1: Core Integration (P1 - Basic Agent Processing)

#### Step 1.1: Enhance GooseSessionWrapper
**File**: `src/goose/session.rs`

**Current State**: Skeleton with TODO comments
**Target**: Wire actual Goose Agent

```rust
// Replace TODO in initialize() method
pub async fn initialize(&mut self) -> Result<()> {
    // Create actual Goose agent
    let agent = Agent::new(
        &self.workspace_path,
        self.configuration.provider.clone(),
        self.configuration.model.clone(),
    ).await?;

    self.goose_agent = Some(agent);
    self.status = SessionStatus::Ready;

    info!("Goose agent initialized for session: {}", self.session_id);
    Ok(())
}
```

**Research Reference**: [research.md](./research.md) - Section 2 "AgentManager with Per-Session Agent Pattern"

#### Step 1.2: Implement Incident-to-Prompt Conversion
**File**: `src/goose/prompts.rs`

**Current State**: Skeleton methods
**Target**: Convert incidents to natural language prompts

```rust
pub fn format_incident_prompt(incidents: &[Incident], context: &str) -> String {
    let mut prompt = String::new();
    prompt.push_str("Please help fix these code migration issues:\n\n");

    for (i, incident) in incidents.iter().enumerate() {
        prompt.push_str(&format!(
            "{}. File: {} (Line {})\n   Issue: {} ({})\n   Description: {}\n   Details: {}\n\n",
            i + 1,
            incident.file_path,
            incident.line_number,
            incident.category,
            incident.severity,
            incident.description,
            incident.message
        ));
    }

    prompt.push_str("Please analyze the files and make the necessary changes to resolve these issues.");
    prompt
}
```

#### Step 1.3: Wire Agent Processing
**File**: `src/goose/agent.rs`

**Current State**: Simulated processing in `process_fix_request()`
**Target**: Actual Goose agent interaction

```rust
pub async fn process_fix_request(
    &self,
    request: &FixGenerationRequest,
) -> Result<(String, tokio::sync::mpsc::UnboundedReceiver<StreamMessage>)> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let handler = AgentMessageHandler::new(tx.clone());

    // Get session wrapper
    let session_wrapper = self.get_session_wrapper(&request.session_id).await?;

    // Set up message callback
    {
        let mut session = session_wrapper.write().await;
        session.set_message_callback(Arc::new(handler));
    }

    // Process with actual Goose agent
    let request_clone = request.clone();
    let session_clone = session_wrapper.clone();

    tokio::spawn(async move {
        let result = {
            let mut session = session_clone.write().await;
            // Convert incidents to prompt and send to Goose agent
            let prompt = prompts::format_incident_prompt(&request_clone.incidents, "");
            session.process_with_goose_agent(prompt).await
        };

        // Handle result and send completion messages
        // ...
    });

    Ok((request_id, rx))
}
```

### Phase 2: Event Streaming (P2 - Real-time Response Streaming)

#### Step 2.1: Implement Goose Event Bridge
**File**: `src/goose/monitoring.rs`

**Current State**: Basic monitoring structure
**Target**: Bridge Goose events to Kaiak streaming

```rust
pub struct GooseEventBridge {
    session_id: String,
    message_callback: Option<Arc<dyn MessageCallback>>,
}

impl GooseEventBridge {
    pub async fn subscribe_to_goose_events(&self, agent: &Agent) -> Result<()> {
        let mut event_stream = agent.subscribe_events();

        while let Some(event) = event_stream.next().await {
            let stream_message = self.convert_goose_event(event).await?;
            if let Some(callback) = &self.message_callback {
                callback.on_message(stream_message)?;
            }
        }

        Ok(())
    }

    async fn convert_goose_event(&self, event: AgentEvent) -> Result<StreamMessage> {
        match event {
            AgentEvent::Message(msg) => {
                // Convert to MessageContent::AiResponse
                Ok(StreamMessage {
                    id: uuid::Uuid::new_v4().to_string(),
                    session_id: self.session_id.clone(),
                    timestamp: Utc::now().to_rfc3339(),
                    sequence_number: self.get_next_sequence(),
                    content: MessageContent::AiResponse {
                        text: msg.content,
                        partial: false,
                        confidence: None,
                    },
                    source: MessageSource::GooseAgent,
                })
            }
            AgentEvent::ToolCall(tool) => {
                // Convert to MessageContent::ToolCall
                // ...
            }
            // Handle other event types
            _ => {
                // Document unsupported features (FR-010/FR-011)
                self.log_unsupported_feature(event);
                // Return system message or skip
            }
        }
    }
}
```

### Phase 3: Tool Integration (P3 - Tool Call Execution and Monitoring)

#### Step 3.1: Integrate Tool Interception
**File**: `src/goose/agent.rs`

**Current State**: Existing `create_safe_tool_call()` method
**Target**: Wire with Goose tool execution

```rust
impl GooseSessionWrapper {
    pub async fn handle_goose_tool_call(&mut self, tool_call: GooseToolCall) -> Result<()> {
        // Use existing safety infrastructure
        let safe_result = self.agent_manager.create_safe_tool_call(
            &self.session_id,
            &tool_call.tool_name,
            tool_call.parameters.clone(),
            None, // original_content - determined during execution
            None, // proposed_content - determined during execution
        ).await?;

        match safe_result {
            SafeToolCallResult::Allowed { tool_name, parameters } => {
                // Execute tool directly
                let result = self.execute_tool(tool_name, parameters).await?;
                self.send_tool_result_to_goose(tool_call.id, result).await?;
            }
            SafeToolCallResult::InterceptedForApproval { interaction, .. } => {
                // Send interaction request and wait for approval
                self.request_user_interaction(interaction).await?;
                // Approval handling will trigger execution
            }
        }

        Ok(())
    }
}
```

### Phase 4: Testing Infrastructure (FR-009)

#### Step 4.1: Implement TestProvider Integration
**File**: `tests/integration/goose_integration.rs`

**Current State**: Placeholder test
**Target**: Comprehensive end-to-end test with recording/replay

```rust
use goose::providers::testprovider::TestProvider;

#[tokio::test]
async fn test_agent_integration_end_to_end() -> Result<()> {
    // Set up test environment
    let test_workspace = setup_test_workspace().await?;
    let test_incidents = load_test_incidents("fixtures/sample_incidents.json").await?;

    // Configure TestProvider for CI/PR compatibility
    let provider = if std::env::var("CI").is_ok() {
        // Replay mode in CI
        TestProvider::new_replaying("tests/recordings/integration_test.json")?
    } else {
        // Recording mode in development
        let real_provider = create_real_provider().await?;
        TestProvider::new_recording(real_provider, "tests/recordings/integration_test.json")
    };

    // Create agent session with test provider
    let session_config = SessionConfiguration {
        provider: Some("test".to_string()),
        model: Some("test-model".to_string()),
        enable_tool_interception: true,
        ..Default::default()
    };

    // Execute complete workflow
    let kaiak_server = KaiakServer::new(config).await?;
    let session_id = kaiak_server.create_session(test_workspace, test_incidents, session_config).await?;

    // Start processing and collect streaming events
    let (request_id, mut event_stream) = kaiak_server.generate_fix(session_id).await?;

    let mut received_events = Vec::new();
    let mut processing_completed = false;

    while let Some(event) = event_stream.recv().await {
        received_events.push(event.clone());

        if matches!(event.content, MessageContent::System { event, .. } if event == "processing_completed") {
            processing_completed = true;
            break;
        }
    }

    // Verify results
    assert!(processing_completed, "Processing should complete successfully");
    assert!(!received_events.is_empty(), "Should receive streaming events");

    // Verify tool calls were made
    let tool_calls: Vec<_> = received_events.iter()
        .filter(|event| matches!(event.content, MessageContent::ToolCall { .. }))
        .collect();
    assert!(!tool_calls.is_empty(), "Should execute tool calls");

    // Verify file modifications (actual changes on disk)
    let session_status = kaiak_server.get_session_status(session_id).await?;
    assert!(!session_status.files_modified.is_empty(), "Should modify files on disk");

    // Verify success criteria (SC-001 to SC-007)
    verify_success_criteria(&received_events, &session_status).await?;

    Ok(())
}

fn verify_success_criteria(events: &[StreamMessage], status: &SessionStatus) -> Result<()> {
    // SC-001: <30s processing time
    assert!(status.processing_time_ms < 30_000, "Processing should complete in under 30s");

    // SC-002: <500ms streaming latency
    verify_streaming_latency(events)?;

    // SC-003: 95% success rate (implemented via test repeatability)
    // SC-004: 100% tool call capture
    // SC-005: 100% error handling
    // SC-006: Goose compatibility demonstration
    // SC-007: Feature gap documentation

    Ok(())
}
```

#### Step 4.2: Create Test Fixtures
**Directory**: `tests/fixtures/`

**Create Sample Data**:
1. **`sample_incidents.json`**: Realistic incidents for testing
2. **`test_workspace/`**: Sample codebase with migration issues
3. **`recordings/`**: TestProvider recordings for CI/PR

```bash
mkdir -p tests/fixtures/test_workspace/src
mkdir -p tests/recordings

# Create sample incident
cat > tests/fixtures/sample_incidents.json << 'EOF'
[
  {
    "id": "incident-001",
    "rule_id": "java-deprecated-api",
    "file_path": "src/example.java",
    "line_number": 15,
    "severity": "error",
    "description": "Deprecated API usage",
    "message": "Use of deprecated method Collections.sort()",
    "category": "deprecated-api",
    "metadata": {}
  }
]
EOF

# Create sample Java file with issue
cat > tests/fixtures/test_workspace/src/example.java << 'EOF'
import java.util.*;

public class Example {
    public void sortList(List<String> items) {
        // This uses deprecated API
        Collections.sort(items);
    }
}
EOF
```

### Phase 5: Documentation (FR-010/FR-011)

#### Step 5.1: Feature Gap Documentation
**File**: `docs/goose_ide_compatibility.md`

```markdown
# Goose-IDE Compatibility Analysis

## Advanced Goose Features Not Supported by Typical IDE Extensions

### 1. Multi-Step Orchestration
- **SubAgent Execution**: Hierarchical task processing
- **Recipe Composition**: Workflow templates with parameters
- **Parallel Processing**: Concurrent agent operations

**IDE Gap**: Most extensions support only single-request processing

### 2. Rich Event Streaming
- **Token Usage Tracking**: Real-time token consumption
- **MCP Notifications**: Server-initiated updates
- **Conversation Replacements**: Context compaction events

**IDE Gap**: Limited to basic text streaming

[... detailed analysis continues ...]
```

---

## Testing Strategy

### Development Testing
1. **Record real provider interactions** during development
2. **Commit recording files** for CI/PR repeatability
3. **Test with various incident types** for robustness

### CI/PR Testing
1. **Replay-only mode** - no API keys required
2. **Safety guards** prevent recording in automated environments
3. **Deterministic results** from committed recordings

### Integration Validation
- **End-to-end workflow**: Session create → process → stream → complete
- **Tool interception**: File modification approval workflow
- **Error handling**: Graceful degradation and recovery
- **Performance**: <30s processing, <500ms streaming latency

---

## Success Metrics Validation

### SC-001: Processing Time
```bash
# Measure in integration test
start_time=$(date +%s%N)
# ... agent processing ...
end_time=$(date +%s%N)
processing_time_ms=$(( (end_time - start_time) / 1000000 ))
assert [ $processing_time_ms -lt 30000 ]
```

### SC-002: Streaming Latency
- Monitor event timestamps during streaming
- Verify <500ms gaps between events
- Validate real-time progress updates

### SC-003: Test Success Rate
- Run integration test suite multiple times
- Target 95% success rate across varied scenarios
- Track and analyze failure patterns

### SC-004-007: Additional Criteria
- Tool call capture verification
- Error handling validation
- Goose compatibility demonstration
- Feature gap documentation completeness

---

## Troubleshooting

### Common Issues

1. **Goose Agent Initialization Fails**
   - Check provider configuration
   - Verify workspace permissions
   - Review model availability

2. **Event Streaming Disconnects**
   - Implement connection retry logic
   - Monitor for timeout conditions
   - Check memory usage patterns

3. **Tool Approval Workflow Hangs**
   - Verify interaction callback setup
   - Check approval timeout configuration
   - Review approval status tracking

4. **TestProvider Recording Issues**
   - Ensure proper provider wrapping
   - Check recording file permissions
   - Verify CI safety guards

### Debug Configuration

```toml
# Add to config for detailed logging
[logging]
level = "debug"
modules = [
    "kaiak::goose",
    "goose::agents",
    "goose::tools"
]
```

### Performance Monitoring

```rust
// Add metrics collection
use std::time::Instant;

let start = Instant::now();
// ... agent processing ...
let duration = start.elapsed();
info!("Agent processing completed in {:?}", duration);
```

---

## Next Steps

After completing this implementation:

1. **Test Comprehensive Coverage**: Verify all user stories and success criteria
2. **Performance Optimization**: Profile and optimize critical paths
3. **Documentation Review**: Ensure feature gap analysis is complete
4. **Integration Preparation**: Prepare for future IDE extension integration

## References

- [Feature Specification](./spec.md) - Complete requirements and acceptance criteria
- [Research Findings](./research.md) - Technical patterns and integration approaches
- [Data Model](./data-model.md) - Entity relationships and validation rules
- [API Contracts](./contracts/) - JSON-RPC and streaming schemas
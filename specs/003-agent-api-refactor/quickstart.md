# Quick Start Guide: Agent API Refactor for Goose Integration

**Date**: 2025-12-24
**Feature**: 003-agent-api-refactor

## Overview

This guide provides a quick start for implementing the Agent API refactor that integrates Kaiak with the Goose AI framework. The refactor simplifies the API to three endpoints while maintaining JSON-RPC protocol compatibility.

## Prerequisites

### Dependencies
- **Rust 1.75+** (stable toolchain)
- **Goose framework** (git dependency from github.com/block/goose)
- **tower-lsp 0.20** (existing JSON-RPC infrastructure)
- **tokio, serde, anyhow, tracing** (existing dependencies)

### Required Knowledge
- Goose SessionManager API patterns
- Goose Agent initialization and tool system
- JSON-RPC 2.0 protocol
- tower-lsp framework usage

## Implementation Strategy

### Phase 1: Core Goose Integration
1. **Replace session management** with Goose SessionManager
2. **Implement agent initialization** using goose::agents::Agent
3. **Update API endpoints** to three-method interface
4. **Preserve JSON-RPC protocol** and transport layer

### Phase 2: Tool System Integration
1. **Configure default Goose tools** (developer, todo, extensionmanager)
2. **Implement permission enforcement** via Goose's tool system
3. **Add custom migration tools** via MCP extensions
4. **Enable planning mode** configuration

### Phase 3: Event Streaming
1. **Map Goose AgentEvents** to Kaiak streaming notifications
2. **Maintain existing notification formats** for compatibility
3. **Implement user interaction handling** for tool approvals
4. **Preserve real-time progress updates**

## Key Code Changes

### 1. Session Management Replacement

**Before** (Custom session management):
```rust
// Remove custom SessionManager implementation
// Remove custom session persistence logic
```

**After** (Goose integration):
```rust
use goose::session::{SessionManager, SessionType};

// Create session
let session = SessionManager::create_session(
    workspace_config.working_dir,
    "Agent Session".to_string(),
    SessionType::User,
).await?;

// Delete session
SessionManager::delete_session(&session_id).await?;
```

### 2. Agent Initialization

**New agent setup pattern**:
```rust
use goose::agents::{Agent, SessionConfig, ExtensionConfig};
use goose::providers::create_with_named_model;

// Initialize agent
let agent = Agent::new();

// Set up provider
let provider = create_with_named_model(&provider_name, &model_name).await?;
agent.update_provider(provider, &session_id).await?;

// Add extensions
let config = ExtensionConfig::stdio(
    "developer",
    "goose",
    "Developer tools",
    300,
).with_args(vec!["mcp", "developer"]);
agent.add_extension(config).await?;

// Configure session
let session_config = SessionConfig {
    id: session_id,
    schedule_id: None,
    max_turns: Some(1000),
    retry_config: None,
};
```

### 3. API Endpoint Updates

**JSON-RPC method routing**:
```rust
async fn execute_command(&self, params: ExecuteCommandParams) -> JsonRpcResult<Option<Value>> {
    match params.command.as_str() {
        "kaiak/configure" => self.handle_configure(params).await,
        "kaiak/generate_fix" => self.handle_generate_fix(params).await,
        "kaiak/delete_session" => self.handle_delete_session(params).await,
        _ => Err(Error::method_not_found())
    }
}
```

### 4. Event Streaming Integration

**Map Goose events to Kaiak notifications**:
```rust
use goose::agents::AgentEvent;
use futures::StreamExt;

let mut stream = agent.reply(user_message, session_config, None).await?;
while let Some(event_result) = stream.next().await {
    match event_result? {
        AgentEvent::Message(msg) => {
            // Send kaiak/stream/ai_response notification
        },
        AgentEvent::McpNotification((req_id, notif)) => {
            // Send kaiak/stream/tool_call notification
        },
        AgentEvent::ModelChange { model, mode } => {
            // Send kaiak/stream/system notification
        },
        AgentEvent::HistoryReplaced(conv) => {
            // Send kaiak/stream/system notification
        },
    }
}
```

## Directory Structure Updates

### Modified Files
```
src/
├── server/
│   ├── server.rs          # Update execute_command routing
│   └── jsonrpc.rs         # Add new method constants
├── handlers/
│   ├── configure.rs       # NEW - configure() endpoint
│   ├── generate_fix.rs    # REFACTOR - Goose agent integration
│   ├── delete_session.rs  # NEW - delete_session() endpoint
│   └── lifecycle.rs       # REMOVE - custom session management
├── models/
│   ├── configuration.rs   # UPDATE - nested JSON structure
│   └── events.rs          # UPDATE - Goose event mapping
└── agents/
    └── goose_integration.rs # NEW - Goose agent wrapper
```

### New Dependencies in Cargo.toml
```toml
[dependencies]
goose = { git = "https://github.com/block/goose" }
# ... existing dependencies remain
```

## Testing Strategy

### Integration Tests to Add
```
tests/integration/
├── goose_session.rs       # SessionManager integration
├── agent_lifecycle.rs     # Agent initialization and tools
├── api_endpoints.rs       # Three-endpoint API validation
└── event_streaming.rs     # Goose event → Kaiak notification mapping
```

### Tests to Remove
```
tests/integration/
├── custom_session.rs      # Remove - replaced by Goose
└── old_endpoints.rs       # Remove - deprecated API methods
```

## Configuration Example

### Agent Configuration Structure
```json
{
  "workspace": {
    "working_dir": "/path/to/project",
    "include_patterns": ["**/*.java"],
    "exclude_patterns": ["target/**"]
  },
  "model": {
    "provider": "databricks",
    "model": "databricks-meta-llama-3-1-405b-instruct"
  },
  "tools": {
    "enabled_extensions": ["developer", "todo"],
    "planning_mode": true
  },
  "session": {
    "max_turns": 1000
  },
  "permissions": {
    "tool_permissions": {
      "read_file": "allow",
      "write_file": "approve",
      "shell_command": "deny"
    }
  }
}
```

## Development Workflow

### 1. Setup Development Environment
```bash
# Clone Goose dependency
git clone https://github.com/block/goose ~/Projects/goose

# Update Cargo.toml
cargo add --git https://github.com/block/goose goose

# Install dependencies
cargo build
```

### 2. Implement Core Changes
1. **Start with session management**: Replace custom session logic with Goose SessionManager
2. **Add agent initialization**: Implement Goose agent setup and configuration
3. **Update API routing**: Modify execute_command to handle three endpoints
4. **Test incrementally**: Verify each component works with Goose integration

### 3. Preserve Existing Infrastructure
- **Keep transport layer unchanged**: stdio/Unix socket support
- **Maintain JSON-RPC format**: Existing message structure and error codes
- **Preserve streaming notifications**: Existing notification method names and formats

### 4. Validation Points
- **Session operations**: Create, reuse, delete via Goose SessionManager
- **Agent functionality**: Tool execution, permission enforcement, planning mode
- **Event streaming**: Real-time progress updates and user interactions
- **Error handling**: Appropriate error codes and recovery mechanisms

## Migration Timeline

### Week 1: Foundation
- [ ] Goose dependency integration
- [ ] Session management replacement
- [ ] Basic agent initialization

### Week 2: API Implementation
- [ ] Three-endpoint API structure
- [ ] Request/response handling
- [ ] Error handling updates

### Week 3: Tool Integration
- [ ] Default tool configuration
- [ ] Permission system mapping
- [ ] Custom tool support

### Week 4: Event System
- [ ] Goose event streaming
- [ ] Notification mapping
- [ ] User interaction handling

### Week 5: Testing & Documentation
- [ ] Integration test updates
- [ ] Documentation updates
- [ ] Performance validation

## Troubleshooting

### Common Issues

1. **Goose session creation fails**
   - Verify workspace directory exists and is accessible
   - Check Goose dependency is correctly configured

2. **Agent initialization errors**
   - Ensure provider configuration is valid
   - Verify model name and provider combination

3. **Tool permission conflicts**
   - Review tool_permissions configuration
   - Check Goose default tool availability

4. **Streaming notification failures**
   - Verify JSON-RPC notification format compatibility
   - Check tower-lsp client notification handling

### Debug Commands
```bash
# Test Goose session creation
cargo test session_manager_integration

# Validate agent initialization
cargo test agent_initialization

# Check API endpoint routing
cargo test three_endpoint_api

# Verify event streaming
cargo test goose_event_mapping
```

## Performance Expectations

### Improvements
- **30% server startup time** improvement (simplified initialization)
- **20% memory usage** reduction (eliminate redundant session management)
- **<100ms event streaming** latency (Goose's optimized event system)
- **25% test execution time** reduction (fewer, focused tests)

### Monitoring Points
- Session creation/deletion performance
- Agent initialization latency
- Tool execution timing
- Memory usage during concurrent sessions

This quick start provides the essential information to begin implementing the agent API refactor with confidence and clear direction.
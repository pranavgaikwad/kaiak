# Quickstart Guide: Kaiak Client Implementation

**Date**: 2025-12-27
**Feature**: 004-kaiak-client
**Status**: Phase 1 Design

This guide provides developers with everything needed to understand and implement the Kaiak client functionality.

## Overview

The Kaiak client implementation adds comprehensive client-server CLI architecture to the existing Kaiak project. Users can now:

1. **Start a server** with flexible configuration options
2. **Connect a client** to the server for remote operations
3. **Execute AI procedures** remotely (generate_fix, configure, delete_session)
4. **Manage connection state** persistently across terminal sessions

## Architecture Summary

```
CLI Commands:
├── kaiak serve [--socket /path] [--config file.toml]     # Server management
├── kaiak connect --socket /path                          # Client connection
├── kaiak disconnect                                       # Client disconnection
├── kaiak configure --input config.json                   # Server base config
├── kaiak generate_fix --input incidents.json             # AI fix generation
└── kaiak delete_session --session <uuid>                 # Session cleanup

Configuration Hierarchy:
CLI args > user config > default config > hardcoded defaults

State Persistence:
~/.kaiak/client.state   # Client connection state
~/.kaiak/server.conf    # Server configuration (TOML)
```

## Key Design Decisions

### 1. Configuration Unification

**Problem**: Two separate configuration systems (ServerSettings and AgentConfiguration) caused confusion.

**Solution**: Unified configuration structure:
```rust
ServerConfig {
    init_config: InitConfig,      // Immutable, set at server start
    base_config: BaseConfig,      // Mutable via configure()
}

AgentConfig {
    workspace: WorkspaceConfig,
    session: GooseSessionConfig,
    override_base_config: BaseConfig,  // Overrides server's base_config
}
```

**Impact**:
- `configure()` only changes server's BaseConfig
- `generate_fix()` handles session creation with AgentConfig
- Eliminates confusion between server-wide and session-specific settings

### 2. Client State Persistence

**Problem**: CLI users need persistent connections across terminal sessions.

**Solution**: File-based state at `~/.kaiak/client.state`
```rust
ClientConnection {
    socket_path: String,
    connected_at: DateTime<Utc>,
    last_validated: Option<DateTime<Utc>>,
}
```

**Benefits**:
- Connections persist across terminal sessions
- Simple JSON format for debugging
- Automatic validation and cleanup

### 3. JSON-RPC Client Architecture

**Problem**: Need client-side transport that mirrors server capabilities.

**Solution**: `JsonRpcClient` that mirrors `KaiakServer`
```rust
JsonRpcClient {
    // Unix socket connection management
    // JSON-RPC request/response handling
    // Error handling with user-friendly messages
}
```

**Benefits**:
- Symmetric client-server architecture
- Reuses existing error codes and request/response types
- Enterprise-safe communication (Unix sockets only)

## Implementation Roadmap

### Phase 1: Configuration Unification ✅ (Foundation)
**Files to modify:**
- Move `src/config/settings.rs` → `src/models/configuration.rs`
- Create `src/logging.rs` (extract from `src/config/mod.rs`)
- Update `src/main.rs` to use unified config
- Delete `src/config/` directory

**Key structures:**
```rust
// src/models/configuration.rs
pub struct ServerConfig {
    pub init_config: InitConfig,
    pub base_config: BaseConfig,
}

pub struct AgentConfig {
    pub workspace: WorkspaceConfig,
    pub session: GooseSessionConfig,
    pub override_base_config: BaseConfig,
}
```

### Phase 2: Client Infrastructure (Core)
**Files to create:**
- `src/models/client.rs` - ClientConnection and ClientState
- `src/client/transport.rs` - JsonRpcClient implementation
- `src/client/mod.rs` - Client module exports

**Key functionality:**
- File-based state persistence
- JSON-RPC client transport
- Connection validation and error handling

### Phase 3: CLI Commands (User-Facing)
**Files to create/modify:**
- `src/cli/commands/connect.rs` - Connection management
- `src/cli/commands/disconnect.rs` - Disconnection
- `src/cli/commands/client_ops.rs` - Remote procedure execution
- Update `src/main.rs` - Add client commands to CLI

**Commands to implement:**
```bash
kaiak serve --socket /tmp/kaiak.sock
kaiak connect --socket /tmp/kaiak.sock
kaiak configure --input-json '{"model": {...}}'
kaiak generate_fix --input incidents.json
kaiak delete_session --session <uuid>
kaiak disconnect
```

### Phase 4: Integration & Testing (Validation)
**Files to create:**
- `tests/test_client.rs` - Comprehensive client integration tests
- Address placeholder code identified in research

**Testing priorities:**
1. End-to-end client-server communication
2. Configuration precedence validation
3. State persistence across sessions
4. Error handling and user experience

## Usage Examples

### Basic Client-Server Workflow

1. **Start server with socket transport:**
   ```bash
   kaiak serve --socket /tmp/kaiak.sock
   ```

2. **Connect client (in another terminal):**
   ```bash
   kaiak connect --socket /tmp/kaiak.sock
   ```

3. **Configure server base settings:**
   ```bash
   kaiak configure --input-json '{
     "model": {"provider": "openai", "model": "gpt-4"},
     "tools": {"enabled_extensions": ["developer", "todo"]},
     "permissions": {"tool_permissions": {"read_file": "allow"}}
   }'
   ```

4. **Generate fixes with session-specific config:**
   ```bash
   kaiak generate_fix --input-json '{
     "incidents": [{"id": "issue-1", "rule_id": "deprecated-api", "message": "Fix deprecated method"}],
     "agentConfig": {
       "workspace": {"working_dir": "/path/to/project"},
       "session": {"name": "Migration Session"},
       "overrideBaseConfig": {"model": {"provider": "anthropic"}}
     }
   }'
   ```

5. **Clean up session:**
   ```bash
   kaiak delete_session --session <uuid-from-previous-response>
   ```

6. **Disconnect client:**
   ```bash
   kaiak disconnect
   ```

### Configuration File Workflow

1. **Create server config:**
   ```toml
   # ~/.kaiak/server.conf
   [init_config]
   transport = "socket"
   socket_path = "/tmp/kaiak.sock"
   log_level = "info"
   max_concurrent_sessions = 10

   [base_config.model]
   provider = "openai"
   model = "gpt-4"

   [base_config.tools]
   enabled_extensions = ["developer", "todo"]
   planning_mode = false

   [base_config.permissions.tool_permissions]
   read_file = "allow"
   write_file = "approve"
   ```

2. **Start server with config file:**
   ```bash
   kaiak serve  # Uses ~/.kaiak/server.conf by default
   ```

3. **Create agent config:**
   ```json
   {
     "workspace": {
       "working_dir": "/path/to/project",
       "include_patterns": ["**/*.rs"],
       "exclude_patterns": ["target/**"]
     },
     "overrideBaseConfig": {
       "model": {"provider": "anthropic", "model": "claude-3-sonnet"}
     }
   }
   ```

4. **Generate fixes with file-based input:**
   ```bash
   kaiak generate_fix --input incidents.json
   # incidents.json contains both incidents array and agentConfig
   ```

## Error Handling Examples

### Connection Issues
```bash
$ kaiak generate_fix --input test.json
Error: No server connection found.
Run 'kaiak connect --socket /path/to/socket' to connect.

$ kaiak connect --socket /nonexistent/path
Error: Failed to connect to server at '/nonexistent/path'.
Error: No such file or directory

Troubleshooting:
• Verify server is running: kaiak serve --socket /nonexistent/path
• Check socket permissions
• Ensure path exists
```

### Configuration Errors
```bash
$ kaiak configure --input-json '{"invalid": true}'
Error: Configuration validation failed.

Missing required fields:
• model: Model configuration is required
• tools: Tool configuration is required

$ kaiak generate_fix --input malformed.json
Error: Failed to parse JSON input.
Invalid JSON syntax: unexpected end of file at line 5

Hint: Validate JSON online: https://jsonlint.com/
```

## Development Notes

### Code Reuse Strategy (80% reusable)

**Direct reuse available:**
- `AgentConfiguration` and sub-structs → Use as-is for AgentConfig
- Handler request/response types → Use same types in client
- Error codes and methods → Import from `server::jsonrpc`
- `KaiakError` enum → Add one variant for client connections

**New abstractions needed:**
- `ClientConnection` - Client state management
- `JsonRpcClient` - Client-side transport
- `UnifiedConfig` - Configuration hierarchy
- Client CLI commands - Extension of existing CLI

### Critical Issues to Address

**Blocking issues (must fix for client implementation):**
1. Handler wiring in `server.rs` - Handlers exist but not connected
2. Transport layer LSP integration - Currently placeholder code
3. Server initialization - Handler initialization incomplete

**Production readiness issues:**
1. Doctor command health checks - All placeholder implementations
2. Permission enforcement - Only logged, not enforced
3. Environment variable handling - Needs better error messages

## Testing Strategy

### Integration Test Coverage
- `tests/test_client.rs` - Comprehensive client integration tests
- Server startup/shutdown lifecycle
- Client connection/disconnection flows
- Remote procedure execution (all three methods)
- Error handling scenarios
- Configuration precedence validation

### Unit Test Coverage
- `ClientState` persistence and validation
- `ConfigurationHierarchy` precedence logic
- `JsonRpcClient` request/response handling
- Error message formatting and user guidance

### End-to-End Scenarios
1. **Happy path**: Server start → client connect → configure → generate_fix → delete_session → disconnect
2. **Error scenarios**: Connection failures, invalid JSON, missing sessions
3. **Configuration scenarios**: All four precedence levels, TOML ↔ JSON conversion
4. **State scenarios**: Connection persistence across terminal sessions

## Next Steps

1. **Complete Phase 1**: Configuration unification and foundation
2. **Implement Phase 2**: Client infrastructure and transport layer
3. **Build Phase 3**: CLI commands and user interface
4. **Validate Phase 4**: Integration testing and production readiness

**Ready for implementation** - All design decisions documented, architecture validated, and implementation path clear.

---

**Documentation Complete** - Ready for `/speckit.tasks` to generate detailed implementation tasks.
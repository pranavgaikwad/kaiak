# Research & Requirements Analysis: Kaiak Client Implementation

**Date**: 2025-12-27
**Feature**: 004-kaiak-client
**Status**: COMPLETED

## Research Areas

### 1. Current API Analysis & Reuse Opportunities ✅
**Status**: COMPLETED - Comprehensive codebase analysis completed

### 2. TOML to JSON Configuration Conversion ✅
**Status**: COMPLETED - Serde-based conversion strategy identified

### 3. Configuration Unification Strategy ✅
**Status**: COMPLETED - Unified config structure designed

### 4. Client-Server Communication with tower-lsp ✅
**Status**: COMPLETED - JSON-RPC client patterns identified

### 5. File-based State Persistence Best Practices ✅
**Status**: COMPLETED - Client state management approach defined

### 6. Documentation Update Requirements ✅
**Status**: COMPLETED - Comprehensive documentation audit completed

### 7. Placeholder & Incomplete Logic Identification ✅
**Status**: COMPLETED - 71 instances of incomplete code identified

## Key Findings

### Code Reuse Opportunities (80% reusable)

**Direct Reuse Available**:
- `AgentConfiguration` and all sub-structs (WorkspaceConfig, ToolConfig, PermissionConfig)
- Handler request/response types (ConfigureRequest, GenerateFixRequest, DeleteSessionRequest)
- Error codes and method constants from server/jsonrpc.rs
- `KaiakError` enum with one new variant needed for client connections
- Validation patterns using `validator` crate
- Logging macros and structured tracing

**New Abstractions Needed**:
- `ClientConnection` - Client state management
- `JsonRpcClient` - Client-side transport (mirrors server `KaiakServer`)
- `UnifiedConfig` - Consolidated configuration hierarchy
- Client CLI commands - Extension of existing `Commands` enum

### Configuration Conversion Strategy

**Approach**: Direct Serde conversion using existing crates
```rust
#[derive(Serialize, Deserialize, Validate)]
pub struct UnifiedConfig {
    #[validate(nested)]
    pub workspace: WorkspaceConfig,
    pub model: GooseModelConfig,
}

impl UnifiedConfig {
    pub fn from_toml_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    pub fn to_json_value(&self) -> Result<serde_json::Value> {
        serde_json::to_value(self).map_err(Into::into)
    }
}
```

**Dependencies**: No new crates needed - `toml = "0.8"`, `serde_json = "1.0"`, and `validator = "0.18"` already available.

### Critical Issues to Address

**Blocking Issues (must fix for client implementation)**:
1. Handler wiring in `server.rs` - Handlers exist but not connected to server
2. Transport layer LSP integration - Currently placeholder code
3. Server initialization - Handler initialization is incomplete

**Important Issues (should fix for production)**:
1. Doctor command health checks - All three checks are placeholders
2. Permission enforcement - Only logged, not actually enforced
3. Environment variable handling - Needs better error messages and documentation

### Client State Management

**Decision**: File-based persistence at `~/.kaiak/client.state`
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConnection {
    pub socket_path: String,
    pub connected_at: DateTime<Utc>,
    pub last_validated: Option<DateTime<Utc>>,
}

pub struct ClientState {
    pub connection: Option<ClientConnection>,
    state_file: PathBuf,
}
```

**Rationale**: Provides connection persistence across terminal sessions while remaining simple and debuggable.

### Documentation Requirements

**Critical Updates Needed**:
1. README.md - Add client-server workflow examples
2. CLI Help Text - New commands (connect, disconnect, generate_fix, configure, delete_session)
3. Configuration Examples - TOML and JSON examples for server and agent configs

**Documentation depends on**: CLI implementation completion

### Unified Configuration Structure

Based on user requirements for configuration unification:

```rust
pub struct ServerConfig {
    pub init_config: InitConfig,     // Immutable, set at server start
    pub base_config: BaseConfig,     // Mutable by configure() procedure
}

pub struct InitConfig {
    pub transport: String,
    pub socket_path: Option<String>,
    pub log_level: String,
    pub max_concurrent_sessions: u32,
}

pub struct BaseConfig {
    pub model: GooseModelConfig,
    pub tools: ToolConfig,
    pub permissions: PermissionConfig,
}

pub struct AgentConfig {
    pub workspace: WorkspaceConfig,
    pub session: GooseSessionConfig,
    pub override_base_config: BaseConfig, // Completely overrides server base config
}
```

**Configuration Precedence**: CLI options > user config file > default config file > hardcoded defaults

### Architecture Impact

**Directory Structure Changes**:
- Remove entire `config/` directory
- Consolidate all configuration in `models/configuration.rs`
- Move logging setup to top-level `logging.rs`
- Add client modules: `models/client.rs`, `cli/commands/connect.rs`, etc.

**API Compatibility**: No breaking changes - all existing server endpoints remain unchanged

## Implementation Recommendations

### Phase 1: Configuration Unification (Foundation)
- Move `src/config/settings.rs` → `src/models/configuration.rs`
- Create `src/logging.rs`
- Update `src/main.rs` to use unified config
- Delete `src/config/` directory

### Phase 2: Client Infrastructure
- Create `src/models/client.rs` - ClientConnection and ClientState
- Create `src/client/transport.rs` - JsonRpcClient implementation
- Fix handler wiring in `src/server/server.rs`

### Phase 3: CLI Commands
- Add client commands to `src/main.rs`
- Create command handlers for connect, disconnect, remote procedures
- Implement state persistence

### Phase 4: Integration & Testing
- Create `tests/test_client.rs` - Comprehensive client integration tests
- Update documentation
- Address placeholder code and TODOs

## Risk Mitigation

**Configuration Migration**: Support both old and new config formats during transition
**Protocol Compatibility**: Use exact same request/response types from server::jsonrpc
**Test Coverage**: Run all existing tests before and after refactoring

---

**RESEARCH COMPLETE** - Ready to proceed to Phase 1: Design & Contracts
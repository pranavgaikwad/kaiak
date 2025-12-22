# Research Findings: Kaiak Migration Server Skeleton

**Created**: 2025-12-22
**Purpose**: Resolve technical unknowns for implementing Kaiak server architecture

## JSON-RPC Library Selection

**Decision**: Use `tower-lsp` as the primary JSON-RPC framework

**Rationale**:
- Production-ready with comprehensive LSP 3.17+ support
- Handles JSON-RPC 2.0 with Content-Length headers automatically
- Native async/await support with Tokio runtime
- Active community and excellent documentation
- Designed specifically for VSCode extension compatibility
- Supports multiple transport layers (stdio, sockets)

**Alternatives Considered**:
- `async-lsp`: More flexible but requires more low-level implementation
- `lsp-server`: Battle-tested in rust-analyzer but synchronous API only
- `jsonrpc-ipc-server`: Lower-level, requires manual LSP protocol handling

## Goose Agent Integration Architecture

**Decision**: Use Goose's `AgentManager` singleton pattern with session-based isolation

**Rationale**:
- Provides built-in session lifecycle management
- Automatic LRU eviction for memory management
- Thread-safe concurrent session handling
- Native support for provider configuration and extension management
- Established pattern used in Goose's own server implementation

**Integration Pattern**:
```rust
// Singleton manager for multiple sessions
let manager = AgentManager::instance().await?;

// Session-specific agents with isolated state
let agent = manager.get_or_create_agent(session_id).await?;

// Streaming event processing
let mut stream = agent.reply(message, config, cancel_token).await?;
while let Some(event) = stream.next().await {
    // Forward events to IDE via JSON-RPC
}
```

**Key Dependencies**:
- `goose` crate (github.com/block/goose)
- `tokio` with full feature set for async runtime
- `futures` for stream processing
- `anyhow` for error handling

## IPC Transport Strategy

**Decision**: Implement stdio as primary transport with Unix sockets as fallback

**Rationale**:
- Stdio is the recommended approach for VSCode extensions (LSP standard)
- Simplest setup with no file management or port conflicts
- Process isolation provides enterprise security requirements
- Cross-platform compatibility without platform-specific code
- Native support in VSCode's language client library

**Implementation Approach**:
- Primary: `tower-lsp`'s built-in stdio transport
- Fallback: Unix domain sockets using `tokio::net::UnixListener`
- Configuration flag to switch between transports
- Both transports use identical JSON-RPC message format

**Transport Comparison**:
| Transport | Setup Complexity | Performance | Security | VSCode Support |
|-----------|-----------------|-------------|----------|----------------|
| Stdio     | Minimal         | Adequate    | High     | Native         |
| Unix Sockets | Low           | High        | High     | Manual         |
| Named Pipes | Medium         | High        | Medium   | Manual         |

## WASM Distribution Considerations

**Decision**: Target native binary distribution initially, WASM as future consideration

**Rationale**:
- Goose agent has native dependencies (process spawning, filesystem access)
- IPC transports require system-level socket APIs
- Performance requirements favor native execution
- Enterprise deployment scenarios prefer native binaries
- WASM limitations would require significant architecture changes

**Future WASM Path**:
- Subset functionality using web-compatible APIs
- Browser-based IDE integration (not VSCode)
- Limited agent capabilities (no local tool execution)

## Testing Strategy

**Decision**: E2E/Integration test pyramid with minimal unit tests

**Test Categories**:

1. **Integration Tests** (Priority 1):
   - Complete fix generation workflow with real Goose agent
   - Real-time streaming validation
   - User interaction approval flow
   - Agent lifecycle management

2. **Contract Tests** (Priority 2):
   - JSON-RPC protocol compliance with LSP spec
   - Goose agent API integration contracts
   - Message format validation

3. **Unit Tests** (Priority 3):
   - Complex prompt generation logic
   - Session state management utilities
   - Error handling and recovery mechanisms

**Test Infrastructure**:
- `cargo test` with async support
- Mock Goose agent for deterministic testing
- Integration test harness with real transport layers
- GitHub Actions for automated CI/CD

## CI/CD Pipeline Design

**Decision**: GitHub Actions with comprehensive automation and local script mirroring

**Pipeline Stages**:
1. **Code Quality**: `cargo fmt`, `cargo clippy`, security audit
2. **Testing**: Unit tests, integration tests, contract tests
3. **Build**: Multi-target compilation (Linux, macOS, Windows)
4. **Security**: Dependency scanning, vulnerability assessment
5. **Distribution**: Binary artifacts with checksums

**Local Development**:
- `scripts/ci.sh`: Mirror GitHub Actions locally
- `scripts/test.sh`: Comprehensive test runner
- `scripts/lint.sh`: Code quality checks
- Pre-commit hooks for quality gates

## Dependency Management

**Decision**: Minimal dependency set with justified inclusions

**Core Dependencies**:
- `goose`: AI agent integration (github.com/block/goose)
- `tower-lsp`: JSON-RPC and LSP protocol handling
- `tokio`: Async runtime with specific feature flags
- `serde`: Serialization for JSON-RPC messages
- `anyhow`: Error handling and context propagation

**Development Dependencies**:
- `tokio-test`: Async test utilities
- `mockall`: Mock generation for testing
- `tempfile`: Temporary file handling in tests

**Security Considerations**:
- Regular dependency audits with `cargo-audit`
- Pinned versions for reproducible builds
- Minimal feature flags to reduce attack surface
- Enterprise-approved crate verification

## Performance Requirements Resolution

**Decision**: Target specifications based on user experience requirements

**Performance Targets**:
- Request acknowledgment: <2 seconds (meets SC-001)
- Streaming latency: <500ms (meets SC-002)
- Concurrent sessions: 10+ without degradation (meets SC-003)
- Processing time: <5 minutes for 10k file codebases (meets SC-007)

**Optimization Strategies**:
- Async/await throughout for non-blocking operations
- Connection pooling for agent sessions
- Streaming responses to minimize perceived latency
- Efficient JSON-RPC message batching
- Memory-efficient session management with LRU eviction

## Error Handling and Recovery

**Decision**: Layered error handling with graceful degradation

**Error Categories**:
1. **Transport Errors**: Connection failures, protocol violations
2. **Agent Errors**: Goose initialization, processing failures
3. **User Errors**: Invalid requests, malformed data
4. **System Errors**: Resource exhaustion, configuration issues

**Recovery Mechanisms**:
- Automatic retry with exponential backoff
- Circuit breaker pattern for failing dependencies
- Graceful session cleanup on connection loss
- Comprehensive error logging for debugging

**Error Propagation**:
- `anyhow::Result` for application-level errors
- JSON-RPC error responses for client communication
- Structured logging with context preservation
- User-friendly error messages with technical details in logs

## Session State Management

**Decision**: Hybrid approach using Goose's persistence with in-memory caching

**State Storage**:
- Primary: Goose's SQLite session database
- Cache: In-memory session metadata for active connections
- Cleanup: Automatic LRU eviction and graceful shutdown

**Session Lifecycle**:
1. **Creation**: New session in Goose database
2. **Activation**: Load into memory cache
3. **Processing**: Stream updates via JSON-RPC
4. **Persistence**: Automatic background saving
5. **Cleanup**: Remove from cache, preserve in database

**Concurrency**:
- Thread-safe session access via `Arc<Mutex<>>`
- Isolated agent instances per session
- Non-blocking session operations

## Configuration Management

**Decision**: Environment-aware configuration with sensible defaults

**Configuration Sources** (priority order):
1. Command-line arguments
2. Environment variables
3. Configuration file
4. Compiled-in defaults

**Key Configuration Areas**:
- Transport selection (stdio/socket)
- Goose agent settings (provider, tools, prompts)
- Performance tuning (timeouts, limits)
- Logging and monitoring settings
- Security parameters (permissions, validation)

## Summary

All technical unknowns have been resolved with specific technology choices and implementation patterns. The architecture leverages proven libraries and patterns while meeting constitutional requirements for enterprise safety, testing integrity, and user experience. The next phase can proceed with detailed design and contract generation.
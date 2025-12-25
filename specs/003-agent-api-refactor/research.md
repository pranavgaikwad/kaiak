# Research: Agent API Refactor for Goose Integration

**Date**: 2025-12-24
**Feature**: 003-agent-api-refactor

## Research Summary

This document consolidates research findings for refactoring Kaiak's agent API to integrate with the Goose AI framework, simplify the API surface, and delegate session management to Goose's native capabilities.

## 1. Goose Session Management Integration

### Decision: Use Goose SessionManager with SessionType::User

**Rationale**: Goose provides a mature, SQLite-based session management system with automatic ID generation, transaction safety, and concurrent access patterns that eliminate the need for custom session persistence.

**Key API Patterns**:

```rust
// Session creation
let session = SessionManager::create_session(
    working_dir,
    "Custom Agent Session".to_string(),
    SessionType::User,
).await?;

// Session retrieval
let session = SessionManager::get_session(&session_id, include_messages).await?;

// Session cleanup
SessionManager::delete_session(&session_id).await?;

// Session updates via builder pattern
SessionManager::update_session(&session_id)
    .user_provided_name("Updated Name")
    .apply()
    .await?;
```

**Implementation Requirements**:
- Replace all custom session CRUD operations with Goose SessionManager calls
- Use client-generated UUIDs as session identifiers (validated by Goose)
- Implement session locking to prevent concurrent access to the same session
- Remove custom session persistence logic and data structures

**Alternatives Considered**:
- Hybrid approach (partial Goose integration) - Rejected due to complexity
- Custom session wrapper - Rejected due to maintenance overhead
- Direct Goose session exposure - Rejected due to API contract requirements

## 2. Goose Agent Initialization and Tool System

### Decision: Standard Agent initialization with MCP tool integration

**Rationale**: Goose's Agent system provides robust tool management, permission enforcement, and streaming capabilities that align with Kaiak's requirements for migration incident processing.

**Key API Patterns**:

```rust
// Agent initialization
let agent = Agent::new();
let provider = create_with_named_model("databricks", "model-name").await?;
agent.update_provider(provider, &session_id).await?;

// Session configuration
let session_config = SessionConfig {
    id: session.id,
    schedule_id: None,
    max_turns: Some(1000),
    retry_config: None,
};

// Extension/tool addition
let config = ExtensionConfig::stdio(
    "developer",
    "./target/debug/goose",
    "Migration tools extension",
    300,
).with_args(vec!["mcp", "developer"]);
agent.add_extension(config).await?;

// Event streaming
let mut stream = agent.reply(user_message, session_config, None).await?;
while let Some(event_result) = stream.next().await {
    match event_result? {
        AgentEvent::Message(msg) => { /* Handle messages */ },
        AgentEvent::McpNotification((req_id, notif)) => { /* Handle tool calls */ },
        AgentEvent::ModelChange { model, mode } => { /* Handle model changes */ },
        AgentEvent::HistoryReplaced(conv) => { /* Handle conversation compaction */ },
    }
}
```

**Permission System Integration**:
- Use Goose's PermissionInspector with SmartApprove mode
- Mark migration tools as `read_only_hint: true` for auto-approval where appropriate
- Implement PermissionConfirmation handling for user interactions
- Maintain existing tool permission enforcement within Goose framework

**Tool System**:
- Default Goose tools automatically available (todo, chatrecall, extensionmanager)
- Custom migration tools via MCP extensions
- Platform tools for built-in capabilities
- Frontend tools for client-side operations

**Alternatives Considered**:
- Direct tool integration - Rejected due to permission complexity
- Custom agent wrapper - Rejected due to maintenance overhead
- Hybrid tool system - Rejected due to inconsistent behavior

## 3. API Surface Simplification

### Decision: Three-endpoint API with JSON-RPC protocol preservation

**Rationale**: Simplifying to configure(), generate_fix(), and delete_session() reduces complexity while maintaining existing JSON-RPC transport and message formatting for seamless client integration.

**Endpoint Specifications**:

1. **`configure(config: AgentConfiguration) -> Result<()>`**
   - Accepts structured JSON with nested sections (workspace, model, tools)
   - Updates agent provider and extension configuration
   - Supports multiple reconfiguration calls during operation
   - Returns success/error status

2. **`generate_fix(session_id: String, incidents: Vec<MigrationIncident>) -> Stream<AgentEvent>`**
   - Accepts client-generated UUID session IDs
   - Creates new Goose session if session ID is unknown
   - Reuses existing Goose session if session ID exists
   - Blocks concurrent access to same session ID
   - Streams agent events via existing JSON-RPC transport
   - Returns success/error status on completion

3. **`delete_session(session_id: String) -> Result<()>`**
   - Delegates to Goose SessionManager::delete_session()
   - Performs transactional cleanup of session and messages
   - Returns success/error status

**Implementation Changes**:
- Remove all existing custom session creation/management endpoints
- Preserve existing JSON-RPC protocol and transport layer
- Update request/response handlers for three-endpoint contract
- Maintain streaming notification patterns for real-time updates

**Alternatives Considered**:
- RESTful API - Rejected due to transport requirements
- GraphQL - Rejected due to complexity overhead
- Extended endpoint set - Rejected due to simplification goals

## 4. Testing Strategy

### Decision: Comprehensive test refactoring with Goose integration focus

**Rationale**: Test updates are critical for validating the refactor while maintaining quality standards and ensuring proper Goose integration.

**Test Categories**:

1. **Integration Tests (Priority 1)**:
   - Goose SessionManager integration
   - Agent initialization and tool availability
   - Three-endpoint API behavior
   - Event streaming via JSON-RPC
   - Permission enforcement

2. **Unit Tests**:
   - Model validation (updated for nested JSON configuration)
   - Handler logic (updated for three endpoints)
   - Error handling and edge cases

3. **Removed Tests**:
   - Custom session management tests
   - Deprecated endpoint tests
   - Custom persistence logic tests

**Coverage Requirements**:
- Maintain >90% test coverage for new Goose integration paths
- Remove outdated tests to achieve 25% test suite execution time reduction
- Validate all three endpoints with comprehensive acceptance scenarios

**Alternatives Considered**:
- Gradual test migration - Rejected due to complexity
- Test-last approach - Rejected due to constitutional requirements
- Minimal test updates - Rejected due to integration complexity

## 5. Documentation Updates

### Decision: Comprehensive documentation refresh for simplified API

**Rationale**: Documentation must reflect the simplified API contract and Goose integration patterns to support client adoption and maintenance.

**Documentation Requirements**:
- Update API documentation for three-endpoint interface
- Document Goose integration patterns and best practices
- Remove references to deprecated endpoints
- Add migration guide for existing clients
- Update development setup instructions for Goose dependencies

**Scope**:
- README.md updates
- API contract documentation
- Integration examples
- Developer guides
- Deployment documentation

**Alternatives Considered**:
- Minimal documentation updates - Rejected due to maintenance requirements
- Separate documentation project - Rejected due to fragmentation concerns

## 6. Performance Optimization Opportunities

### Decision: Leverage Goose optimizations for performance improvements

**Rationale**: Goose's mature session management and agent system provide opportunities for significant performance improvements over custom implementations.

**Expected Improvements**:
- 30% server startup time improvement (simplified initialization)
- 20% memory usage reduction (eliminated redundant session management)
- <100ms event streaming latency (Goose's optimized event system)
- 25% test suite execution time reduction (fewer, more focused tests)

**Implementation Focus**:
- Remove custom session persistence overhead
- Leverage Goose's SQLite WAL mode for concurrent access
- Utilize Goose's automatic context management
- Implement efficient event streaming patterns

## 7. Migration Risks and Mitigation

### Risk Assessment:

1. **High Risk**: Goose API compatibility changes
   - **Mitigation**: Pin specific Goose version, comprehensive integration tests

2. **Medium Risk**: JSON-RPC protocol incompatibilities
   - **Mitigation**: Preserve existing transport layer, thorough protocol testing

3. **Medium Risk**: Tool permission enforcement gaps
   - **Mitigation**: Comprehensive permission testing, gradual tool migration

4. **Low Risk**: Performance regression during transition
   - **Mitigation**: Performance benchmarking, rollback procedures

## 8. Implementation Dependencies

### Required Knowledge Areas:
- [COMPLETED] Goose SessionManager API patterns and lifecycle
- [COMPLETED] Goose Agent initialization and tool integration
- [COMPLETED] Current Kaiak models and handlers requiring updates
- [COMPLETED] Current JSON-RPC message format compatibility

### External Dependencies:
- Goose framework (git dependency)
- Existing JSON-RPC transport layer
- Current migration incident processing workflows
- IDE extension integration patterns

## 9. Success Metrics

### Technical Metrics:
- API surface reduced to exactly 3 endpoints
- Zero custom session persistence logic in codebase
- 100% Goose SessionManager usage for session operations
- >90% test coverage for Goose integration paths

### Performance Metrics:
- <100ms event streaming latency
- 30% server startup improvement
- 20% memory usage reduction
- 25% test execution time improvement

### Quality Metrics:
- 100% API documentation completeness
- Zero broken documentation references
- 100% existing tool permission enforcement preservation

## 10. Next Steps

1. **Phase 1 - Design & Contracts**: Create data models and API contracts based on research findings
2. **Phase 1 - Agent Context Update**: Update development context with new technologies
3. **Phase 2 - Implementation Tasks**: Generate detailed implementation tasks from design artifacts

## 11. Current Kaiak Implementation Analysis

### Current API Structure:
- **Primary Handler**: `src/handlers/lifecycle.rs` with session lifecycle management
- **Current Endpoints**: create_session, terminate_session, initialize_session, get_session_status, get_session_metrics
- **Fix Generation**: Separate `FixGenerationHandler` with generate/cancel methods
- **Streaming**: Comprehensive streaming infrastructure via `StreamingHandler`

### JSON-RPC Protocol Implementation:
- **Framework**: tower-lsp (version 0.20) for LSP infrastructure
- **Transport**: stdio/Unix socket with enterprise-safe IPC
- **Routing**: All custom methods via `execute_command` pattern
- **Error Handling**: Comprehensive error codes (-32001 to -32015)
- **Streaming**: Background tokio tasks with mpsc channels for real-time notifications

### Integration Points for Refactor:
- **PRESERVE**: Transport layer (stdio/Unix socket), JSON-RPC protocol, streaming infrastructure
- **REFACTOR**: Handler implementations, session management, API endpoint routing
- **ADD**: Three new method constants, Goose agent integration, session delegation

### Test Infrastructure:
- **Integration Tests**: `tests/integration/` with lifecycle, fix_workflow, goose_integration, streaming
- **Contract Tests**: `tests/contract/` for JSON-RPC validation
- **Coverage Areas**: Session lifecycle, streaming notifications, error handling

This research provides the foundation for confident implementation planning with clear decisions, concrete examples, and risk mitigation strategies.
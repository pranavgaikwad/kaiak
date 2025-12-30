# Implementation Plan: Kaiak Client Implementation

**Branch**: `004-kaiak-client` | **Date**: 2025-12-27 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/004-kaiak-client/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Implement a comprehensive client-server CLI architecture for Kaiak, splitting the current monolithic server into separate `serve` and client commands. The client will connect to servers via Unix sockets, maintain persistent connection state, and provide all existing server procedures (generate_fix, configure, delete_session) through client commands. This includes unifying the configuration system and restructuring the CLI to support both server management and remote client operations.

## Technical Context

**Language/Version**: Rust 1.75+ (stable toolchain, consistent with existing codebase)
**Primary Dependencies**: clap 4.x (CLI), tower-lsp (JSON-RPC), tokio (async runtime), goose (git dependency), serde (serialization)
**Storage**: File-based state persistence (~/.kaiak/client.state, ~/.kaiak/server.conf), Goose SQLite session management
**Testing**: cargo test with integration tests priority (tests/test_client.rs for comprehensive CLI coverage)
**Target Platform**: Linux/macOS/Windows cross-platform CLI tool
**Project Type**: Single project with client-server architecture
**Performance Goals**: <30s server startup + client connection, <2s client command execution, <500ms latency for common operations
**Constraints**: Enterprise-safe IPC (Unix sockets, stdio only - no network ports), file system permissions required for socket creation
**Scale/Scope**: Single-user development tool, 10+ concurrent sessions supported, configuration hierarchy with 4-level precedence

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### ✅ User Experience First
- **Progress Indicators**: Client commands provide real-time feedback for operations >2s
- **Error Messages**: Spec requires clear error messages with retry suggestions and connection validation guidance (FR-010)
- **Comprehensive Logging**: Global `--log-level` and `--log-file` options supported across all commands

### ✅ Testing Integrity (NON-NEGOTIABLE)
- **Integration Test Priority**: Comprehensive integration test required at `tests/test_client.rs` covering all CLI code paths
- **E2E Coverage**: Each user story (P1-P4) includes independent testability with acceptance scenarios
- **Critical Journey Tests**: Server management, client connection, remote procedures all covered with test scenarios

### ✅ Enterprise-Safe Communication
- **IPC Compliance**: Uses Unix domain sockets and stdio transport only - no network ports
- **Socket-based Architecture**: Primary production deployment pattern uses socket communication
- **Security Alignment**: Aligns with corporate firewall and security policies

### ✅ Code Quality Standards
- **Rust Best Practices**: Follows existing codebase patterns with stable toolchain (Rust 1.75+)
- **Existing Infrastructure**: Leverages established dependencies (clap, tower-lsp, tokio, goose)
- **Automated Standards**: Will use existing CI pipeline for linting, formatting, and static analysis

### ✅ Continuous Integration
- **CI Compliance**: Will integrate with existing GitHub Actions pipeline
- **Quality Gates**: Code quality checks, testing, and security scanning required before merge

### ✅ Progressive Development
- **Incremental Value**: Feature broken into prioritized user stories (P1: Server Management → P2: Client Connection → P3: Remote Procedures → P4: Global CLI)
- **Independent Testing**: Each priority level delivers standalone value and can be tested independently
- **Measurable Outcomes**: Success criteria defined with specific metrics and timelines

**GATE STATUS: ✅ PASSED** - No constitutional violations detected.

## Project Structure

### Documentation (this feature)

```text
specs/[###-feature]/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
src/
├── models/
│   ├── configuration.rs      # Unified configuration structures (see data-model.md):
│   │                        #   - ServerConfig: Complete server startup config
│   │                        #   - InitConfig: Server initialization subset
│   │                        #   - BaseConfig: Runtime server config subset
│   │                        #   - AgentConfig: Session-specific agent config, the existing AgentConfiguration will be replaced with this
│   ├── client.rs            # NEW: Client connection state and management
│   ├── errors.rs            # Error types and JSON-RPC error codes
│   └── mod.rs
├── server/
│   ├── transport.rs         # JSON-RPC server transport (stdio, socket)
│   ├── handlers.rs          # Request handlers (configure, generate_fix, delete_session)
│   └── mod.rs
├── agents/
│   ├── manager.rs           # Agent lifecycle and session management
│   ├── session_wrapper.rs   # Session state with unified AgentConfig
│   └── mod.rs
├── cli/
│   ├── commands/
│   │   ├── serve.rs         # UPDATED: Server management command
│   │   ├── connect.rs       # NEW: Client connection command
│   │   ├── disconnect.rs    # NEW: Client disconnect command
│   │   ├── client_ops.rs    # NEW: Client procedure execution (generate_fix, configure, delete_session)
│   │   ├── doctor.rs        # Existing: Health checks
│   │   └── mod.rs
│   ├── parser.rs            # CLI argument parsing and validation
│   └── mod.rs
├── logging.rs               # MOVED: Logging setup (from config/logging.rs)
└── main.rs                  # UPDATED: New CLI structure with serve/client commands

tests/
├── test_client.rs           # NEW: Comprehensive client integration tests
├── test_configure_endpoint.rs    # EXISTING: Configure endpoint tests
├── test_delete_session_endpoint.rs # EXISTING: Delete session endpoint tests
├── test_generate_fix_endpoint.rs   # EXISTING: Generate fix endpoint tests
├── streaming.rs             # EXISTING: Streaming tests
├── benchmarks.rs            # EXISTING: Performance benchmarks
├── common/                  # EXISTING: Common test utilities
└── data/                    # EXISTING: Test data files

# REMOVED DIRECTORIES:
# config/ - All logic consolidated into models/configuration.rs and logging.rs
```

**Structure Decision**: Single project architecture maintained with enhanced CLI structure. The existing modular design is preserved while adding client-specific modules and consolidating configuration management. Key changes:

- **Configuration Unification**: Server startup config (ServerConfig) and runtime config (BaseConfig) unified in single module per data-model.md, while removing scattered config files. Session-specific AgentConfig remains separate for per-session customization.
- **Client Architecture**: New client modules for connection state and procedure execution
- **Enhanced CLI**: Restructured commands to support both server and client operations
- **Test Coverage**: Added `test_client.rs` for comprehensive client integration tests alongside existing endpoint tests

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| [e.g., 4th project] | [current need] | [why 3 projects insufficient] |
| [e.g., Repository pattern] | [specific problem] | [why direct DB access insufficient] |

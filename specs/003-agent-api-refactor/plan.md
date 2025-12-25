# Implementation Plan: Agent API Refactor for Goose Integration

**Branch**: `003-agent-api-refactor` | **Date**: 2025-12-24 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/003-agent-api-refactor/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Refactor Kaiak's agent API to properly integrate with the Goose AI framework by simplifying the API surface to three endpoints (configure, generate_fix, delete_session), delegating all session management to Goose's native SessionManager, and correctly initializing Goose agents with proper tool system integration. The refactor maintains existing JSON-RPC protocol and transport while eliminating custom session persistence logic.

## Technical Context

**Language/Version**: Rust 1.75+ (stable toolchain)
**Primary Dependencies**: Goose (git dependency), tower-lsp, tokio, serde, anyhow, tracing
**Storage**: Delegated to Goose's SQLite session management (no custom persistence)
**Testing**: cargo test, integration tests for Goose API integration
**Target Platform**: Linux server (primary), cross-platform compatibility
**Project Type**: Single Rust server project (agent refactor)
**Performance Goals**: <100ms event streaming, 30% server startup improvement, 20% memory reduction
**Constraints**: Enterprise-safe IPC only (Unix sockets/stdio), maintain existing JSON-RPC protocol
**Scale/Scope**: Support 10+ concurrent sessions, handle migration incident processing workflows

**Key Technical Research Completed**:
- ✅ Goose SessionManager API patterns documented in research.md with concrete examples
- ✅ Goose Agent initialization patterns with tool system integration defined
- ✅ Goose SessionConfig structure and configuration options mapped to data-model.md
- ✅ Goose AgentEvent streaming patterns mapped to contracts/jsonrpc-api.md
- ✅ Custom tool integration patterns via MCP extensions documented
- ✅ Permission enforcement mechanisms mapped to Goose's tool system
- ✅ Current Kaiak test infrastructure analysis complete - removal plan in tasks.md
- ✅ Existing Kaiak models updated per data-model.md specifications
- ✅ JSON-RPC message format compatibility confirmed in contracts/jsonrpc-api.md

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

✅ **I. User Experience First**: Maintains existing JSON-RPC transport and streaming for progress indicators. Error handling through Goose's native mechanisms.

✅ **II. Testing Integrity (NON-NEGOTIABLE)**: Plan includes comprehensive test refactoring - removing outdated endpoint tests, adding Goose integration tests, maintaining >90% coverage.

✅ **III. Enterprise-Safe Communication**: Preserves existing Unix socket/stdio IPC. No network ports introduced.

✅ **IV. Code Quality Standards**: Rust implementation with proper error handling via Result types. Goose integration follows idiomatic Rust patterns.

✅ **V. Continuous Integration**: Refactor maintains existing CI pipeline compatibility. Test updates ensure automated validation continues.

✅ **VI. Progressive Development**: Feature broken into clear phases - API simplification, session delegation, agent initialization, event streaming.

**Gate Status**: ✅ **PASSED** - No constitutional violations. Phase 0 research complete.

**Re-evaluation Post-Design**: ✅ **PASSED** - Design artifacts maintain constitutional compliance:
- User Experience: Maintained JSON-RPC streaming and error handling
- Testing Integrity: Comprehensive test refactoring with >90% coverage target
- Enterprise-Safe Communication: Preserved stdio/Unix socket transport
- Code Quality: Rust implementation with proper Goose integration patterns
- Continuous Integration: Compatible with existing CI pipeline
- Progressive Development: Clear phased implementation approach

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
├── server.rs              # JSON-RPC server (existing, modifications needed)
├── agent_manager.rs       # Goose agent management (major refactor)
├── session.rs             # Session handling (remove custom logic, wrap Goose)
├── models/
│   ├── configuration.rs   # Agent configuration models (update for nested JSON)
│   ├── incidents.rs       # Migration incident models (existing)
│   └── events.rs          # Agent event models (update for Goose events)
├── handlers/
│   ├── configure.rs       # configure() endpoint handler (new/refactored)
│   ├── generate_fix.rs    # generate_fix() endpoint handler (refactored)
│   └── delete_session.rs  # delete_session() endpoint handler (new)
└── lib.rs                 # Module exports and common utilities

tests/
├── integration/
│   ├── goose_session.rs   # Goose SessionManager integration tests (new)
│   ├── agent_lifecycle.rs # Agent initialization and tool tests (new)
│   └── api_endpoints.rs   # Three-endpoint API tests (updated)
├── unit/
│   ├── models.rs          # Model validation tests (updated)
│   └── handlers.rs        # Handler logic tests (updated)
└── removed/               # Deprecated tests to be deleted
    ├── session_crud.rs    # Custom session management tests (remove)
    └── old_endpoints.rs   # Removed endpoint tests (remove)
```

**Structure Decision**: Single Rust project structure maintained. Focus on refactoring existing modules rather than creating new project structure. The main changes are in agent_manager.rs (Goose integration), handlers/ (three-endpoint API), and comprehensive test updates.

## Complexity Tracking

> **No constitutional violations identified - section not applicable**

## Phase Completion Status

### Phase 0: Research & Planning ✅ COMPLETED
- ✅ Goose SessionManager API research complete
- ✅ Goose Agent initialization patterns documented
- ✅ Current Kaiak codebase structure analyzed
- ✅ JSON-RPC protocol compatibility confirmed
- ✅ Research findings consolidated in research.md

### Phase 1: Design & Contracts ✅ COMPLETED
- ✅ Data models created for three-endpoint API
- ✅ JSON-RPC API contracts defined
- ✅ Event streaming patterns specified
- ✅ Quick start guide developed
- ✅ Agent context updated with Goose technologies

### Phase 2: Implementation Tasks 🔄 READY FOR /speckit.tasks
The plan is complete and ready for task generation. All research has been conducted, design artifacts created, and architectural decisions documented with concrete implementation guidance.

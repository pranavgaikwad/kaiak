# Implementation Plan: Kaiak Migration Server Skeleton

**Branch**: `001-kaiak-skeleton` | **Date**: 2025-12-22 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/001-kaiak-skeleton/spec.md`

## Summary

Build a standalone Rust server (Kaiak) that runs the Goose AI agent for code migration use cases. The server accepts fix generation requests from IDE extensions, manages Goose agent lifecycle, and streams real-time progress updates back to clients using LSP-style JSON-RPC over enterprise-safe IPC channels. Core focus on controlled migration workflows with file modification approval requirements.

## Technical Context

**Language/Version**: Rust 1.75+ (stable toolchain)
**Primary Dependencies**: Goose (github.com/block/goose), JSON-RPC compatible library, tokio async runtime
**Storage**: File-based session state, in-memory caching for active sessions
**Testing**: cargo test with focus on integration/e2e tests over unit tests
**Target Platform**: Cross-platform (Linux, macOS, Windows) with optional WASM distribution
**Project Type**: single (standalone server binary)
**Performance Goals**: <2s request processing, <500ms streaming latency, 10+ concurrent sessions
**Constraints**: Enterprise-safe IPC only (no network ports), minimal dependencies, LSP message compatibility
**Scale/Scope**: Single-user sessions, typical codebases up to 10k files, 5min processing time

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Initial Check (Pre-Research)
**✅ PASSED** - All constitutional requirements validated

### Post-Design Re-Evaluation

**✅ I. User Experience First**
- Real-time progress streaming via JSON-RPC notifications
- Structured error handling with actionable messages and recovery suggestions
- Comprehensive logging with structured formats (ERROR/WARN/INFO/DEBUG levels)
- Progress indicators with percentage completion and phase descriptions
- User interactions with clear prompts and timeout mechanisms

**✅ II. Testing Integrity (NON-NEGOTIABLE)**
- Test pyramid prioritizes E2E/integration tests over unit tests
- Critical user journeys covered: fix generation, streaming, user interactions, agent lifecycle
- Contract tests validate JSON-RPC protocol compliance and Goose API integration
- TDD approach with tests written and verified to fail before implementation

**✅ III. Enterprise-Safe Communication**
- Primary: stdio transport (LSP-compatible, no network exposure)
- Fallback: Unix domain sockets with proper file permissions (0600)
- JSON-RPC 2.0 with Content-Length headers for message framing
- Bidirectional streaming for real-time communication
- Process isolation provides enterprise security
- Connection management with automatic reconnection capabilities

**✅ IV. Code Quality Standards**
- Rust 1.75+ with stable toolchain and best practices
- Minimal dependencies: tower-lsp, tokio, goose, serde, anyhow
- Each dependency justified (JSON-RPC handling, async runtime, AI agent, serialization, error handling)
- Automated linting (cargo clippy) and formatting (cargo fmt)
- Self-documenting code with comments only for complex business logic

**✅ V. Continuous Integration**
- GitHub Actions CI pipeline with PR gating
- Local scripts mirror CI: scripts/ci.sh, scripts/test.sh, scripts/lint.sh
- Comprehensive testing, linting, security scanning, code quality checks
- Multi-platform builds (Linux, macOS, Windows)
- Automated dependency auditing and vulnerability scanning

**✅ VI. Progressive Development**
- User stories prioritized for incremental delivery (P1: Basic processing → P4: Lifecycle management)
- Each user story independently testable with acceptance criteria
- MVP delivers core value (fix generation and streaming)
- Modular architecture supports independent feature development

### Design-Specific Validations

**✅ IPC Architecture Requirements** (Constitution III)
- Message serialization/deserialization via serde (transparent handling)
- Streaming support through JSON-RPC notifications for progress updates
- Heartbeat and reconnection via process supervision (stdio) or socket keepalive
- Message format versioning through API version embedding in method names
- Error propagation preserves context via anyhow error chain and JSON-RPC error codes

**✅ Rust Implementation Requirements** (Constitution - Development Standards)
- Result types for error handling, no panics in production code
- Justified minimal dependencies with clear separation of concerns
- Performance-critical streaming operations designed with benchmarks in mind
- Memory safety verified through Rust compiler guarantees

**✅ Progress Transparency** (Constitution - Quality Assurance)
- Real-time feedback for operations exceeding 2s via progress notifications
- Percentage completion indicators where possible (phase-based progress)
- Structured logging throughout system with consistent formats
- No silent failures - all operations report status

**✅ Error Handling Standards** (Constitution - Quality Assurance)
- Error messages include context, suggested actions, and error codes
- JSON-RPC error responses for client communication with technical details
- User-facing errors translated to plain language while preserving context in logs
- Recovery suggestions provided through error message content

**GATE STATUS: ✅ PASSED** - Design maintains constitutional compliance with enhanced validation

## Project Structure

### Documentation (this feature)

```text
specs/001-kaiak-skeleton/
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
├── main.rs              # Entry point and CLI argument parsing
├── server/              # Core server implementation
│   ├── mod.rs
│   ├── jsonrpc.rs       # JSON-RPC protocol handling
│   └── transport.rs     # IPC transport layer (sockets/stdio)
├── goose/               # Goose agent integration
│   ├── mod.rs
│   ├── agent.rs         # Agent lifecycle management
│   ├── session.rs       # Session state management
│   └── prompts.rs       # Migration-specific prompt templates
├── models/              # Data models and entities
│   ├── mod.rs
│   ├── request.rs       # Fix generation request structures
│   ├── incident.rs      # Incident data models
│   └── messages.rs      # Stream message types
├── handlers/            # Request processing logic
│   ├── mod.rs
│   ├── fix_generation.rs # Core fix generation handler
│   └── lifecycle.rs     # Agent lifecycle operations
└── config/              # Configuration management
    ├── mod.rs
    └── settings.rs      # Environment-aware configuration

tests/
├── integration/         # End-to-end integration tests
│   ├── mod.rs
│   ├── fix_workflow.rs  # Complete fix generation workflow
│   ├── streaming.rs     # Real-time progress streaming
│   └── approval.rs      # File modification approval flow
├── contract/            # API contract tests
│   ├── mod.rs
│   ├── jsonrpc.rs       # JSON-RPC protocol compliance
│   └── goose_api.rs     # Goose agent API integration
└── unit/                # Focused unit tests for complex logic
    ├── mod.rs
    ├── prompts.rs       # Prompt generation logic
    └── session.rs       # Session management utilities

Cargo.toml               # Rust project configuration
.github/workflows/       # GitHub Actions CI pipeline
scripts/                 # Local development scripts
    ├── ci.sh            # Local CI script mirroring GitHub Actions
    ├── test.sh          # Comprehensive test runner
    └── lint.sh          # Code quality checks
```

**Structure Decision**: Single project structure chosen as this is a standalone server binary. The modular organization separates concerns clearly: server infrastructure, Goose integration, data models, request handlers, and configuration. Testing structure prioritizes integration tests with contract tests for API compliance and minimal unit tests for complex business logic only.

## Complexity Tracking

> **No constitutional violations detected - section intentionally empty**
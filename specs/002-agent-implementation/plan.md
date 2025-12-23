# Implementation Plan: Agent Implementation

**Branch**: `002-agent-implementation` | **Date**: 2025-12-23 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/002-agent-implementation/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Integrate Goose Agent framework via direct Rust library integration to process incidents and provide real-time streaming responses. Primary requirement is to establish end-to-end agent processing capabilities with comprehensive integration testing while documenting Goose-to-IDE compatibility gaps for future enhancement planning. Technical approach uses direct Rust crate integration with Goose's native event stream for real-time updates.

## Technical Context

**Language/Version**: Rust 1.75+ (stable toolchain)
**Primary Dependencies**: goose (git dependency), tower-lsp, tokio, serde, anyhow, tracing
**Storage**: N/A (no data persistence beyond session management for this feature)
**Testing**: cargo test (integration tests prioritized over unit tests)
**Target Platform**: Linux server (standalone verification)
**Project Type**: Single Rust project (binary application)
**Performance Goals**: <30s incident processing, <500ms streaming latency, 95% test success rate
**Constraints**: Enterprise-safe IPC only (stdio, unix sockets), no network ports, mock-capable for CI/PR testing
**Scale/Scope**: Single incident processing, standalone agent verification, end-to-end test harness

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### I. User Experience First
✅ **COMPLIANT** - FR-004 ensures real-time streaming (<500ms latency), FR-006 requires actionable error messages, comprehensive logging via existing tracing infrastructure

### II. Testing Integrity (NON-NEGOTIABLE)
✅ **COMPLIANT** - FR-009 mandates comprehensive integration test, user stories designed for e2e testing, 95% success rate target, prioritizes integration over unit tests

### III. Enterprise-Safe Communication
✅ **COMPLIANT** - Direct Rust library integration avoids network communication, uses existing stdio/unix socket IPC, no network ports required

### IV. Code Quality Standards
✅ **COMPLIANT** - Rust 1.75+ stable toolchain, follows existing project patterns, automated linting/formatting via CI

### V. Continuous Integration
✅ **COMPLIANT** - Investigates model provider mocking for CI/PR testing, no manual testing gates, follows existing GitHub Actions pipeline

### VI. Progressive Development
✅ **COMPLIANT** - Three prioritized user stories (P1: Basic Processing, P2: Streaming, P3: Tool Calls), each independently testable and valuable

**GATE STATUS**: ✅ **PASSED** - No constitutional violations identified

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
├── goose/              # Goose integration module (EXISTING - ENHANCE)
│   ├── agent.rs        # ✅ Exists - needs actual Goose integration
│   ├── session.rs      # ✅ Exists - enhance for agent sessions
│   ├── prompts.rs      # ✅ Exists - incident-to-prompt formatting
│   └── monitoring.rs   # ✅ Exists - enhance for streaming events
├── models/             # ✅ Existing data models
│   ├── incident.rs     # ✅ Already defined
│   ├── session.rs      # ✅ Existing - enhance for agent sessions
│   └── messages.rs     # ✅ Existing - enhance for agent messages
├── handlers/           # ✅ Existing JSON-RPC handlers
│   ├── streaming.rs    # ✅ Exists - connect to Goose event stream
│   └── fix_generation.rs # ✅ Exists - integrate with Goose agent
├── server/             # ✅ Existing JSON-RPC server
├── config/             # ✅ Existing configuration management
└── main.rs

tests/
├── integration/        # ✅ Existing - ENHANCE PRIMARY FOCUS
│   ├── goose_integration.rs # ✅ Exists - implement comprehensive e2e test
│   ├── streaming.rs    # ✅ Exists - enhance for agent streaming
│   └── fixtures/       # To be added - sample incidents, test workspace
└── contract/           # ✅ Existing JSON-RPC contract tests
```

**Structure Decision**: Leveraging existing well-structured Kaiak foundation. The `src/goose/` module already exists with skeleton implementations that need actual Goose library integration. Focus will be on enhancing existing files rather than creating new modules, with primary emphasis on the `goose_integration.rs` test becoming the comprehensive end-to-end test required by FR-009.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

No constitutional violations identified - complexity tracking not required for this feature.

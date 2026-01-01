# kaiak Development Guidelines

Auto-generated from all feature plans. Last updated: 2025-12-22

## Active Technologies

**Core Stack**:
- Rust 1.75+ (stable toolchain)
- Goose AI agent (github.com/block/goose) - git dependency
- tokio (async runtime)
- tower-lsp (JSON-RPC communication)
- serde (JSON serialization)
- clap 4.x (CLI interface)
- anyhow, tracing (error handling and logging)

**Data Persistence**:
- Goose SQLite session management (agent sessions)
- File-based client state (~/.kaiak/client.state, ~/.kaiak/server.conf)
- No additional persistence for notifications (transient only)

## Project Structure

```text
src/
tests/
```

## Commands

cargo test [ONLY COMMANDS FOR ACTIVE TECHNOLOGIES][ONLY COMMANDS FOR ACTIVE TECHNOLOGIES] cargo clippy

## Code Style

Rust 1.75+ (stable toolchain): Follow standard conventions

## Recent Changes
- 005-client-notifications: Added Rust 1.75 (consistent with existing Kaiak codebase) + tokio (async runtime), serde (JSON serialization), existing Kaiak JSON-RPC infrastructure
- 004-kaiak-client: Added Rust 1.75+ (stable toolchain, consistent with existing codebase) + clap 4.x (CLI), tower-lsp (JSON-RPC), tokio (async runtime), goose (git dependency), serde (serialization)
- 003-agent-api-refactor: Added Rust 1.75+ (stable toolchain) + Goose (git dependency), tower-lsp, tokio, serde, anyhow, tracing


<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->

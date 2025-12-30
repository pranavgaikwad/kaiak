# kaiak Development Guidelines

Auto-generated from all feature plans. Last updated: 2025-12-22

## Active Technologies
- Rust 1.75+ (stable toolchain) + goose (git dependency), tower-lsp, tokio, serde, anyhow, tracing (002-agent-implementation)
- N/A (no data persistence beyond session management for this feature) (002-agent-implementation)
- Rust 1.75+ (stable toolchain) + Goose (git dependency), tower-lsp, tokio, serde, anyhow, tracing (003-agent-api-refactor)
- Delegated to Goose's SQLite session management (no custom persistence) (003-agent-api-refactor)
- Rust 1.75+ (stable toolchain, consistent with existing codebase) + clap 4.x (CLI), tower-lsp (JSON-RPC), tokio (async runtime), goose (git dependency), serde (serialization) (004-kaiak-client)
- File-based state persistence (~/.kaiak/client.state, ~/.kaiak/server.conf), Goose SQLite session management (004-kaiak-client)

- Rust 1.75+ (stable toolchain) + Goose (github.com/block/goose), JSON-RPC compatible library, tokio async runtime (001-kaiak-skeleton)

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
- 004-kaiak-client: Added Rust 1.75+ (stable toolchain, consistent with existing codebase) + clap 4.x (CLI), tower-lsp (JSON-RPC), tokio (async runtime), goose (git dependency), serde (serialization)
- 003-agent-api-refactor: Added Rust 1.75+ (stable toolchain) + Goose (git dependency), tower-lsp, tokio, serde, anyhow, tracing
- 002-agent-implementation: Added Rust 1.75+ (stable toolchain) + goose (git dependency), tower-lsp, tokio, serde, anyhow, tracing


<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->

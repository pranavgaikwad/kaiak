# Kaiak Developer Guide

Complete guide for developers working on the Kaiak Migration Server codebase.

## Table of Contents

1. [Development Environment Setup](#development-environment-setup)
2. [Project Architecture](#project-architecture)
3. [Code Organization](#code-organization)
4. [Building and Testing](#building-and-testing)
5. [Adding New Features](#adding-new-features)
6. [Debugging](#debugging)
7. [Performance Optimization](#performance-optimization)
8. [Security Considerations](#security-considerations)
9. [Contributing Guidelines](#contributing-guidelines)

## Development Environment Setup

### Prerequisites

- **Rust 1.75+** with stable toolchain
- **Git** for version control
- **IDE** with Rust support (VSCode with rust-analyzer recommended)

### Initial Setup

1. **Install Rust**:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

2. **Clone Repository**:
```bash
git clone https://github.com/pranavgaikwad/kaiak.git
cd kaiak
```

3. **Install Development Tools**:
```bash
# Install additional components
rustup component add clippy rustfmt

# Install development dependencies
cargo install cargo-watch cargo-audit cargo-tarpaulin
```

4. **Set Up Git Hooks** (optional):
```bash
# Pre-commit hook for code quality
cat > .git/hooks/pre-commit << 'EOF'
#!/bin/sh
cargo fmt --check || exit 1
cargo clippy -- -D warnings || exit 1
cargo test || exit 1
EOF
chmod +x .git/hooks/pre-commit
```

### Environment Configuration

Set up development environment variables:

```bash
# Add to ~/.bashrc or ~/.zshrc
export RUST_LOG=kaiak=debug
export KAIAK_DEV_MODE=true

# For testing with AI providers (optional)
export OPENAI_API_KEY="sk-test-key"  # Use test keys for development
```

## Project Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    IDE Clients                               │
│  (VSCode, IntelliJ, Vim, Emacs)                             │
└─────────────────┬───────────────────────────────────────────┘
                  │ JSON-RPC over LSP transport
                  ▼
┌─────────────────────────────────────────────────────────────┐
│                 Transport Layer                              │
│  • stdio (primary)  • Unix sockets (fallback)              │
└─────────────────┬───────────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────────┐
│              JSON-RPC Protocol Layer                        │
│  • Method routing  • Request validation  • Response format  │
└─────────────────┬───────────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────────┐
│                Handler Layer                                 │
│  • FixGenerationHandler  • LifecycleHandler                │
│  • StreamingHandler      • InteractionHandler              │
└─────────────┬───────────────────┬───────────────────────────┘
              │                   │
              ▼                   ▼
┌─────────────────────┐ ┌─────────────────────┐
│   Session Manager   │ │   Security Layer    │
│  • LRU Cache        │ │  • File Validation  │
│  • Lifecycle Mgmt   │ │  • Approval Flow    │
│  • Performance Opt  │ │  • Path Sanitization│
└─────────┬───────────┘ └─────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────┐
│                 Goose Integration                            │
│  • Agent Manager  • Tool Execution  • Context Management   │
└─────────────────────────────────────────────────────────────┘
```

### Core Components

#### 1. Server Module (`src/server/`)

- **`transport.rs`**: IPC transport implementations (stdio, sockets)
- **`jsonrpc.rs`**: JSON-RPC 2.0 protocol handling
- **`server.rs`**: Main server orchestration and LSP integration

#### 2. Handlers Module (`src/handlers/`)

- **`fix_generation.rs`**: Core fix generation workflow
- **`lifecycle.rs`**: Session and agent lifecycle management
- **`streaming.rs`**: Real-time progress and message streaming
- **`interactions.rs`**: User interaction and approval handling
- **`modifications.rs`**: File modification proposal management

#### 3. Goose Module (`src/goose/`)

- **`agent.rs`**: Goose agent lifecycle and communication
- **`session.rs`**: Session management with performance optimizations
- **`monitoring.rs`**: Performance monitoring and metrics
- **`resources.rs`**: Resource management and cleanup

#### 4. Models Module (`src/models/`)

- **`session.rs`**: Session and configuration data structures
- **`request.rs`**: Fix generation request models
- **`incident.rs`**: Code incident and issue representations
- **`messages.rs`**: Stream message types and content
- **`proposal.rs`**: File modification proposal structures
- **`interaction.rs`**: User interaction data models

#### 5. Config Module (`src/config/`)

- **`settings.rs`**: Configuration structures and loading
- **`security.rs`**: Security policies and validation
- **`validation.rs`**: Comprehensive configuration validation

## Code Organization

### Module Structure

```rust
// src/lib.rs - Main library exports and error types
pub mod config;     // Configuration management
pub mod server;     // Server implementation
pub mod goose;      // Goose agent integration
pub mod models;     // Data models
pub mod handlers;   // Request handlers

// Common error type used throughout
pub type KaiakResult<T> = Result<T, KaiakError>;
```

### Naming Conventions

- **Modules**: `snake_case` (e.g., `fix_generation.rs`)
- **Structs**: `PascalCase` (e.g., `SessionManager`)
- **Functions**: `snake_case` (e.g., `create_session`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `MAX_SESSIONS`)
- **Type Aliases**: `PascalCase` (e.g., `KaiakResult`)

### Error Handling

Consistent error handling pattern throughout:

```rust
use anyhow::Result;
use crate::{KaiakError, KaiakResult};

// Use anyhow::Result for internal operations
async fn internal_operation() -> Result<String> {
    // Internal implementation
    Ok("success".to_string())
}

// Use KaiakResult for public APIs
pub async fn public_api() -> KaiakResult<String> {
    let result = internal_operation()
        .await
        .map_err(|e| KaiakError::internal(format!("Operation failed: {}", e)))?;

    Ok(result)
}
```

### Async Patterns

Consistent async/await usage:

```rust
use tokio::sync::{RwLock, Mutex};
use std::sync::Arc;

// Shared state pattern
type SharedState<T> = Arc<RwLock<T>>;

impl SomeHandler {
    // Async methods for I/O operations
    pub async fn handle_request(&self, req: Request) -> KaiakResult<Response> {
        // Read-heavy operations use read locks
        let state = self.state.read().await;

        // Write operations use write locks
        drop(state);
        let mut state = self.state.write().await;
        state.update(req);

        Ok(Response::success())
    }
}
```

## Building and Testing

### Build Commands

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Check without building
cargo check

# Build specific package
cargo build -p kaiak

# Build with all features
cargo build --all-features
```

### Testing

#### Unit Tests

```bash
# Run all unit tests
cargo test --lib

# Run specific module tests
cargo test --lib models::session

# Run with output
cargo test --lib -- --nocapture

# Run specific test
cargo test --lib test_session_creation
```

#### Integration Tests

```bash
# Run integration tests
cargo test --test integration

# Run specific integration test file
cargo test --test quickstart_validation

# Run benchmarks
cargo test --test benchmarks --release
```

#### Test Organization

```rust
// Unit tests in same file as implementation
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_creation() {
        // Test implementation
    }
}

// Integration tests in tests/ directory
// tests/integration/session_management.rs
use kaiak::*;

#[tokio::test]
async fn test_complete_session_workflow() {
    // End-to-end test
}
```

### Development Workflow

```bash
# Watch for changes and run tests
cargo watch -x check -x test -x run

# Format code
cargo fmt

# Run linting
cargo clippy -- -D warnings

# Security audit
cargo audit

# Code coverage
cargo tarpaulin --out Html

# Documentation
cargo doc --open
```

## Adding New Features

### 1. Feature Planning

Before implementing, consider:

- **User Story**: What problem does this solve?
- **API Design**: How will clients interact with this feature?
- **Backwards Compatibility**: Does this break existing APIs?
- **Performance Impact**: How does this affect system performance?
- **Security Implications**: Are there any security concerns?

### 2. Implementation Steps

#### Step 1: Add Tests First (TDD)

```rust
// tests/integration/new_feature.rs
#[tokio::test]
async fn test_new_feature_workflow() {
    // Define expected behavior
    let result = new_feature_handler.handle_request(request).await;
    assert!(result.is_ok());
    // More assertions...
}
```

#### Step 2: Define Data Models

```rust
// src/models/new_feature.rs
use serde::{Deserialize, Serialize};
use crate::models::{Id, Timestamp, Metadata};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewFeatureRequest {
    pub id: Id,
    pub session_id: Id,
    pub parameters: NewFeatureParameters,
    pub created_at: Timestamp,
    #[serde(default)]
    pub metadata: Metadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewFeatureParameters {
    // Feature-specific parameters
}
```

#### Step 3: Implement Handler

```rust
// src/handlers/new_feature.rs
use anyhow::Result;
use crate::models::NewFeatureRequest;
use crate::{KaiakResult, KaiakError};

pub struct NewFeatureHandler {
    // Handler state
}

impl NewFeatureHandler {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn handle_request(&self, request: NewFeatureRequest) -> KaiakResult<String> {
        // Implementation
        Ok("success".to_string())
    }
}
```

#### Step 4: Add JSON-RPC Methods

```rust
// src/server/jsonrpc.rs
pub mod methods {
    pub const NEW_FEATURE_METHOD: &str = "kaiak/feature/action";
}

// In server implementation
async fn handle_new_feature_request(&self, params: Value, id: RequestId) -> JsonRpcResult<Value> {
    let request: NewFeatureRequest = serde_json::from_value(params)
        .map_err(|e| create_error(error_codes::INVALID_PARAMS, &e.to_string(), None))?;

    let handler = self.new_feature_handler.read().await;
    let result = handler.as_ref()
        .ok_or_else(|| create_error(error_codes::INTERNAL_ERROR, "Handler not initialized", None))?
        .handle_request(request)
        .await
        .map_err(|e| create_error(e.error_code(), &e.to_string(), None))?;

    Ok(serde_json::to_value(result).unwrap())
}
```

#### Step 5: Update Configuration (if needed)

```rust
// src/config/settings.rs - Add new config section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewFeatureConfig {
    pub enabled: bool,
    pub max_requests: u32,
}

// Add to main Settings struct
pub struct Settings {
    // ... existing fields
    pub new_feature: NewFeatureConfig,
}
```

#### Step 6: Add Documentation

```rust
//! New Feature Module
//!
//! This module implements the new feature functionality for Kaiak.
//!
//! # Example Usage
//!
//! ```rust
//! use kaiak::handlers::NewFeatureHandler;
//!
//! let handler = NewFeatureHandler::new();
//! let result = handler.handle_request(request).await?;
//! ```

/// Handler for new feature requests
pub struct NewFeatureHandler {
    // ... implementation
}
```

### 3. Feature Integration Checklist

- [ ] Tests written and passing
- [ ] Data models defined
- [ ] Handler implementation complete
- [ ] JSON-RPC methods added
- [ ] Configuration updated (if needed)
- [ ] Documentation added
- [ ] Integration tests pass
- [ ] Performance impact assessed
- [ ] Security review completed

## Debugging

### Logging

Structured logging throughout the application:

```rust
use tracing::{info, debug, warn, error, instrument};

#[instrument]
pub async fn process_request(request: &Request) -> KaiakResult<Response> {
    debug!("Processing request: {}", request.id);

    match validate_request(request) {
        Ok(_) => info!("Request validation passed"),
        Err(e) => {
            warn!("Request validation failed: {}", e);
            return Err(KaiakError::invalid_request(e.to_string()));
        }
    }

    // Process request...
    info!(request_id = %request.id, "Request processed successfully");
    Ok(response)
}
```

### Debug Configuration

```bash
# Environment variables for debugging
export RUST_LOG="kaiak=debug,tower_lsp=info"
export KAIAK_TRACE_RPC=true
export KAIAK_PROFILE=true

# Run with debugging enabled
cargo run -- serve --stdio
```

### Development Tools

#### Using rust-analyzer

Configure in VSCode settings:

```json
{
    "rust-analyzer.cargo.features": "all",
    "rust-analyzer.checkOnSave.command": "clippy",
    "rust-analyzer.procMacro.enable": true
}
```

#### Using GDB/LLDB

```bash
# Build with debug symbols
cargo build

# Run with debugger
rust-gdb target/debug/kaiak
# or
rust-lldb target/debug/kaiak
```

## Performance Optimization

### Profiling

```bash
# Install profiling tools
cargo install flamegraph

# Generate flame graph
cargo flamegraph --bin kaiak -- serve --stdio

# Benchmark with criterion
cargo bench
```

### Memory Management

#### Efficient Data Structures

```rust
// Use appropriate data structures
use std::collections::HashMap;
use lru::LruCache;

// LRU cache for frequently accessed data
type SessionCache = LruCache<String, Arc<Session>>;

// Efficient string handling
use std::sync::Arc;
type SharedString = Arc<str>;
```

#### Async Performance

```rust
// Use buffered channels for high-throughput scenarios
use tokio::sync::mpsc;

let (tx, mut rx) = mpsc::channel(1000);  // Buffered channel

// Batch operations when possible
let batch_size = 100;
let mut batch = Vec::with_capacity(batch_size);

while let Some(item) = rx.recv().await {
    batch.push(item);

    if batch.len() >= batch_size {
        process_batch(batch.drain(..).collect()).await;
    }
}
```

### Monitoring

```rust
// Add metrics to critical paths
use std::time::{Duration, Instant};

async fn critical_operation() -> KaiakResult<()> {
    let start = Instant::now();

    // Perform operation
    let result = do_work().await;

    let duration = start.elapsed();
    if duration > Duration::from_millis(100) {
        warn!("Slow operation detected: {:?}", duration);
    }

    result
}
```

## Security Considerations

### Input Validation

```rust
use crate::config::security::SecurityConfig;

impl SecurityConfig {
    pub fn validate_file_path(&self, path: &str, workspace: &str) -> KaiakResult<String> {
        // Prevent directory traversal
        let normalized = path.replace("..", "");

        // Ensure within workspace
        let full_path = Path::new(workspace).join(&normalized);
        let canonical = full_path.canonicalize()
            .map_err(|e| KaiakError::invalid_workspace_path(format!("Invalid path: {}", e)))?;

        if !canonical.starts_with(workspace) {
            return Err(KaiakError::invalid_workspace_path("Path outside workspace"));
        }

        Ok(normalized)
    }
}
```

### Safe Async Operations

```rust
use tokio::time::{timeout, Duration};

// Always use timeouts for external operations
async fn safe_ai_request(request: AiRequest) -> KaiakResult<AiResponse> {
    let response = timeout(
        Duration::from_secs(300),  // 5-minute timeout
        make_ai_request(request)
    ).await
    .map_err(|_| KaiakError::timeout("AI request timed out"))?;

    response
}
```

### Resource Limits

```rust
// Enforce resource limits
const MAX_CONCURRENT_REQUESTS: usize = 10;
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;  // 10MB

async fn validate_request_limits(&self) -> KaiakResult<()> {
    if self.active_requests.len() >= MAX_CONCURRENT_REQUESTS {
        return Err(KaiakError::resource_exhausted("Too many concurrent requests"));
    }

    Ok(())
}
```

## Contributing Guidelines

### Code Style

Follow Rust conventions and project patterns:

```rust
// Use explicit error handling
match operation() {
    Ok(result) => process(result),
    Err(e) => {
        error!("Operation failed: {}", e);
        return Err(KaiakError::from(e));
    }
}

// Prefer owned types in public APIs
pub async fn public_method(&self, data: String) -> KaiakResult<String> {
    // Implementation
}

// Use borrowed types for internal operations
async fn internal_helper(&self, data: &str) -> Result<&str> {
    // Implementation
}
```

### Pull Request Process

1. **Create Feature Branch**: `git checkout -b feature/description`
2. **Implement Changes**: Follow TDD approach
3. **Run Tests**: `cargo test && cargo clippy && cargo fmt --check`
4. **Update Documentation**: Add/update relevant documentation
5. **Submit PR**: Include description, testing notes, and breaking changes
6. **Code Review**: Address feedback and update as needed
7. **Merge**: Squash commits and merge to main

### Commit Message Format

```
type(scope): brief description

Detailed explanation of what this change does and why it was needed.

Fixes #123
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

### Testing Requirements

- **Unit tests** for all new functionality
- **Integration tests** for workflows
- **Benchmarks** for performance-critical code
- **Documentation examples** that compile and run

### Documentation Requirements

- **API documentation** for all public items
- **Module documentation** explaining purpose and usage
- **Example code** for complex functionality
- **Update user guide** for user-facing features

This completes the developer documentation. The codebase is well-structured for collaborative development with clear patterns, comprehensive testing, and security considerations built in.
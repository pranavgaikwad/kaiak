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

## Project Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Clients                                 │
│  • IDE Extensions (stdio)  • CLI Client (Unix socket)       │
└─────────────────┬───────────────────────────────────────────┘
                  │ JSON-RPC 2.0 over LSP transport
                  │ (bidirectional notifications)
                  ▼
┌─────────────────────────────────────────────────────────────┐
│                 Transport Layer                              │
│  • StdioTransport  • IpcTransport  • IpcServerTransport     │
└─────────────────┬───────────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────────┐
│              JSON-RPC Server                                 │
│  • Method routing  • Concurrent streaming (tokio::select!)  │
│  • Bidirectional notifications  • Error codes               │
└─────────────────┬───────────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────────┐
│                Handler Layer                                 │
│  • GenerateFixHandler (streaming, optional session_id)      │
│  • DeleteSessionHandler                                     │
└─────────────┬───────────────────────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────────────────────────┐
│              Agent Manager                                   │
│  • Session creation (optional IDs)  • Agent lifecycle       │
│  • Event streaming  • Goose SessionManager integration      │
└─────────────────┬───────────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────────┐
│                 Goose Integration                            │
│  • Goose Agent  • Tool Execution  • AI Model Integration    │
└─────────────────────────────────────────────────────────────┘
```

### Core Components

#### 1. JSON-RPC Module (`src/jsonrpc/`)

- **`protocol.rs`**: Core JSON-RPC 2.0 types (Request, Response, Notification, Error) - shared between client/server
- **`transport.rs`**: Transport trait and implementations (StdioTransport, IpcTransport, IpcServerTransport)
- **`server.rs`**: JSON-RPC server with concurrent streaming using `tokio::select!`
- **`methods.rs`**: Method constants and registration helpers
- **`core.rs`**: Kaiak-specific request/response wrappers

#### 2. Handlers Module (`src/handlers/`)

- **`generate_fix.rs`**: Core fix generation with concurrent streaming notifications (optional session_id)
- **`delete_session.rs`**: Session cleanup and resource management

#### 3. Agent Module (`src/agent/`)

- **`mod.rs`**: GooseAgentManager for agent lifecycle and session coordination
- **`session_wrapper.rs`**: Session management wrapper with optional session ID support (Goose generates if not provided)

#### 4. Models Module (`src/models/`)

- **`configuration.rs`**: ServerConfig, BaseConfig, AgentConfig with hierarchy support
- **`incidents.rs`**: MigrationIncident representations

#### 5. Client Module (`src/client/`)

- **`transport.rs`**: JSON-RPC client with unified `call()` method for requests and notification handling
- **`mod.rs`**: Client exports (JsonRpcClient, ConnectionState) + re-exports shared JSON-RPC types

#### 6. Server Module (`src/server/`)

- **`server.rs`**: High-level server startup and orchestration

## Code Organization

### Module Structure

```rust
// src/lib.rs - Main library exports and error types
pub mod jsonrpc;    // JSON-RPC protocol and server
pub mod server;     // High-level server orchestration
pub mod agent;      // Goose agent integration
pub mod models;     // Data models and configuration
pub mod handlers;   // Request handlers
pub mod client;     // Client-side transport

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

#### Step 4: Register JSON-RPC Method

```rust
// src/jsonrpc/methods.rs - Add method constant
pub const NEW_FEATURE: &str = "kaiak/new_feature";

// src/jsonrpc/mod.rs - Register method with server
pub async fn register_kaiak_methods(
    server: &JsonRpcServer,
    // ... dependencies
) -> anyhow::Result<()> {
    // For non-streaming methods
    server.register_async_method(
        NEW_FEATURE.to_string(),
        move |params| {
            async move {
                let request: NewFeatureRequest = serde_json::from_value(
                    params.unwrap_or(serde_json::Value::Null)
                ).map_err(|e| JsonRpcError::custom(
                    error_codes::INVALID_PARAMS,
                    format!("Failed to parse parameters: {}", e),
                    None,
                ))?;
                
                // Handle request...
                Ok(serde_json::to_value(response)?)
            }
        },
    ).await?;
    
    // For streaming methods (sends concurrent notifications during execution)
    // Notifications are sent in real-time using tokio::select!, not buffered
    server.register_streaming_method(
        NEW_FEATURE.to_string(),
        move |params, notifier| {
            async move {
                // Use notifier.send() to send progress notifications
                // These are sent concurrently while the handler executes
                notifier.send(JsonRpcNotification::new(
                    "kaiak/newFeature/progress",
                    Some(serde_json::json!({"stage": "started"})),
                ))?;
                
                // Handle request...
                // More notifications can be sent at any time
                notifier.send(JsonRpcNotification::new(
                    "kaiak/newFeature/progress",
                    Some(serde_json::json!({"stage": "processing", "progress": 50})),
                ))?;
                
                Ok(serde_json::to_value(response)?)
            }
        },
    ).await?;
}
```

**Note**: The server can also receive notifications from clients. When a JSON-RPC message without an `id` field is received, it's processed as a notification - the handler runs but no response is sent.

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
# Kaiak

```
                _______________________
         ______/_______________________\______
    ____/_______/____  K A I A K  _____\_______\____
    \____\_______\_____________________/_______/____/
         \________\___________________/________/

```

[![CI](https://github.com/pranavgaikwad/kaiak/workflows/CI/badge.svg)](https://github.com/pranavgaikwad/kaiak/actions)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE-APACHE)

Kaiak is a standalone server that orchestrates the Goose AI agent for code migration workflows. It provides LSP-style JSON-RPC communication for IDE extensions, supports real-time concurrent progress streaming, bidirectional notifications, flexible session management and a full-featured CLI client.

## Quick Start

### Installation

#### Option 1: Pre-built Binary (Recommended)
```bash
# Download latest release
curl -L https://github.com/pranavgaikwad/kaiak/releases/latest/download/kaiak-linux-x86_64.tar.gz | tar xz
sudo mv kaiak /usr/local/bin/

# Verify installation
kaiak --version
```

#### Option 2: Build from Source

**Requirements**:
- Rust 1.75+ (stable toolchain)
- Git (for Goose dependency from GitHub)
- OpenSSL development libraries

```bash
# Clone the repository
git clone https://github.com/pranavgaikwad/kaiak.git
cd kaiak

# Build with release optimizations
cargo build --release

# Install to system
cargo install --path .
```

**Note**: Kaiak depends on [Goose](https://github.com/block/goose) as a git dependency. The build process will automatically fetch Goose from GitHub during compilation.

### Basic Setup

1. **Configure AI Provider**:
```bash
export OPENAI_API_KEY="your-api-key"
# or
export ANTHROPIC_API_KEY="your-api-key"
```

2. **Initialize Configuration**:
```bash
kaiak init
kaiak config edit  # Optional: customize settings
```

3. **Start Server**:
```bash
# Stdio transport (recommended for IDE integration)
kaiak serve --stdio

# Unix socket transport
kaiak serve --socket /tmp/kaiak.sock
```

### First Fix Generation

Kaiak provides a two-method JSON-RPC API:
- **kaiak/generate_fix** - Generate fixes for migration incidents (streaming, session_id optional)
- **kaiak/delete_session** - Clean up agent session

Configuration is provided at server startup via CLI arguments or config files.

**Session ID Handling**: The `session_id` in `generate_fix` is optional. If not provided, Kaiak creates a new session and returns the Goose-generated session ID in the response. Clients can reuse this ID for subsequent requests.

#### Using the CLI Client

```bash
# Step 1: Start the server with Unix socket
kaiak serve --socket /tmp/kaiak.sock

# Step 2: Connect the CLI client
kaiak connect /tmp/kaiak.sock

# Step 3: Generate fixes (session_id is optional - will be created if not provided)
kaiak generate-fix --params-json '{
  "incidents": [{
    "id": "issue-1",
    "rule_id": "deprecated-api",
    "message": "old_method() is deprecated",
    "severity": "warning"
  }],
  "agent_config": {
    "workspace": {"working_dir": "/path/to/project"},
    "model": {"provider": "openai", "model_id": "gpt-4"}
  }
}'

# The response will contain the generated session_id to reuse

# Step 4: Clean up (use the session_id from the response)
kaiak delete-session <session_id_from_response>

# Step 5: Disconnect
kaiak disconnect
```

#### Using JSON-RPC Directly

**Generate fixes**

```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/generate_fix",
  "params": {
    "incidents": [
      {
        "id": "issue-1",
        "rule_id": "deprecated-api",
        "message": "old_method() is deprecated, use new_method()",
        "description": "Deprecated API usage detected",
        "effort": "low",
        "severity": "warning"
      }
    ],
    "agent_config": {
      "workspace": {
        "working_dir": "/path/to/your/project"
      },
      "model": {
        "provider": "openai",
        "model_id": "gpt-4"
      }
    }
  },
  "id": 1
}
```

**Response includes the session_id**

```json
{
  "jsonrpc": "2.0",
  "result": {
    "request_id": "req-abc123",
    "session_id": "goose-generated-session-id",
    "created_at": "2025-12-30T10:00:00Z"
  },
  "id": 1
}
```

**Real-time streaming notifications (sent concurrently during processing):**

```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/generateFix/progress",
  "params": {
    "session_id": "goose-generated-session-id",
    "stage": "analyzing",
    "progress": 25
  }
}
```

**Clean up when done (use session_id from response):**

```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/delete_session",
  "params": {
    "session_id": "goose-generated-session-id"
  },
  "id": 2
}
```

## Architecture

Kaiak follows a modular architecture designed for enterprise safety and performance:

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   IDE Client    │◄──►│  Transport Layer │◄──►│  JSON-RPC Server│
│   CLI Client    │    │  (stdio/socket)  │    │  (streaming)    │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                                                         │
                                                         ▼
                       ┌──────────────────┐    ┌─────────────────┐
                       │     Handlers     │◄──►│  Notifications  │
                       │ (generate_fix)   │    │  (real-time)    │
                       └──────────────────┘    └─────────────────┘
                                │
                                ▼
                       ┌──────────────────┐    ┌─────────────────┐
                       │  Agent Manager   │◄──►│ Config Manager  │
                       │  (Goose Agent)   │    │  (hierarchy)    │
                       └──────────────────┘    └─────────────────┘
```

### Key Components

- **Transport Layer**: LSP-compatible stdio and Unix socket communication
- **JSON-RPC Server**: Protocol handling with method registration and concurrent streaming notifications
- **Handlers**: Request processing for `generate_fix` (streaming) and `delete_session`
- **Agent Manager**: Goose AI agent lifecycle and session coordination (optional session IDs)
- **Client Module**: CLI client for Unix socket communication with notification display
- **Configuration**: Hierarchical config loading (CLI > file > defaults)
- **Notification System**: Bidirectional - server can send and receive JSON-RPC notifications

## Security

Enterprise-safe design with multiple security layers:

- **Process Isolation**: No network exposure, stdio/socket only
- **File Modification Approval**: User confirmation required for all changes
- **Workspace Validation**: Configurable allowed directories
- **Input Sanitization**: Path traversal and injection prevention
- **API Key Validation**: Format verification and secure storage

## Configuration

### Server Configuration

```toml
[server]
transport = "stdio"  # or "socket"
socket_path = "/tmp/kaiak.sock"
log_level = "info"
max_concurrent_sessions = 10


```


## IDE Integration

### VSCode Extension

Kaiak integrates seamlessly with VSCode through the Language Server Protocol:

```typescript
import { LanguageClient } from 'vscode-languageclient/node';

const client = new LanguageClient(
    'kaiak',
    'Kaiak Migration Server',
    {
        command: 'kaiak',
        args: ['serve', '--stdio']
    },
    {
        documentSelector: [{ scheme: 'file', language: '*' }]
    }
);

client.start();
```

### Other IDEs

- **IntelliJ/JetBrains**: Language Server Protocol plugin
- **Vim/Neovim**: LSP configuration with stdio transport
- **Emacs**: lsp-mode integration

## Development

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run benchmarks
cargo test --test benchmarks --release

# Check formatting and linting
cargo fmt --check
cargo clippy -- -D warnings
```

### Testing

```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test integration

# Benchmark tests
cargo test --test benchmarks --release

# End-to-end validation
cargo test quickstart_validation
```

### Project Structure

```
src/
├── main.rs              # Entry point, CLI commands (serve, connect, generate-fix, etc.)
├── lib.rs               # Library exports and error types
├── jsonrpc/             # JSON-RPC 2.0 implementation
│   ├── protocol.rs      # Core types (Request, Response, Notification, Error)
│   ├── transport.rs     # Transport trait (StdioTransport, IpcTransport, IpcServerTransport)
│   ├── server.rs        # JSON-RPC server with concurrent streaming support
│   ├── methods.rs       # Method constants and registration
│   ├── core.rs          # Kaiak request/response wrappers
│   └── mod.rs           # Module exports and method registration
├── server/              # High-level server orchestration
│   ├── server.rs        # Server startup and configuration
│   └── mod.rs           # Server module exports
├── agent/               # Goose agent integration
│   ├── mod.rs           # GooseAgentManager for agent lifecycle
│   └── session_wrapper.rs # Session management with optional IDs
├── models/              # Data models and configuration
│   ├── configuration.rs # ServerConfig, BaseConfig, AgentConfig
│   ├── incidents.rs     # MigrationIncident models
│   └── mod.rs           # Model exports
├── handlers/            # Request handlers
│   ├── generate_fix.rs  # kaiak/generate_fix (streaming, optional session_id)
│   ├── delete_session.rs # kaiak/delete_session
│   └── mod.rs           # Handler exports
└── client/              # CLI client implementation
    ├── transport.rs     # JsonRpcClient with notification handling
    └── mod.rs           # Client exports (ConnectionState, shared types)
```

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Workflow

1. **Fork and clone** the repository
2. **Create a feature branch**: `git checkout -b feature-name`
3. **Make your changes** and add tests
4. **Run the test suite**: `cargo test`
5. **Check formatting**: `cargo fmt --check`
6. **Run linting**: `cargo clippy -- -D warnings`
7. **Submit a pull request**

### Code Standards

- **Rust 1.75+** with stable toolchain
- **Comprehensive testing** with focus on integration tests
- **Security-first** approach to all changes
- **Performance considerations** for concurrent operations
- **Documentation** for all public APIs

## License

**Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)


## Acknowledgments

- **Goose AI Agent**: Core AI processing capabilities are provided through [Goose](https://github.com/block/goose).

* AI agents (& curious humans): For project background and context, refer to [.specify/memory/context.md](.specify/memory/context.md).

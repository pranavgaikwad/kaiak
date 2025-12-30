# Kaiak

```
                _______________________
         ______/_______________________\______
    ____/_______/____  K A I A K  _____\_______\____
    \____\_______\_____________________/_______/____/
         \________\___________________/________/

```

[![CI](https://github.com/pranavgaikwad/kaiak/workflows/CI/badge.svg)](https://github.com/pranavgaikwad/kaiak/actions)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

Kaiak is a standalone server that orchestrates the Goose AI agent for code migration workflows. It provides LSP-style JSON-RPC communication for IDE extensions, supports real-time progress streaming, interactive tool calls, and session management.

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
- **kaiak/generate_fix** - Generate fixes for migration incidents (streaming)
- **kaiak/delete_session** - Clean up agent session

Configuration is provided at server startup via CLI arguments or config files.

#### Using the CLI Client

```bash
# Step 1: Start the server with Unix socket
kaiak serve --socket /tmp/kaiak.sock

# Step 2: Connect the CLI client
kaiak connect /tmp/kaiak.sock

# Step 3: Generate fixes
kaiak generate-fix --params-json '{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
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

# Step 4: Clean up
kaiak delete-session 550e8400-e29b-41d4-a716-446655440000

# Step 5: Disconnect
kaiak disconnect
```

#### Using JSON-RPC Directly

**Generate fixes:**

```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/generate_fix",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
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

**Monitor progress via streaming notifications:**

```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/generateFix/progress",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "stage": "analyzing",
    "progress": 25
  }
}
```

**Clean up when done:**

```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/delete_session",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000"
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
- **JSON-RPC Server**: Protocol handling with method registration and streaming notifications
- **Handlers**: Request processing for `generate_fix` and `delete_session`
- **Agent Manager**: Goose AI agent lifecycle and session coordination
- **Client Module**: CLI client for Unix socket communication
- **Configuration**: Hierarchical config loading (CLI > file > defaults)

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
├── main.rs              # Entry point, CLI commands, and argument parsing
├── lib.rs               # Library exports and error types
├── jsonrpc/             # JSON-RPC 2.0 implementation
│   ├── protocol.rs      # Core types (Request, Response, Notification, Error)
│   ├── transport.rs     # Transport trait (StdioTransport, IpcTransport)
│   ├── server.rs        # JSON-RPC server with streaming support
│   ├── methods.rs       # Method constants and registration
│   ├── core.rs          # Kaiak request/response wrappers
│   └── mod.rs           # Module exports and method registration
├── server/              # High-level server orchestration
│   ├── server.rs        # Server startup and configuration
│   └── mod.rs           # Server module exports
├── agent/               # Goose agent integration
│   └── mod.rs           # GooseAgentManager for agent lifecycle
├── models/              # Data models and configuration
│   ├── configuration.rs # ServerConfig, BaseConfig, AgentConfig
│   ├── incidents.rs     # MigrationIncident models
│   └── mod.rs           # Model exports
├── handlers/            # Request handlers
│   ├── generate_fix.rs  # kaiak/generate_fix (streaming)
│   ├── delete_session.rs # kaiak/delete_session
│   └── mod.rs           # Handler exports
└── client/              # CLI client implementation
    ├── transport.rs     # JsonRpcClient for Unix socket communication
    └── mod.rs           # Client exports (ConnectionState, etc.)
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

* AI agents: For project background and context, refer to [.specify/memory/context.md](.specify/memory/context.md).

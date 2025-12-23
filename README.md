# Kaiak

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
```bash
git clone https://github.com/pranavgaikwad/kaiak.git
cd kaiak
cargo build --release
cargo install --path .
```

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

Create a test workspace and run Kaiak:

```bash
# Navigate to your project
cd /path/to/your/project

# Start server for this workspace
kaiak serve --stdio --workspace $(pwd)
```

Send a JSON-RPC request for fix generation:

```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/session/create",
  "params": {
    "workspace_path": "/path/to/your/project",
    "session_name": "migration-session"
  },
  "id": 1
}
```

```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/fix/generate",
  "params": {
    "session_id": "your-session-id",
    "incidents": [
      {
        "id": "issue-1",
        "rule_id": "deprecated-api",
        "file_path": "src/main.rs",
        "line_number": 42,
        "severity": "warning",
        "description": "Deprecated API usage",
        "message": "old_method() is deprecated, use new_method()"
      }
    ]
  },
  "id": 2
}
```

Monitor real-time progress updates and approve/reject proposed file modifications through streaming notifications.

## Architecture

Kaiak follows a modular architecture designed for enterprise safety and performance:

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   IDE Client    │◄──►│  Transport Layer │◄──►│  JSON-RPC Core  │
└─────────────────┘    │   (stdio/socket) │    └─────────────────┘
                       └──────────────────┘              │
                                                         ▼
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│ Session Manager │◄──►│     Handlers     │◄──►│ Stream Manager  │
│  (LRU Cache)    │    │ (Fix/Lifecycle)  │    │  (Real-time)    │
└─────────────────┘    └──────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│  Goose Agent    │    │ Security Layer   │    │ Config Manager  │
│  Integration    │    │ (Approval Flow)  │    │  (Validation)   │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

### Key Components

- **Transport Layer**: LSP-compatible stdio/socket communication
- **JSON-RPC Core**: Protocol handling with streaming support
- **Session Manager**: Concurrent session handling with LRU caching
- **Goose Integration**: AI agent lifecycle and processing
- **Security Layer**: File modification approval and validation
- **Stream Manager**: Real-time progress updates and notifications

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

[ai]
provider = "openai"  # or "anthropic"
model = "gpt-4"
timeout = 300
max_turns = 50

[security]
require_approval = true
approval_timeout = 300  # 5 minutes

[performance]
stream_buffer_size = 1000
session_cache_size = 100
```

### Environment Variables

```bash
# AI Provider Configuration
export OPENAI_API_KEY="your-key"
export ANTHROPIC_API_KEY="your-key"

# Kaiak Configuration
export KAIAK_CONFIG_PATH="/custom/config/path"
export KAIAK_LOG_LEVEL="debug"
export KAIAK_WORKSPACE_ROOT="/default/workspace"

# Development
export RUST_LOG="kaiak=debug"
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
├── main.rs              # Entry point and CLI argument parsing
├── server/              # Core server implementation
│   ├── transport.rs     # IPC transport layer
│   ├── jsonrpc.rs       # JSON-RPC protocol handling
│   └── server.rs        # Main server orchestration
├── goose/               # Goose agent integration
│   ├── agent.rs         # Agent lifecycle management
│   ├── session.rs       # Session state management
│   ├── monitoring.rs    # Performance monitoring
│   └── resources.rs     # Resource management
├── models/              # Data models and entities
│   ├── session.rs       # Session models
│   ├── request.rs       # Fix generation requests
│   ├── incident.rs      # Code incident models
│   └── messages.rs      # Stream message types
├── handlers/            # Request processing logic
│   ├── fix_generation.rs # Core fix generation
│   ├── lifecycle.rs     # Agent lifecycle operations
│   ├── streaming.rs     # Real-time streaming
│   └── interactions.rs  # User interaction handling
└── config/              # Configuration management
    ├── settings.rs      # Configuration structures
    ├── security.rs      # Security hardening
    └── validation.rs    # Configuration validation
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

This project is dual-licensed under either:

- **MIT License** ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

at your option.

## Support

- **Documentation**: [docs.kaiak.dev](https://docs.kaiak.dev)
- **Issues**: [GitHub Issues](https://github.com/pranavgaikwad/kaiak/issues)
- **Discussions**: [GitHub Discussions](https://github.com/pranavgaikwad/kaiak/discussions)

## Acknowledgments

- **Goose AI Agent**: Core AI processing capabilities
- **Tower LSP**: Robust Language Server Protocol implementation
- **Tokio**: Asynchronous runtime foundation
- **The Rust Community**: For excellent tooling and libraries
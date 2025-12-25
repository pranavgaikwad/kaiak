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

Kaiak provides a simplified three-endpoint API for agent operations:
- **kaiak/configure** - Configure agent for a session
- **kaiak/generate_fix** - Generate fixes for migration incidents
- **kaiak/delete_session** - Clean up agent session

**Step 1**: Start the server

```bash
# Start server with stdio transport (recommended for IDE integration)
kaiak serve --stdio
```

**Step 2**: Configure the agent (one-time per session)

```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/configure",
  "params": {
    "workspace": {
      "working_dir": "/path/to/your/project",
      "include_patterns": ["**/*.rs", "**/*.toml"],
      "exclude_patterns": ["target/**", "**/*.bak"]
    },
    "model": {
      "provider": "openai",
      "model": "gpt-4"
    },
    "tools": {
      "enabled_extensions": ["developer", "todo"],
      "custom_tools": [],
      "planning_mode": false
    },
    "permissions": {
      "tool_permissions": {
        "file_write": "approve",
        "file_read": "allow",
        "shell_command": "deny"
      }
    }
  },
  "id": 1
}
```

**Step 3**: Generate fixes for identified incidents

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
    ]
  },
  "id": 2
}
```

**Step 4**: Monitor real-time progress via streaming notifications

The agent streams events as it works:
- **kaiak/stream/progress** - Execution progress updates
- **kaiak/stream/ai_response** - AI model responses
- **kaiak/stream/tool_call** - Tool execution status
- **kaiak/stream/user_interaction** - Approval prompts for file modifications

**Step 5**: Clean up when done

```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/delete_session",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "cleanup_options": {
      "force": false,
      "cleanup_temp_files": true,
      "preserve_logs": true
    }
  },
  "id": 3
}
```

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
# Provider configuration is passed as arbitrary JSON to Goose
# Set via environment variables or session creation parameters
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
├── lib.rs               # Library exports and error types
├── server/              # Core server implementation
│   ├── transport.rs     # IPC transport layer (stdio/socket)
│   ├── jsonrpc.rs       # JSON-RPC protocol types and error codes
│   ├── server.rs        # Main LSP server orchestration
│   └── mod.rs           # Server module exports
├── agents/              # Goose agent integration layer
│   ├── mod.rs           # GooseAgentManager for agent lifecycle
│   ├── session_wrapper.rs  # Session management with Goose SessionManager
│   └── event_streaming.rs  # Event mapping (Goose → Kaiak notifications)
├── models/              # Data models and entities
│   ├── configuration.rs # AgentConfiguration (per-session config)
│   ├── incidents.rs     # MigrationIncident models
│   ├── events.rs        # AgentEventNotification types
│   ├── interactions.rs  # UserInteractionRequest types
│   ├── session.rs       # Session type re-exports
│   └── mod.rs           # Model exports
├── handlers/            # Three-endpoint request handlers
│   ├── configure.rs     # kaiak/configure handler
│   ├── generate_fix.rs  # kaiak/generate_fix handler
│   ├── delete_session.rs # kaiak/delete_session handler
│   └── mod.rs           # Handler exports
└── config/              # Server configuration management
    ├── settings.rs      # ServerSettings (server-wide config)
    ├── security.rs      # Security hardening
    ├── validation.rs    # Configuration validation
    └── mod.rs           # Config module and logging setup
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

- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)


## Acknowledgments

- **Goose AI Agent**: Core AI processing capabilities are provided through [Goose](https://github.com/block/goose).
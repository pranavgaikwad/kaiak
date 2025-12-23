# Quickstart Guide: Kaiak Migration Server

**Purpose**: Get Kaiak server running and integrated with IDE for fix generation workflows
**Audience**: Developers, DevOps engineers, IDE extension developers
**Prerequisites**: Rust 1.75+, Git, Basic understanding of JSON-RPC

## Table of Contents

1. [Installation](#installation)
2. [Basic Setup](#basic-setup)
3. [IDE Integration](#ide-integration)
4. [First Fix Generation](#first-fix-generation)
5. [Configuration](#configuration)
6. [Testing](#testing)
7. [Troubleshooting](#troubleshooting)

---

## Installation

### Option 1: Pre-built Binary (Recommended)

```bash
# Download latest release
curl -L https://github.com/your-org/kaiak/releases/latest/download/kaiak-linux-x86_64.tar.gz | tar xz

# Move to PATH
sudo mv kaiak /usr/local/bin/

# Verify installation
kaiak --version
```

### Option 2: Build from Source

```bash
# Clone repository
git clone https://github.com/your-org/kaiak.git
cd kaiak

# Build release binary
cargo build --release

# Install locally
cargo install --path .
```

### Option 3: Cargo Install

```bash
# Install from crates.io (when published)
cargo install kaiak

# Or install from git
cargo install --git https://github.com/your-org/kaiak.git
```

---

## Basic Setup

### 1. Configure AI Provider

Set up your preferred AI provider credentials:

**OpenAI**:
```bash
export OPENAI_API_KEY="your-api-key"
```

**Anthropic**:
```bash
export ANTHROPIC_API_KEY="your-api-key"
```

### 2. Initialize Configuration

```bash
# Create default configuration
kaiak init

# Edit configuration file
kaiak config edit
```

Default configuration location: `~/.config/kaiak/config.toml`

### 3. Test Server

```bash
# Start server with stdio transport
kaiak serve --stdio

# Or start with Unix socket
kaiak serve --socket /tmp/kaiak.sock

# Verify server is responding
echo '{"jsonrpc":"2.0","method":"kaiak/session/create","params":{"workspace_path":"/tmp"},"id":1}' | kaiak serve --stdio
```

---

## IDE Integration

### VSCode Extension Integration

**1. Install Extension** (when available):
```bash
code --install-extension kaiak-migration-assistant
```

**2. Manual Integration** for development:

Create `package.json` for your extension:
```json
{
  "name": "kaiak-client",
  "version": "0.1.0",
  "dependencies": {
    "vscode-languageclient": "^9.0.0"
  }
}
```

Create `src/extension.ts`:
```typescript
import * as vscode from 'vscode';
import { LanguageClient, ServerOptions, LanguageClientOptions } from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: vscode.ExtensionContext) {
    // Server executable
    const serverOptions: ServerOptions = {
        command: 'kaiak',
        args: ['serve', '--stdio']
    };

    // Client configuration
    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: '*' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*')
        }
    };

    // Create and start client
    client = new LanguageClient(
        'kaiak',
        'Kaiak Migration Server',
        serverOptions,
        clientOptions
    );

    client.start();
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
```

### Other IDE Integration

**IntelliJ/JetBrains**:
- Use Language Server Protocol plugin
- Configure external tool with stdio transport

**Vim/Neovim**:
```lua
-- LSP configuration for Neovim
local lspconfig = require('lspconfig')

lspconfig.kaiak = {
    default_config = {
        cmd = { 'kaiak', 'serve', '--stdio' },
        filetypes = { 'rust', 'javascript', 'python', 'java' },
        root_dir = function(fname)
            return lspconfig.util.find_git_ancestor(fname)
        end,
        settings = {}
    }
}
```

**Emacs**:
```elisp
;; LSP mode configuration
(use-package lsp-mode
  :hook ((rust-mode . lsp-deferred))
  :commands lsp
  :config
  (add-to-list 'lsp-language-id-configuration '(rust-mode . "rust"))
  (lsp-register-client
   (make-lsp-client :new-connection (lsp-stdio-connection '("kaiak" "serve" "--stdio"))
                    :major-modes '(rust-mode)
                    :server-id 'kaiak)))
```

---

## First Fix Generation

### 1. Open Workspace with Issues

```bash
# Navigate to a project with code issues
cd /path/to/your/project

# Start Kaiak server for this workspace
kaiak serve --stdio --workspace $(pwd)
```

### 2. Generate Fix Request

Using the IDE extension UI:
1. Open file with static analysis issues
2. Select "Generate Fix" from context menu
3. Choose incidents to fix
4. Review AI-generated solutions
5. Approve/reject proposed changes

**Manual JSON-RPC request** (for testing):
```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/session/create",
  "params": {
    "workspace_path": "/path/to/your/project",
    "session_name": "test-session"
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

### 3. Monitor Progress

Watch for streaming notifications:
- Progress updates
- AI thinking process
- Tool execution results
- File modification proposals
- User interaction requests

### 4. Review and Approve

When file modifications are proposed:
1. Review the diff in your IDE
2. Validate the changes make sense
3. Approve or reject via interaction response
4. Changes are applied automatically upon approval

---

## Configuration

### Server Configuration (`~/.config/kaiak/config.toml`)

```toml
[server]
# Transport method: "stdio" or "socket"
transport = "stdio"
socket_path = "/tmp/kaiak.sock"
log_level = "info"
max_concurrent_sessions = 10

[ai]
# Default provider
provider = "openai"
model = "gpt-4"
timeout = 300
max_turns = 50

[ai.providers.openai]
api_key = "${OPENAI_API_KEY}"
base_url = "https://api.openai.com/v1"

[ai.providers.anthropic]
api_key = "${ANTHROPIC_API_KEY}"

[workspace]
# File patterns to exclude from analysis
exclude_patterns = ["target/", "node_modules/", ".git/", "*.tmp"]
max_file_size = 1048576  # 1MB

[security]
# File modification approval required
require_approval = true
approval_timeout = 300  # 5 minutes

[performance]
# Streaming message buffer size
stream_buffer_size = 1000
# Session cache size (LRU)
session_cache_size = 100
```

### Environment Variables

```bash
# AI Provider Configuration
export OPENAI_API_KEY="your-key"
export ANTHROPIC_API_KEY="your-key"
export DATABRICKS_HOST="your-host"
export DATABRICKS_TOKEN="your-token"

# Kaiak Configuration
export KAIAK_CONFIG_PATH="/custom/config/path"
export KAIAK_LOG_LEVEL="debug"
export KAIAK_WORKSPACE_ROOT="/default/workspace"

# Development
export KAIAK_DEV_MODE="true"
export RUST_LOG="kaiak=debug"
```

---

## Testing

### Unit Tests

```bash
# Run all tests
cargo test

# Run specific test modules
cargo test --package kaiak --lib handlers::fix_generation

# Run with output
cargo test -- --nocapture
```

### Integration Tests

```bash
# Full integration test suite
cargo test --test integration

# End-to-end workflow tests
cargo test --test e2e

# Performance tests
cargo test --test benchmarks --release
```

### Manual Testing with CLI

```bash
# Test session creation
kaiak test session-create --workspace /tmp

# Test fix generation with sample data
kaiak test fix-generate --incidents test-data/sample-incidents.json

# Test streaming output
kaiak test stream --session-id test-session

# Test transport layer
kaiak test transport --method stdio
```

### JSON-RPC Testing

Using `curl` with stdio simulation:
```bash
# Create session
echo '{"jsonrpc":"2.0","method":"kaiak/session/create","params":{"workspace_path":"/tmp"},"id":1}' \
  | kaiak serve --stdio

# Generate fix
echo '{"jsonrpc":"2.0","method":"kaiak/fix/generate","params":{"session_id":"test","incidents":[]},"id":2}' \
  | kaiak serve --stdio
```

---

## Troubleshooting

### Common Issues

**1. Server Won't Start**
```bash
# Check configuration
kaiak config validate

# Check dependencies
kaiak doctor

# View detailed logs
RUST_LOG=debug kaiak serve --stdio
```

**2. AI Provider Errors**
```bash
# Verify credentials
kaiak test provider --provider openai

# Check network connectivity
kaiak test network

# Validate model configuration
kaiak config show ai.providers
```

**3. Permission Issues**
```bash
# Check file permissions
ls -la ~/.config/kaiak/

# Fix socket permissions
chmod 600 /tmp/kaiak.sock
```

**4. IDE Integration Issues**
- Verify Kaiak is in PATH: `which kaiak`
- Check IDE extension logs
- Test manual JSON-RPC communication
- Validate workspace path permissions

### Debug Mode

```bash
# Enable debug logging
export RUST_LOG=debug
kaiak serve --stdio

# Trace JSON-RPC messages
export KAIAK_TRACE_RPC=true
kaiak serve --stdio

# Performance profiling
export KAIAK_PROFILE=true
kaiak serve --stdio
```

### Log Locations

- **Server logs**: `~/.config/kaiak/logs/kaiak.log`
- **Session logs**: `~/.config/kaiak/logs/sessions/`
- **Error logs**: `~/.config/kaiak/logs/errors/`

### Health Check

```bash
# Server health check
kaiak health

# System requirements check
kaiak doctor

# Configuration validation
kaiak config validate

# Performance benchmark
kaiak benchmark
```

---

## Next Steps

1. **Explore Advanced Configuration**: Customize AI prompts, add custom tools
2. **IDE Extension Development**: Build custom IDE integrations
3. **Multi-Language Support**: Configure for different programming languages
4. **CI/CD Integration**: Automate fix generation in build pipelines
5. **Team Setup**: Configure shared settings for development teams

For detailed documentation, visit: [https://docs.kaiak.dev](https://docs.kaiak.dev)

For support and issues: [https://github.com/your-org/kaiak/issues](https://github.com/your-org/kaiak/issues)
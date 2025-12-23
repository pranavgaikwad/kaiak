# Kaiak User Guide

Complete guide to using Kaiak for AI-powered code migration workflows.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Configuration](#configuration)
3. [IDE Integration](#ide-integration)
4. [Fix Generation Workflows](#fix-generation-workflows)
5. [Managing Sessions](#managing-sessions)
6. [File Modification Approval](#file-modification-approval)
7. [Monitoring and Debugging](#monitoring-and-debugging)
8. [Best Practices](#best-practices)
9. [Troubleshooting](#troubleshooting)

## Getting Started

### Installation

#### Pre-built Binaries

Download the latest release for your platform:

```bash
# Linux (x86_64)
curl -L https://github.com/pranavgaikwad/kaiak/releases/latest/download/kaiak-linux-x86_64.tar.gz | tar xz
sudo mv kaiak /usr/local/bin/

# macOS (ARM64)
curl -L https://github.com/pranavgaikwad/kaiak/releases/latest/download/kaiak-macos-arm64.tar.gz | tar xz
sudo mv kaiak /usr/local/bin/

# Windows
# Download kaiak-windows-x86_64.zip and extract to your PATH
```

#### Build from Source

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build Kaiak
git clone https://github.com/pranavgaikwad/kaiak.git
cd kaiak
cargo build --release

# Install locally
cargo install --path .
```

### Initial Setup

1. **Verify Installation**:
```bash
kaiak --version
```

2. **Initialize Configuration**:
```bash
kaiak init
```

This creates a default configuration file at `~/.config/kaiak/config.toml`.

3. **Configure AI Provider**:

Set up your API key for your preferred AI provider:

```bash
# OpenAI
export OPENAI_API_KEY="sk-your-openai-api-key"

# Anthropic
export ANTHROPIC_API_KEY="sk-ant-your-anthropic-api-key"

# Make persistent (add to ~/.bashrc, ~/.zshrc, etc.)
echo 'export OPENAI_API_KEY="sk-your-openai-api-key"' >> ~/.bashrc
```

4. **Test Configuration**:
```bash
kaiak config validate
```

## Configuration

### Configuration File

The main configuration file is located at `~/.config/kaiak/config.toml`:

```toml
[server]
# Transport method: "stdio" (recommended for IDEs) or "socket"
transport = "stdio"
socket_path = "/tmp/kaiak.sock"
log_level = "info"
max_concurrent_sessions = 10

[ai]
# AI provider configuration
provider = "openai"  # or "anthropic"
model = "gpt-4"     # or "claude-3-opus", "gpt-4-turbo", etc.
timeout = 300       # 5 minutes
max_turns = 50

[ai.providers.openai]
api_key = "${OPENAI_API_KEY}"
base_url = "https://api.openai.com/v1"

[ai.providers.anthropic]
api_key = "${ANTHROPIC_API_KEY}"

[workspace]
# File patterns to exclude from processing
exclude_patterns = [
    "target/",      # Rust build artifacts
    "node_modules/", # Node.js dependencies
    ".git/",        # Git repository
    "*.tmp",        # Temporary files
    "*.log"         # Log files
]
max_file_size = 1048576  # 1MB

[security]
# File modification requires user approval
require_approval = true
approval_timeout = 300  # 5 minutes

[performance]
stream_buffer_size = 1000
session_cache_size = 100
```

### Environment Variables

Override configuration with environment variables:

```bash
# Server settings
export KAIAK_LOG_LEVEL="debug"
export KAIAK_TRANSPORT="stdio"
export KAIAK_MAX_SESSIONS="5"

# AI settings
export KAIAK_AI_PROVIDER="anthropic"
export KAIAK_AI_MODEL="claude-3-opus"

# Workspace settings
export KAIAK_WORKSPACE_ROOT="/home/user/projects"

# Development
export RUST_LOG="kaiak=debug"
```

### Configuration Commands

```bash
# Show current configuration
kaiak config show

# Edit configuration file
kaiak config edit

# Validate configuration
kaiak config validate

# Reset to defaults
kaiak config reset
```

## IDE Integration

### VSCode Extension

#### Installation

1. **Install the Extension** (when available):
```bash
code --install-extension kaiak-migration-assistant
```

2. **Manual Setup** (for development):

Create a VSCode extension with the following `package.json`:

```json
{
  "name": "kaiak-client",
  "version": "0.1.0",
  "engines": {
    "vscode": "^1.74.0"
  },
  "dependencies": {
    "vscode-languageclient": "^9.0.0"
  }
}
```

And `src/extension.ts`:

```typescript
import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: vscode.ExtensionContext) {
    const serverOptions: ServerOptions = {
        command: 'kaiak',
        args: ['serve', '--stdio']
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: '*' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*')
        }
    };

    client = new LanguageClient(
        'kaiak',
        'Kaiak Migration Server',
        serverOptions,
        clientOptions
    );

    client.start();
}

export function deactivate(): Thenable<void> | undefined {
    return client?.stop();
}
```

#### Usage

1. **Open Project**: Open your codebase in VSCode
2. **Start Kaiak**: The extension automatically starts the Kaiak server
3. **Generate Fixes**: Use the Command Palette (`Ctrl+Shift+P`) and search for "Kaiak: Generate Fixes"
4. **Review Changes**: Proposed changes appear in the diff editor
5. **Approve/Reject**: Use the approval UI to accept or reject changes

### Other IDEs

#### IntelliJ IDEA / JetBrains IDEs

1. Install the "Language Server Protocol Support" plugin
2. Add Kaiak as an external tool:
   - **Program**: `kaiak`
   - **Arguments**: `serve --stdio`
   - **Working directory**: `$ProjectFileDir$`

#### Vim/Neovim

Add to your `.vimrc` or `init.lua`:

```lua
local lspconfig = require('lspconfig')

lspconfig.kaiak = {
    default_config = {
        cmd = { 'kaiak', 'serve', '--stdio' },
        filetypes = { 'rust', 'javascript', 'python', 'java', 'go' },
        root_dir = function(fname)
            return lspconfig.util.find_git_ancestor(fname)
        end,
        settings = {}
    }
}

-- Start Kaiak for supported file types
require('lspconfig').kaiak.setup{}
```

#### Emacs

Add to your Emacs configuration:

```elisp
(use-package lsp-mode
  :hook ((rust-mode . lsp-deferred))
  :commands lsp
  :config
  (lsp-register-client
   (make-lsp-client :new-connection (lsp-stdio-connection '("kaiak" "serve" "--stdio"))
                    :major-modes '(rust-mode python-mode javascript-mode)
                    :server-id 'kaiak)))
```

## Fix Generation Workflows

### Basic Fix Generation

1. **Identify Issues**: Run static analysis tools to identify issues:
```bash
# Examples with different tools
cargo clippy --message-format=json > issues.json      # Rust
eslint --format=json src/ > issues.json               # JavaScript
pylint --output-format=json src/ > issues.json        # Python
```

2. **Start Kaiak Server**:
```bash
cd /path/to/your/project
kaiak serve --stdio --workspace $(pwd)
```

3. **Create Session**: Use your IDE extension or send JSON-RPC directly:
```bash
# Using curl to simulate IDE communication
echo '{
  "jsonrpc": "2.0",
  "method": "kaiak/session/create",
  "params": {
    "workspace_path": "'$(pwd)'",
    "session_name": "fix-deprecated-apis"
  },
  "id": 1
}' | kaiak serve --stdio
```

4. **Generate Fixes**: Send incidents for processing:
```bash
echo '{
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
        "description": "Deprecated function usage",
        "message": "replace old_func() with new_func()"
      }
    ]
  },
  "id": 2
}' | kaiak serve --stdio
```

### Batch Processing

Process multiple files or issues at once:

```bash
# Create a batch fix request
kaiak batch-fix --workspace /path/to/project --issues issues.json --session "batch-migration"
```

### Migration Scenarios

#### API Version Upgrades

```json
{
  "incidents": [
    {
      "id": "api-v1-to-v2",
      "rule_id": "api-version-upgrade",
      "file_path": "src/api_client.rs",
      "migration_context": {
        "from_version": "1.0",
        "to_version": "2.0",
        "breaking_changes": [
          "Method signatures changed",
          "New authentication required"
        ]
      }
    }
  ]
}
```

#### Dependency Updates

```json
{
  "incidents": [
    {
      "id": "tokio-upgrade",
      "rule_id": "dependency-upgrade",
      "file_path": "Cargo.toml",
      "migration_context": {
        "dependency": "tokio",
        "from_version": "1.0",
        "to_version": "1.35",
        "changelog_url": "https://github.com/tokio-rs/tokio/blob/master/CHANGELOG.md"
      }
    }
  ]
}
```

## Managing Sessions

### Session Lifecycle

1. **Create**: Initialize a new session
2. **Process**: Submit fix generation requests
3. **Monitor**: Track progress and handle interactions
4. **Cleanup**: Terminate when done

### Session Commands

```bash
# List active sessions
kaiak session list

# Show session details
kaiak session status <session-id>

# Terminate session
kaiak session terminate <session-id>

# Cleanup all sessions
kaiak session cleanup
```

### Session Configuration

Each session can have custom configuration:

```json
{
  "session_name": "custom-session",
  "configuration": {
    "provider": "anthropic",
    "model": "claude-3-opus",
    "timeout": 600,
    "max_turns": 100,
    "custom_prompts": {
      "system": "You are a migration expert for Rust code...",
      "context": "Focus on memory safety and performance..."
    }
  }
}
```

## File Modification Approval

### Approval Workflow

1. **AI Proposes Changes**: Kaiak analyzes code and proposes modifications
2. **User Review**: Changes are presented for review with diff view
3. **Approval Decision**: User approves, rejects, or requests modifications
4. **Application**: Approved changes are applied automatically

### Approval Interface

#### Command Line

```bash
# Review pending proposals
kaiak proposals list

# Approve proposal
kaiak proposals approve <proposal-id>

# Reject proposal
kaiak proposals reject <proposal-id>

# Approve all proposals in session
kaiak proposals approve-all <session-id>
```

#### IDE Integration

- **Diff View**: Side-by-side comparison of original and proposed code
- **Approval Buttons**: One-click approve/reject for each change
- **Batch Operations**: Approve/reject multiple changes at once
- **Comments**: Add feedback for rejected changes

### Security Features

- **Workspace Validation**: Only files within the workspace can be modified
- **Path Sanitization**: Prevents directory traversal attacks
- **Backup Creation**: Original files are backed up before modification
- **Rollback Support**: Easy rollback of approved changes

### Approval Configuration

```toml
[security]
# Require approval for all modifications
require_approval = true

# Approval timeout (5 minutes)
approval_timeout = 300

# Auto-apply after approval
auto_apply_approved_modifications = true

# Maximum concurrent approvals per session
max_concurrent_approvals = 10
```

## Monitoring and Debugging

### Real-time Progress

Kaiak provides real-time progress updates through streaming:

- **Progress Percentage**: Overall completion status
- **Current Phase**: What the AI is currently working on
- **Detailed Description**: Human-readable progress description

### AI Thinking Process

Monitor the AI's reasoning process:

- **Thinking Streams**: See the AI's thought process in real-time
- **Tool Calls**: Monitor which tools the AI is using
- **Context Analysis**: Understand how the AI interprets your code

### Logging

Configure detailed logging for debugging:

```bash
# Enable debug logging
export RUST_LOG="kaiak=debug"
kaiak serve --stdio

# Trace-level logging for maximum detail
export RUST_LOG="kaiak=trace"

# Log to file
kaiak serve --stdio 2>&1 | tee kaiak.log
```

### Performance Monitoring

```bash
# Show performance statistics
kaiak stats

# Monitor resource usage
kaiak monitor

# Performance benchmarks
kaiak benchmark
```

### Health Checks

```bash
# Basic health check
kaiak health

# Detailed system check
kaiak doctor

# Configuration validation
kaiak config validate

# Network connectivity test
kaiak test network
```

## Best Practices

### Project Setup

1. **Use Version Control**: Always commit before running Kaiak
2. **Create Branches**: Use feature branches for migration work
3. **Backup Important Files**: Additional backup for critical files

### Configuration

1. **Start Conservative**: Begin with small timeouts and limited concurrent sessions
2. **Customize Prompts**: Add project-specific context to AI prompts
3. **Exclude Build Artifacts**: Ensure build outputs are excluded from processing

### Workflow

1. **Small Batches**: Process issues in small batches for better control
2. **Review Everything**: Carefully review all proposed changes
3. **Test After Changes**: Run tests after applying changes
4. **Incremental Migration**: Migrate incrementally rather than all at once

### Security

1. **Review File Paths**: Verify all proposed file modifications are expected
2. **Check Permissions**: Ensure proper file permissions after modifications
3. **Validate Changes**: Review the actual code changes, not just the descriptions

## Troubleshooting

### Common Issues

#### Server Won't Start

```bash
# Check configuration
kaiak config validate

# Check dependencies
kaiak doctor

# View detailed logs
RUST_LOG=debug kaiak serve --stdio
```

**Solutions**:
- Verify API keys are set correctly
- Check file permissions on config directory
- Ensure workspace path exists and is accessible

#### AI Provider Errors

```bash
# Test provider connectivity
kaiak test provider --provider openai

# Validate API key format
kaiak config validate
```

**Solutions**:
- Verify API key is valid and has sufficient credits
- Check network connectivity
- Try alternative provider

#### Permission Issues

```bash
# Check file permissions
ls -la ~/.config/kaiak/

# Fix socket permissions (if using socket transport)
chmod 600 /tmp/kaiak.sock
```

**Solutions**:
- Ensure user has write access to config directory
- Fix socket file permissions
- Check workspace directory permissions

#### IDE Integration Issues

**Solutions**:
- Verify Kaiak is in PATH: `which kaiak`
- Check IDE extension logs
- Test manual JSON-RPC communication
- Restart IDE after installing/updating extension

### Debug Mode

Enable comprehensive debugging:

```bash
# Maximum verbosity
export RUST_LOG=debug
export KAIAK_TRACE_RPC=true
export KAIAK_PROFILE=true

kaiak serve --stdio 2>&1 | tee debug.log
```

### Log Locations

- **Server logs**: `~/.config/kaiak/logs/kaiak.log`
- **Session logs**: `~/.config/kaiak/logs/sessions/`
- **Error logs**: `~/.config/kaiak/logs/errors/`

### Getting Help

- **Documentation**: [docs.kaiak.dev](https://docs.kaiak.dev)
- **Issues**: [GitHub Issues](https://github.com/pranavgaikwad/kaiak/issues)
- **Discussions**: [GitHub Discussions](https://github.com/pranavgaikwad/kaiak/discussions)
- **Discord**: [Kaiak Community](https://discord.gg/kaiak)

When reporting issues, include:

1. Kaiak version: `kaiak --version`
2. Operating system and version
3. Configuration file (remove API keys)
4. Error logs with debug information
5. Steps to reproduce the issue
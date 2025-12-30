# JSON-RPC API Reference: Kaiak Agent API

**Version**: 2.0  
**Date**: 2025-12-29  
**Protocol**: JSON-RPC 2.0  
**Transport**: LSP-compatible (Content-Length headers + JSON)

## Overview

Kaiak provides a simplified JSON-RPC API for agent operations. All communication follows LSP message framing and supports real-time concurrent streaming notifications. The server supports bidirectional notifications - it can both send and receive JSON-RPC notifications.

## Transport Layer

### Message Framing

All messages use standard LSP (Language Server Protocol) framing:

```
Content-Length: {byte_length}\r\n
\r\n
{json_message}
```

### Supported Transports

- **stdio**: Messages over stdin/stdout (recommended for IDE integration)
- **Unix domain socket**: Enterprise-safe IPC communication (recommended for CLI)

## API Methods

Kaiak exposes two methods:

| Method | Description | Streaming |
|--------|-------------|-----------|
| `kaiak/generate_fix` | Generate fixes for migration incidents | Yes |
| `kaiak/delete_session` | Clean up agent session | No |

---

## 1. kaiak/generate_fix

Process migration incidents with the Goose AI agent. Sends progress notifications **concurrently** during execution using `tokio::select!` for real-time streaming.

### Request

```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/generate_fix",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "incidents": [
      {
        "id": "incident-1",
        "rule_id": "deprecated-api-usage",
        "message": "Use of deprecated API javax.xml.bind.DatatypeConverter",
        "description": "The javax.xml.bind.DatatypeConverter class is deprecated in Java 9+",
        "effort": "trivial",
        "severity": "warning"
      }
    ],
    "migration_context": {
      "source_technology": "Java 8",
      "target_technology": "Java 17",
      "migration_hints": ["Use java.util.Base64 instead of DatatypeConverter"]
    },
    "agent_config": {
      "workspace": {
        "working_dir": "/path/to/project",
        "include_patterns": ["**/*.java"],
        "exclude_patterns": ["target/**"]
      },
      "model": {
        "provider": "openai",
        "model_id": "gpt-4",
        "temperature": 0.1,
        "max_tokens": 4096
      }
    }
  },
  "id": 1
}
```

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `session_id` | string | **No** | Session identifier. If omitted, a new session is created and the Goose-generated ID is returned |
| `incidents` | array | Yes | Migration incidents to process (1-1000 items) |
| `migration_context` | object | No | Additional context for the migration |
| `agent_config` | object | Yes | Agent configuration |

#### Incident Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | Yes | Unique incident identifier |
| `rule_id` | string | Yes | Static analysis rule identifier |
| `message` | string | Yes | Brief incident description |
| `description` | string | No | Detailed incident explanation |
| `effort` | string | No | Estimated fix effort: `trivial`, `low`, `medium`, `high` |
| `severity` | string | No | Issue severity: `info`, `warning`, `error`, `critical` |

#### Agent Config Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `workspace.working_dir` | string | Yes | Absolute path to workspace directory |
| `workspace.include_patterns` | array | No | Glob patterns for included files |
| `workspace.exclude_patterns` | array | No | Glob patterns for excluded files |
| `model.provider` | string | Yes | Provider: `openai`, `anthropic`, `databricks` |
| `model.model_id` | string | Yes | Model identifier |
| `model.temperature` | number | No | Generation temperature (0.0-1.0) |
| `model.max_tokens` | number | No | Maximum tokens per response |

### Response (Success)

```json
{
  "jsonrpc": "2.0",
  "result": {
    "request_id": "req-550e8400-e29b-41d4-a716-446655440001",
    "session_id": "goose-generated-or-provided-session-id",
    "created_at": "2025-12-25T10:35:45Z"
  },
  "id": 1
}
```

**Note:** The `session_id` in the response is the actual session ID used. If you didn't provide one, this is the Goose-generated ID that you should use for subsequent requests (e.g., `delete_session`) or to continue an existing session.

### Response (Error)

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32602,
    "message": "Invalid params: Must provide 1-1000 incidents",
    "data": null
  },
  "id": 1
}
```

---

## 2. kaiak/delete_session

Delete an agent session and clean up associated resources.

### Request

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

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `session_id` | string | Yes | Session identifier to delete |

### Response (Success)

```json
{
  "jsonrpc": "2.0",
  "result": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "deleted": true,
    "deleted_at": "2025-12-25T10:40:00Z"
  },
  "id": 2
}
```

### Response (Error - Session Not Found)

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32003,
    "message": "Session not found",
    "data": {
      "session_id": "550e8400-e29b-41d4-a716-446655440000"
    }
  },
  "id": 2
}
```

---

## Streaming Notifications

During `kaiak/generate_fix` processing, the server sends real-time notifications **concurrently** as they are generated (not buffered). All notifications have no `id` field (per JSON-RPC 2.0 specification for notifications).

The server uses `tokio::select!` to stream notifications immediately while request processing continues, ensuring clients receive progress updates in real-time.

### Progress Notification

**Method:** `kaiak/generateFix/progress`

```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/generateFix/progress",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "stage": "analyzing",
    "progress": 25,
    "timestamp": "2025-12-25T10:35:05Z",
    "data": {
      "message": "Analyzing code incidents and generating context"
    }
  }
}
```

### Progress Fields

| Field | Type | Description |
|-------|------|-------------|
| `session_id` | string | Session this notification belongs to |
| `stage` | string | Current processing stage |
| `progress` | number | Percentage complete (0-100) |
| `timestamp` | string | ISO 8601 timestamp |
| `data` | object | Optional stage-specific data |

### Processing Stages

| Stage | Description |
|-------|-------------|
| `started` | Processing has begun |
| `analyzing` | Analyzing incidents |
| `generating` | Generating fixes with AI |
| `completed` | Processing finished |
| `failed` | Processing encountered an error |

---

## Error Codes

### Standard JSON-RPC Errors

| Code | Name | Description |
|------|------|-------------|
| -32700 | Parse error | Invalid JSON |
| -32600 | Invalid request | Invalid JSON-RPC structure |
| -32601 | Method not found | Unknown method |
| -32602 | Invalid params | Invalid parameters |
| -32603 | Internal error | Server error |

### Kaiak-Specific Error Codes

| Code | Name | Description |
|------|------|-------------|
| -32001 | Transport error | Communication failure |
| -32003 | Session not found | Session does not exist |
| -32010 | Agent error | AI agent failure |
| -32011 | Workspace error | Workspace access issue |
| -32012 | Agent initialization | Failed to initialize agent |
| -32013 | Session in use | Concurrent access blocked |
| -32014 | Configuration error | Invalid configuration |
| -32015 | Resource exhausted | System resources exceeded |
| -32016 | I/O error | File system error |
| -32017 | Serialization error | JSON encoding/decoding failure |

---

## Server Configuration

Configuration is set at server startup, not via API calls.

### Configuration Sources (in order of precedence)

1. **CLI inline JSON**: `--config-json '{...}'`
2. **CLI config file**: `--config-path /path/to/config.json`
3. **User config**: `~/.kaiak/server.conf`
4. **Default values**

### Example Configuration

```json
{
  "model": {
    "provider": "openai",
    "model_id": "gpt-4",
    "temperature": 0.1
  },
  "workspace": {
    "working_dir": "/home/user/project"
  },
  "init_config": {
    "transport": "socket",
    "socket_path": "/tmp/kaiak.sock"
  }
}
```

---

## CLI Client

Kaiak includes a CLI client for interacting with servers over Unix sockets.

### Commands

```bash
# Start server
kaiak serve --socket /tmp/kaiak.sock

# Connect to server
kaiak connect /tmp/kaiak.sock

# Generate fixes (with notification streaming)
kaiak generate-fix --params-file request.json
kaiak generate-fix --params-json '{"session_id": "...", ...}'

# Delete session
kaiak delete-session <session_id>

# Disconnect
kaiak disconnect
```

### Connection State

The CLI stores connection state in `~/.kaiak/connection`. Once connected, subsequent commands automatically use the stored socket path.

---

## Complete Workflow Example

### 1. Start Server

```bash
kaiak serve --socket /tmp/kaiak.sock --config-json '{
  "model": {"provider": "openai", "model_id": "gpt-4"}
}'
```

### 2. Connect Client

```bash
kaiak connect /tmp/kaiak.sock
```

### 3. Generate Fixes (session_id is optional)

```bash
kaiak generate-fix --params-json '{
  "incidents": [{
    "id": "dep-1",
    "rule_id": "deprecated-api",
    "message": "Replace Collections.sort() with List.sort()"
  }],
  "agent_config": {
    "workspace": {"working_dir": "/home/user/project"},
    "model": {"provider": "openai", "model_id": "gpt-4"}
  }
}'
```

**Output (streaming notifications arrive in real-time):**
```
[kaiak/generateFix/progress] stage=started progress=0%
[kaiak/generateFix/progress] stage=analyzing progress=25%
[kaiak/generateFix/progress] stage=generating progress=50%
[kaiak/generateFix/progress] stage=completed progress=100%

--- Final Result ---
{
  "request_id": "req-abc123",
  "session_id": "goose-generated-session-id",
  "created_at": "2025-12-29T10:00:00Z"
}
```

### 4. Clean Up (use session_id from response)

```bash
kaiak delete-session goose-generated-session-id
kaiak disconnect
```

---

## Integration Notes

- Use LSP-compatible client libraries for transport handling
- Handle streaming notifications asynchronously (they arrive concurrently during processing)
- Implement error recovery for network interruptions
- Session IDs are **optional** - server creates new sessions if not provided
- Use the `session_id` from responses to continue sessions or clean up
- Only one client can use a session at a time
- Server can receive notifications from clients (messages without `id` field)

---

## Programmatic Client Example (Rust)

```rust
use kaiak::client::{JsonRpcClient, ClientRequest, JsonRpcNotification};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = JsonRpcClient::new("/tmp/kaiak.sock".to_string());

    // session_id is optional - omit to have server create a new session
    let params = serde_json::json!({
        "incidents": [{"id": "1", "rule_id": "test", "message": "Test"}],
        "agent_config": {
            "workspace": {"working_dir": "/tmp/project"},
            "model": {"provider": "openai", "model_id": "gpt-4"}
        }
    });

    let request = ClientRequest::new("kaiak/generate_fix".to_string(), params);

    // Notifications arrive in real-time during processing
    let result = client.call(request, |notification: JsonRpcNotification| {
        if let Some(params) = &notification.params {
            println!("Progress: {}", params);
        }
    }).await?;

    // Extract session_id from response for later use (e.g., delete_session)
    let session_id = result["session_id"].as_str().unwrap();
    println!("Session ID: {}", session_id);
    println!("Result: {}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
```

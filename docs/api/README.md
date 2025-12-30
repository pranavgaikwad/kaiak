# Kaiak API Documentation

Complete API reference for the Kaiak Migration Server JSON-RPC interface.

## Overview

Kaiak provides a simplified JSON-RPC 2.0 API over LSP transport for all client communication. The API is designed for real-time streaming workflows with comprehensive error handling.

### API Methods

| Method | Description |
|--------|-------------|
| `kaiak/generate_fix` | Generate fixes for migration incidents (streaming) |
| `kaiak/delete_session` | Clean up agent session |

### Transport

All messages use LSP message framing:
```
Content-Length: {byte_length}\r\n
\r\n
{json_message}
```

Supported transports:
- **stdio**: Messages over stdin/stdout (for IDE integration)
- **Unix domain socket**: IPC communication (for CLI and external clients)

## Quick Start

### 1. Start the Server

```bash
# With default config from ~/.kaiak/server.conf
kaiak serve --socket /tmp/kaiak.sock

# Or with inline JSON config
kaiak serve --socket /tmp/kaiak.sock --config-json '{"model":{"provider":"openai"}}'
```

### 2. Connect a Client

```bash
# Connect to the server
kaiak connect /tmp/kaiak.sock

# Generate a fix
kaiak generate-fix --params-json '{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "incidents": [...],
  "agent_config": {...}
}'

# Clean up
kaiak delete-session 550e8400-e29b-41d4-a716-446655440000

# Disconnect
kaiak disconnect
```

## API Methods

### kaiak/generate_fix

Process migration incidents with the Goose AI agent. Streams progress notifications during execution.

**Request:**
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
      "target_technology": "Java 17"
    },
    "agent_config": {
      "workspace": {
        "working_dir": "/path/to/project"
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

**Parameters:**
- `session_id` (string, required): Client-generated UUID for the session
- `incidents` (array, required): Migration incidents to process (1-1000)
- `migration_context` (object, optional): Additional context for the migration
- `agent_config` (object, required): Agent configuration including workspace and model settings

**Response (Success):**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "request_id": "req-550e8400-e29b-41d4-a716-446655440001",
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "created_at": "2025-12-25T10:35:45Z"
  },
  "id": 1
}
```

### kaiak/delete_session

Delete an agent session and clean up resources.

**Request:**
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

**Response (Success):**
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

## Streaming Notifications

During `kaiak/generate_fix` processing, the server sends real-time notifications. Notifications have no `id` field (per JSON-RPC 2.0 spec).

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
      "message": "Analyzing migration incidents..."
    }
  }
}
```

**Fields:**
- `session_id`: The session this notification belongs to
- `stage`: Current processing stage (e.g., "started", "analyzing", "generating", "completed")
- `progress`: Percentage complete (0-100)
- `timestamp`: ISO 8601 timestamp
- `data`: Optional stage-specific data

## Error Codes

### Standard JSON-RPC Errors
| Code | Description |
|------|-------------|
| -32700 | Parse error |
| -32600 | Invalid request |
| -32601 | Method not found |
| -32602 | Invalid params |
| -32603 | Internal error |

### Kaiak-Specific Errors
| Code | Description |
|------|-------------|
| -32003 | Session not found |
| -32010 | Agent error |
| -32011 | Workspace error |
| -32012 | Agent initialization failed |
| -32013 | Session in use |
| -32014 | Configuration error |
| -32015 | Resource exhausted |

## Configuration

Server configuration is provided via:
1. CLI arguments (`--config-json` or `--config-path`)
2. User config file (`~/.kaiak/server.conf`)
3. Default values

Configuration is **not** set via API calls—it's determined at server startup.

## Client Integration

### Using the CLI

```bash
# Connect to a running server
kaiak connect /tmp/kaiak.sock

# Check connection (implicitly used by other commands)
kaiak generate-fix --params-file request.json

# Disconnect
kaiak disconnect
```

### Programmatic Access

Use any JSON-RPC 2.0 client with LSP framing support. The client should:

1. Connect to the Unix socket
2. Send requests with `Content-Length` headers
3. Handle interleaved notifications and responses
4. Parse notifications (messages without `id`) separately from responses

Example with the Rust client:

```rust
use kaiak::client::{JsonRpcClient, ClientRequest, JsonRpcNotification};

let client = JsonRpcClient::new("/tmp/kaiak.sock".to_string());

let params = serde_json::json!({
    "session_id": "my-session-id",
    "incidents": [...],
    "agent_config": {...}
});

let request = ClientRequest::new("kaiak/generate_fix".to_string(), params);

// Call with notification handler
let result = client.call(request, |notification: JsonRpcNotification| {
    println!("Progress: {:?}", notification.params);
}).await?;
```

## Session Lifecycle

1. **Start Server**: Configure via CLI or config file
2. **Connect Client**: Establish Unix socket connection
3. **Generate Fix**: Call `kaiak/generate_fix` with session ID
4. **Monitor Progress**: Handle streaming notifications
5. **Receive Result**: Get final response when processing completes
6. **Cleanup**: Call `kaiak/delete_session` when done

## Complete Documentation

For additional details, see:
- [Main README](../../README.md)
- [Integration Tests](../../tests/integration/)

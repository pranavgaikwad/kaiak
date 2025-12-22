# JSON-RPC API Contract: Kaiak Migration Server

**Protocol**: JSON-RPC 2.0 over LSP transport (Content-Length headers + JSON)
**Transport**: Stdio (primary), Unix domain sockets (fallback)
**Created**: 2025-12-22

## Transport Format

All messages follow LSP message format:
```
Content-Length: {byte_length}\r\n
\r\n
{json_payload}
```

## Core Methods

### 1. Session Management

#### `kaiak/session/create`

Create a new AI session for fix generation.

**Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/session/create",
  "params": {
    "workspace_path": "/absolute/path/to/workspace",
    "session_name": "migration-session-1",
    "configuration": {
      "provider": "openai",
      "model": "gpt-4",
      "timeout": 300,
      "max_turns": 50
    }
  },
  "id": 1
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "status": "created",
    "created_at": "2025-12-22T10:30:00Z"
  },
  "id": 1
}
```

**Errors**:
- `-32602`: Invalid params (invalid workspace path, configuration)
- `-32001`: Session creation failed
- `-32002`: Workspace access denied

---

#### `kaiak/session/terminate`

Terminate an existing AI session and cleanup resources.

**Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/session/terminate",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000"
  },
  "id": 2
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "status": "terminated",
    "message_count": 15,
    "terminated_at": "2025-12-22T11:00:00Z"
  },
  "id": 2
}
```

**Errors**:
- `-32602`: Invalid session ID
- `-32003`: Session not found
- `-32004`: Session already terminated

---

### 2. Fix Generation

#### `kaiak/fix/generate`

Generate fixes for one or more incidents using AI agent.

**Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/fix/generate",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "incidents": [
      {
        "id": "incident-1",
        "rule_id": "deprecated-api-usage",
        "file_path": "src/main.rs",
        "line_number": 42,
        "severity": "warning",
        "description": "Use of deprecated API",
        "message": "Function `old_method()` is deprecated, use `new_method()` instead",
        "category": "deprecated-api",
        "metadata": {
          "deprecated_since": "1.5.0",
          "replacement": "new_method()"
        }
      }
    ],
    "migration_context": {
      "target_version": "2.0.0",
      "migration_guide_url": "https://example.com/migration-guide"
    }
  },
  "id": 3
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "request_id": "req-550e8400-e29b-41d4-a716-446655440001",
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "status": "processing",
    "incident_count": 1,
    "created_at": "2025-12-22T10:35:00Z"
  },
  "id": 3
}
```

**Errors**:
- `-32602`: Invalid incidents data
- `-32003`: Session not found
- `-32005`: Session not ready
- `-32006`: Agent initialization failed

---

#### `kaiak/fix/cancel`

Cancel an in-progress fix generation request.

**Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/fix/cancel",
  "params": {
    "request_id": "req-550e8400-e29b-41d4-a716-446655440001"
  },
  "id": 4
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "request_id": "req-550e8400-e29b-41d4-a716-446655440001",
    "status": "cancelled",
    "cancelled_at": "2025-12-22T10:40:00Z"
  },
  "id": 4
}
```

**Errors**:
- `-32602`: Invalid request ID
- `-32007`: Request not found
- `-32008`: Request already completed

---

### 3. User Interaction

#### `kaiak/interaction/respond`

Respond to a user interaction request (approval, choice, input).

**Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/interaction/respond",
  "params": {
    "interaction_id": "int-550e8400-e29b-41d4-a716-446655440002",
    "response_type": "approval",
    "response_data": {
      "approved": true
    }
  },
  "id": 5
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "interaction_id": "int-550e8400-e29b-41d4-a716-446655440002",
    "status": "processed",
    "responded_at": "2025-12-22T10:45:00Z"
  },
  "id": 5
}
```

**Errors**:
- `-32602`: Invalid interaction ID or response data
- `-32009`: Interaction not found
- `-32010`: Interaction already responded
- `-32011`: Response validation failed

---

### 4. Session Status

#### `kaiak/session/status`

Get current status of an AI session.

**Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/session/status",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000"
  },
  "id": 6
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "status": "processing",
    "active_request_id": "req-550e8400-e29b-41d4-a716-446655440001",
    "message_count": 8,
    "error_count": 0,
    "created_at": "2025-12-22T10:30:00Z",
    "updated_at": "2025-12-22T10:44:00Z"
  },
  "id": 6
}
```

**Errors**:
- `-32602`: Invalid session ID
- `-32003`: Session not found

---

## Streaming Notifications

### `kaiak/stream/progress`

Progress updates during fix generation processing.

**Notification**:
```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/stream/progress",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "request_id": "req-550e8400-e29b-41d4-a716-446655440001",
    "message_id": "msg-1",
    "timestamp": "2025-12-22T10:36:00Z",
    "content": {
      "percentage": 25,
      "phase": "analyzing_incidents",
      "description": "Analyzing code incidents and generating context"
    }
  }
}
```

---

### `kaiak/stream/ai_response`

AI-generated response chunks for streaming display.

**Notification**:
```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/stream/ai_response",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "request_id": "req-550e8400-e29b-41d4-a716-446655440001",
    "message_id": "msg-2",
    "timestamp": "2025-12-22T10:37:00Z",
    "content": {
      "text": "I've analyzed the deprecated API usage in your code. Here's what I found...",
      "partial": false,
      "confidence": 0.95
    }
  }
}
```

---

### `kaiak/stream/tool_call`

Tool execution status and results.

**Notification**:
```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/stream/tool_call",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "request_id": "req-550e8400-e29b-41d4-a716-446655440001",
    "message_id": "msg-3",
    "timestamp": "2025-12-22T10:38:00Z",
    "content": {
      "tool_name": "file_reader",
      "operation": "complete",
      "parameters": {
        "file_path": "src/main.rs",
        "line_range": [40, 50]
      },
      "result": {
        "success": true,
        "data": "// File content excerpt\nfn main() {\n    old_method(); // This line needs updating\n}"
      }
    }
  }
}
```

---

### `kaiak/stream/thinking`

AI reasoning and thinking process for transparency.

**Notification**:
```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/stream/thinking",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "request_id": "req-550e8400-e29b-41d4-a716-446655440001",
    "message_id": "msg-4",
    "timestamp": "2025-12-22T10:38:30Z",
    "content": {
      "text": "The user has a deprecated API call on line 42. I need to check the migration guide to find the correct replacement method. Let me read the file first to understand the context better."
    }
  }
}
```

---

### `kaiak/stream/user_interaction`

Request for user input or approval.

**Notification**:
```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/stream/user_interaction",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "request_id": "req-550e8400-e29b-41d4-a716-446655440001",
    "message_id": "msg-5",
    "timestamp": "2025-12-22T10:39:00Z",
    "interaction_id": "int-550e8400-e29b-41d4-a716-446655440002",
    "content": {
      "interaction_type": "approval",
      "prompt": "I found a fix for the deprecated API usage. Should I apply this change?",
      "proposal_id": "prop-550e8400-e29b-41d4-a716-446655440003",
      "timeout": 300
    }
  }
}
```

---

### `kaiak/stream/file_modification`

Proposed file modifications requiring approval.

**Notification**:
```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/stream/file_modification",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "request_id": "req-550e8400-e29b-41d4-a716-446655440001",
    "message_id": "msg-6",
    "timestamp": "2025-12-22T10:39:00Z",
    "proposal_id": "prop-550e8400-e29b-41d4-a716-446655440003",
    "content": {
      "file_path": "src/main.rs",
      "change_type": "edit",
      "description": "Replace deprecated old_method() with new_method()",
      "original_content": "fn main() {\n    old_method();\n}",
      "proposed_content": "fn main() {\n    new_method();\n}",
      "confidence": 0.98
    }
  }
}
```

---

### `kaiak/stream/error`

Error notifications during processing.

**Notification**:
```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/stream/error",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "request_id": "req-550e8400-e29b-41d4-a716-446655440001",
    "message_id": "msg-7",
    "timestamp": "2025-12-22T10:39:30Z",
    "content": {
      "error_code": "TOOL_EXECUTION_FAILED",
      "message": "Failed to read file src/main.rs",
      "details": "Permission denied: insufficient file access permissions",
      "recoverable": false
    }
  }
}
```

---

### `kaiak/stream/system`

System status and lifecycle events.

**Notification**:
```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/stream/system",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "message_id": "msg-8",
    "timestamp": "2025-12-22T10:40:00Z",
    "content": {
      "event": "request_completed",
      "request_id": "req-550e8400-e29b-41d4-a716-446655440001",
      "status": "success",
      "summary": {
        "incidents_processed": 1,
        "proposals_generated": 1,
        "user_interactions": 1,
        "processing_time_ms": 300000
      }
    }
  }
}
```

---

## Error Codes

### Standard JSON-RPC Errors
- `-32700`: Parse error
- `-32600`: Invalid Request
- `-32601`: Method not found
- `-32602`: Invalid params
- `-32603`: Internal error

### Kaiak-Specific Errors
- `-32001`: Session creation failed
- `-32002`: Workspace access denied
- `-32003`: Session not found
- `-32004`: Session already terminated
- `-32005`: Session not ready
- `-32006`: Agent initialization failed
- `-32007`: Request not found
- `-32008`: Request already completed
- `-32009`: Interaction not found
- `-32010`: Interaction already responded
- `-32011`: Response validation failed
- `-32012`: File modification failed
- `-32013`: Tool execution timeout
- `-32014`: Configuration error
- `-32015`: Resource exhausted

## Message Size Limits

- **Request payload**: Maximum 10MB
- **Incident array**: Maximum 1000 incidents per request
- **File content**: Maximum 1MB per file
- **Stream message**: Maximum 1MB per message

## Authentication

For initial implementation: No authentication required (process isolation provides security)
Future: Token-based authentication for multi-client scenarios

## Rate Limiting

- **Fix generation requests**: Maximum 10 concurrent per session
- **User interactions**: No limit (required for workflow completion)
- **Status requests**: Maximum 100 per minute per session

## Versioning

API version is embedded in method names. Future versions would use:
- `kaiak/v2/session/create`
- Backward compatibility maintained for at least one major version
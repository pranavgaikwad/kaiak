# Kaiak API Documentation

Complete API reference for the Kaiak Migration Server JSON-RPC interface.

## Overview

Kaiak provides a JSON-RPC 2.0 API over LSP transport (Content-Length headers + JSON) for all client communication. The API is designed for real-time streaming workflows with comprehensive error handling.

### Transport Format

All messages follow the LSP message format:

```
Content-Length: {byte_length}\r\n
\r\n
{json_payload}
```

### API Versioning

Current API version: **v1**
- Version is embedded in method names (e.g., `kaiak/session/create`)
- Backward compatibility maintained for one major version
- Future versions will use `kaiak/v2/session/create` format

## Session Management API

### Create Session

Create a new AI session for fix generation workflows.

**Method**: `kaiak/session/create`

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

**Parameters**:
- `workspace_path` (string, required): Absolute path to workspace directory
- `session_name` (string, optional): Human-readable session identifier
- `configuration` (object, optional): Session-specific AI configuration
  - `provider` (string): AI provider ("openai", "anthropic")
  - `model` (string): Model name ("gpt-4", "claude-3-opus", etc.)
  - `timeout` (number): Request timeout in seconds
  - `max_turns` (number): Maximum conversation turns

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

### Terminate Session

Terminate an existing AI session and cleanup resources.

**Method**: `kaiak/session/terminate`

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

### Session Status

Get current status of an AI session.

**Method**: `kaiak/session/status`

**Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/session/status",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000"
  },
  "id": 3
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
  "id": 3
}
```

## Fix Generation API

### Generate Fixes

Generate fixes for one or more code incidents using AI agent.

**Method**: `kaiak/fix/generate`

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
  "id": 4
}
```

**Parameters**:
- `session_id` (string, required): Active session identifier
- `incidents` (array, required): Array of code incidents to fix
  - `id` (string): Unique incident identifier
  - `rule_id` (string): Static analysis rule that triggered
  - `file_path` (string): Relative path from workspace root
  - `line_number` (number): Line number where issue occurs
  - `severity` (string): "error", "warning", or "info"
  - `description` (string): Human-readable description
  - `message` (string): Detailed issue explanation
  - `category` (string): Issue category classification
  - `metadata` (object, optional): Additional tool-specific data
- `migration_context` (object, optional): Additional migration context

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
  "id": 4
}
```

### Cancel Fix Generation

Cancel an in-progress fix generation request.

**Method**: `kaiak/fix/cancel`

**Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/fix/cancel",
  "params": {
    "request_id": "req-550e8400-e29b-41d4-a716-446655440001"
  },
  "id": 5
}
```

## User Interaction API

### Respond to Interaction

Respond to a user interaction request (approval, choice, input).

**Method**: `kaiak/interaction/respond`

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
  "id": 6
}
```

**Response Data Types**:

**Approval Response**:
```json
{
  "approved": true  // or false
}
```

**Choice Response**:
```json
{
  "selected_indices": [0, 2]  // Array of selected option indices
}
```

**Input Response**:
```json
{
  "text": "User provided input text"
}
```

## Streaming Notifications

Kaiak provides real-time streaming notifications for various events during processing. All notifications are sent as JSON-RPC notifications (no response expected).

### Progress Updates

**Notification**: `kaiak/stream/progress`

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

### AI Response Streaming

**Notification**: `kaiak/stream/ai_response`

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
      "text": "I've analyzed the deprecated API usage...",
      "partial": false,
      "confidence": 0.95
    }
  }
}
```

### Tool Execution

**Notification**: `kaiak/stream/tool_call`

```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/stream/tool_call",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
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
        "data": "// File content excerpt..."
      }
    }
  }
}
```

### User Interactions

**Notification**: `kaiak/stream/user_interaction`

```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/stream/user_interaction",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "message_id": "msg-4",
    "timestamp": "2025-12-22T10:39:00Z",
    "interaction_id": "int-550e8400-e29b-41d4-a716-446655440002",
    "content": {
      "interaction_type": "approval",
      "prompt": "Should I apply this change to src/main.rs?",
      "proposal_id": "prop-550e8400-e29b-41d4-a716-446655440003",
      "timeout": 300
    }
  }
}
```

### File Modification Proposals

**Notification**: `kaiak/stream/file_modification`

```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/stream/file_modification",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "message_id": "msg-5",
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

## Error Handling

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

### Error Response Format

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32003,
    "message": "Session not found",
    "data": {
      "session_id": "invalid-session-id",
      "suggestion": "Create a new session with kaiak/session/create"
    }
  },
  "id": 1
}
```

## Rate Limits

- **Fix generation requests**: Maximum 10 concurrent per session
- **User interactions**: No limit (required for workflow completion)
- **Status requests**: Maximum 100 per minute per session

## Message Size Limits

- **Request payload**: Maximum 10MB
- **Incident array**: Maximum 1000 incidents per request
- **File content**: Maximum 1MB per file
- **Stream message**: Maximum 1MB per message

## Authentication

Current implementation: No authentication required (process isolation provides security)

Future versions will support token-based authentication for multi-client scenarios.

## SDK and Client Libraries

### Official Clients

- **TypeScript/JavaScript**: `@kaiak/client` (coming soon)
- **Python**: `kaiak-python` (coming soon)
- **Go**: `kaiak-go` (coming soon)

### Community Clients

- Submit your client library via GitHub Issues for inclusion

## Examples

### Complete Fix Generation Workflow

```javascript
// 1. Create session
const createSession = {
  jsonrpc: "2.0",
  method: "kaiak/session/create",
  params: {
    workspace_path: "/path/to/project",
    session_name: "fix-deprecated-apis"
  },
  id: 1
};

// 2. Generate fixes
const generateFixes = {
  jsonrpc: "2.0",
  method: "kaiak/fix/generate",
  params: {
    session_id: "session-id-from-step-1",
    incidents: [
      {
        id: "dep-1",
        rule_id: "deprecated-api",
        file_path: "src/main.rs",
        line_number: 42,
        severity: "warning",
        description: "Deprecated function usage",
        message: "replace old_func() with new_func()"
      }
    ]
  },
  id: 2
};

// 3. Handle streaming notifications
// Listen for kaiak/stream/* notifications

// 4. Respond to user interactions
const approveChange = {
  jsonrpc: "2.0",
  method: "kaiak/interaction/respond",
  params: {
    interaction_id: "interaction-id-from-notification",
    response_type: "approval",
    response_data: { approved: true }
  },
  id: 3
};
```

For more examples, see the [Examples Directory](../examples/) and [Integration Tests](../../tests/integration/).
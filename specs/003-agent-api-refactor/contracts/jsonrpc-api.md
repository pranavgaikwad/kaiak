# JSON-RPC API Contract: Agent API Refactor for Goose Integration

**Date**: 2025-12-24
**Feature**: 003-agent-api-refactor
**Protocol**: JSON-RPC 2.0
**Transport**: stdio, Unix domain socket

## Overview

This document defines the JSON-RPC 2.0 API contract for the refactored Kaiak agent system. The API provides three endpoints for agent configuration, incident processing, and session management while maintaining compatibility with existing JSON-RPC infrastructure.

## Transport Layer

### Supported Transports
- **stdio** (default): Messages over stdin/stdout
- **Unix domain socket**: Enterprise-safe IPC communication

### Message Framing
All messages use standard LSP (Language Server Protocol) framing:
```
Content-Length: <length>\r\n
\r\n
<JSON-RPC message>
```

## API Methods

### 1. configure

Configure agent settings, workspace, model provider, and tool permissions.

**Method**: `workspace/executeCommand`
**Command**: `kaiak/configure`

**Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "workspace/executeCommand",
  "params": {
    "command": "kaiak/configure",
    "arguments": [{
      "configuration": {
        "workspace": {
          "working_dir": "/path/to/project",
          "include_patterns": ["**/*.java", "**/*.xml"],
          "exclude_patterns": [".git/**", "target/**"]
        },
        "model": {
          "provider": "databricks",
          "model": "databricks-meta-llama-3-1-405b-instruct",
          "temperature": 0.1,
          "max_tokens": 4096
        },
        "tools": {
          "enabled_extensions": ["developer", "todo", "extensionmanager"],
          "custom_tools": [],
          "planning_mode": true,
          "max_tool_calls": 10
        },
        "session": {
          "max_turns": 1000,
          "retry_config": null
        },
        "permissions": {
          "tool_permissions": {
            "read_file": "allow",
            "write_file": "approve",
            "shell_command": "deny",
            "web_search": "allow"
          }
        }
      },
      "reset_existing": false
    }]
  },
  "id": 1
}
```

**Response (Success)**:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "status": "success",
    "message": "Agent configured successfully",
    "configuration_applied": {
      "workspace": { /* ... applied config ... */ },
      "model": { /* ... applied config ... */ },
      "tools": { /* ... applied config ... */ },
      "session": { /* ... applied config ... */ },
      "permissions": { /* ... applied config ... */ }
    },
    "warnings": [],
    "timestamp": "2025-12-24T10:30:00Z"
  },
  "id": 1
}
```

**Response (Error)**:
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32014,
    "message": "Configuration error: Invalid workspace directory",
    "data": {
      "field": "workspace.working_dir",
      "details": "Directory '/invalid/path' does not exist"
    }
  },
  "id": 1
}
```

### 2. generate_fix

Process migration incidents with Goose agent. Creates or reuses session, streams progress updates, waits for completion.

**Method**: `workspace/executeCommand`
**Command**: `kaiak/generate_fix`

**Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "workspace/executeCommand",
  "params": {
    "command": "kaiak/generate_fix",
    "arguments": [{
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
        "migration_hints": ["Use java.util.Base64 instead of DatatypeConverter"],
        "constraints": ["Maintain backward compatibility"],
        "preferences": {
          "code_style": "google",
          "test_generation": true
        }
      },
      "options": {
        "auto_apply_safe_fixes": false,
        "max_processing_time": 300,
        "parallel_processing": false,
        "include_explanations": true
      }
    }]
  },
  "id": 2
}
```

**Response (Success)**:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "request_id": "req-550e8400-e29b-41d4-a716-446655440001",
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "status": "completed",
    "incident_count": 1,
    "completed_at": "2025-12-24T10:35:45Z"
  },
  "id": 2
}
```

**Response (Error - Session Not Found)**:
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32003,
    "message": "Session not found",
    "data": {
      "session_id": "nonexistent-session-id"
    }
  },
  "id": 2
}
```

**Response (Error - Session In Use)**:
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32016,
    "message": "Session is currently in use by another client",
    "data": {
      "session_id": "550e8400-e29b-41d4-a716-446655440000",
      "in_use_since": "2025-12-24T10:30:00Z"
    }
  },
  "id": 2
}
```

### 3. delete_session

Delete agent session and cleanup resources.

**Method**: `workspace/executeCommand`
**Command**: `kaiak/delete_session`

**Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "workspace/executeCommand",
  "params": {
    "command": "kaiak/delete_session",
    "arguments": [{
      "session_id": "550e8400-e29b-41d4-a716-446655440000",
      "force": false,
      "cleanup_files": true
    }]
  },
  "id": 3
}
```

**Response (Success)**:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "status": "deleted",
    "cleanup_summary": {
      "session_removed": true,
      "messages_cleaned": 15,
      "temp_files_removed": 3,
      "errors": []
    },
    "deleted_at": "2025-12-24T10:40:00Z"
  },
  "id": 3
}
```

**Response (Error)**:
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
  "id": 3
}
```

## Streaming Notifications

During `generate_fix` processing, the server sends real-time notifications to the client. All notifications are sent without an `id` field.

### Progress Notifications

**Method**: `kaiak/stream/progress`

```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/stream/progress",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "request_id": "req-550e8400-e29b-41d4-a716-446655440001",
    "message_id": "msg-1",
    "timestamp": "2025-12-24T10:35:05Z",
    "content": {
      "percentage": 25,
      "phase": "analyzing_incidents",
      "description": "Analyzing code incidents and generating context",
      "current_step": "Processing incident 1 of 4",
      "total_steps": 4
    }
  }
}
```

### AI Response Notifications

**Method**: `kaiak/stream/ai_response`

```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/stream/ai_response",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "request_id": "req-550e8400-e29b-41d4-a716-446655440001",
    "message_id": "msg-2",
    "timestamp": "2025-12-24T10:35:10Z",
    "content": {
      "text": "I'll help you migrate from javax.xml.bind.DatatypeConverter to java.util.Base64.",
      "partial": false,
      "confidence": 0.95,
      "tokens": 18
    }
  }
}
```

### Tool Call Notifications

**Method**: `kaiak/stream/tool_call`

```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/stream/tool_call",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "request_id": "req-550e8400-e29b-41d4-a716-446655440001",
    "message_id": "msg-3",
    "timestamp": "2025-12-24T10:35:15Z",
    "content": {
      "tool_name": "read_file",
      "operation": "read",
      "parameters": {
        "file_path": "src/main/java/com/example/DataConverter.java"
      },
      "status": "completed",
      "result": {
        "success": true,
        "output": "File content here...",
        "error": null,
        "execution_time": 50
      }
    }
  }
}
```

### User Interaction Notifications

**Method**: `kaiak/stream/user_interaction`

```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/stream/user_interaction",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "request_id": "req-550e8400-e29b-41d4-a716-446655440001",
    "message_id": "msg-4",
    "timestamp": "2025-12-24T10:35:20Z",
    "content": {
      "interaction_id": "interaction-1",
      "interaction_type": "file_approval",
      "prompt": "Apply this fix to DataConverter.java?",
      "options": ["approve", "deny", "edit"],
      "default_response": "approve",
      "timeout": 30
    }
  }
}
```

### File Modification Notifications

**Method**: `kaiak/stream/file_modification`

```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/stream/file_modification",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "request_id": "req-550e8400-e29b-41d4-a716-446655440001",
    "message_id": "msg-5",
    "timestamp": "2025-12-24T10:35:25Z",
    "content": {
      "proposal_id": "proposal-1",
      "file_path": "src/main/java/com/example/DataConverter.java",
      "operation": "modify",
      "diff": "@@ -15,7 +15,7 @@\n-import javax.xml.bind.DatatypeConverter;\n+import java.util.Base64;",
      "risk_level": "low",
      "requires_approval": true
    }
  }
}
```

### Error Notifications

**Method**: `kaiak/stream/error`

```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/stream/error",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "request_id": "req-550e8400-e29b-41d4-a716-446655440001",
    "message_id": "msg-6",
    "timestamp": "2025-12-24T10:35:30Z",
    "content": {
      "error_code": "TOOL_EXECUTION_FAILED",
      "message": "Failed to read file: Permission denied",
      "details": "File /protected/config.xml requires elevated permissions",
      "recoverable": true,
      "suggested_action": "Check file permissions or run with elevated privileges"
    }
  }
}
```

### System Notifications

**Method**: `kaiak/stream/system`

```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/stream/system",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "request_id": "req-550e8400-e29b-41d4-a716-446655440001",
    "message_id": "msg-7",
    "timestamp": "2025-12-24T10:35:35Z",
    "content": {
      "message": "Agent processing completed successfully",
      "level": "info",
      "component": "agent_manager"
    }
  }
}
```

## Error Codes

### Standard JSON-RPC Errors
- `-32700`: Parse error
- `-32600`: Invalid request
- `-32601`: Method not found
- `-32602`: Invalid params
- `-32603`: Internal error

### Kaiak-Specific Error Codes
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
- `-32016`: Session in use (NEW - for concurrent access blocking)

## User Interaction Response Pattern

When the server sends a `kaiak/stream/user_interaction` notification, the client must respond using the existing interaction response mechanism. The specific method for responding is not part of the three-endpoint API but maintains compatibility with existing infrastructure.

## Session Lifecycle

1. **Session Creation**: Automatic when `generate_fix` is called with unknown session_id
2. **Session Processing**: Agent processes incidents, streams notifications
3. **Session Completion**: Agent finishes, returns final response
4. **Session Cleanup**: Optional cleanup via `delete_session`

## Concurrent Access Control

- Only one client can use a session at a time
- Attempts to use an active session return error `-32016`
- Sessions automatically release after completion or timeout

## Example Complete Flow

### 1. Configure Agent
```json
// Request
{
  "jsonrpc": "2.0",
  "method": "workspace/executeCommand",
  "params": {
    "command": "kaiak/configure",
    "arguments": [{ /* configuration */ }]
  },
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": { "status": "success", /* ... */ },
  "id": 1
}
```

### 2. Process Incidents
```json
// Request
{
  "jsonrpc": "2.0",
  "method": "workspace/executeCommand",
  "params": {
    "command": "kaiak/generate_fix",
    "arguments": [{
      "session_id": "uuid-here",
      "incidents": [/* incidents */]
    }]
  },
  "id": 2
}

// Streaming notifications (no id field)
{ "jsonrpc": "2.0", "method": "kaiak/stream/progress", /* ... */ }
{ "jsonrpc": "2.0", "method": "kaiak/stream/ai_response", /* ... */ }
{ "jsonrpc": "2.0", "method": "kaiak/stream/tool_call", /* ... */ }
{ "jsonrpc": "2.0", "method": "kaiak/stream/user_interaction", /* ... */ }
// ... more notifications ...

// Final response (when agent completes)
{
  "jsonrpc": "2.0",
  "result": {
    "status": "completed",
    "completed_at": "2025-12-24T10:35:45Z",
    /* ... */
  },
  "id": 2
}
```

### 3. Cleanup Session
```json
// Request
{
  "jsonrpc": "2.0",
  "method": "workspace/executeCommand",
  "params": {
    "command": "kaiak/delete_session",
    "arguments": [{ "session_id": "uuid-here" }]
  },
  "id": 3
}

// Response
{
  "jsonrpc": "2.0",
  "result": { "status": "deleted", /* ... */ },
  "id": 3
}
```

This contract maintains full compatibility with existing JSON-RPC infrastructure while providing a simplified three-endpoint API for agent operations.
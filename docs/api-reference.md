# JSON-RPC API Reference: Kaiak Agent API

**Version**: 1.0
**Date**: 2025-12-25
**Protocol**: JSON-RPC 2.0
**Transport**: LSP-compatible (Content-Length headers + JSON)

## Overview

Kaiak provides a simplified JSON-RPC API for agent operations. All communication follows LSP message framing and supports real-time streaming notifications.

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
- **Unix domain socket**: Enterprise-safe IPC communication

## API Endpoints

Kaiak exposes the following methods via the `workspace/executeCommand` pattern:

1. **kaiak/configure** - Configure agent workspace and settings
2. **kaiak/generate_fix** - Generate fixes for migration incidents
3. **kaiak/delete_session** - Clean up agent session

## 1. kaiak/configure

Configure agent settings, workspace, model provider, and tool permissions for a session.

### Request

**Method**: `workspace/executeCommand`
**Command**: `kaiak/configure`

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

### Parameters

- **configuration** (object, required): Complete agent configuration
  - **workspace** (object): Workspace configuration
    - **working_dir** (string): Absolute path to workspace directory
    - **include_patterns** (array): Glob patterns for included files
    - **exclude_patterns** (array): Glob patterns for excluded files
  - **model** (object): AI model configuration
    - **provider** (string): Provider name ("databricks", "openai", "anthropic")
    - **model** (string): Model identifier
    - **temperature** (number, optional): Generation temperature (0.0-1.0)
    - **max_tokens** (number, optional): Maximum tokens per response
  - **tools** (object): Tool system configuration
    - **enabled_extensions** (array): Default Goose extensions to enable
    - **custom_tools** (array): Custom tool configurations
    - **planning_mode** (boolean): Enable agent planning mode
    - **max_tool_calls** (number, optional): Maximum tool calls per turn
  - **session** (object): Session behavior configuration
    - **max_turns** (number, optional): Maximum conversation turns
    - **retry_config** (object, optional): Retry configuration
  - **permissions** (object): Tool permission enforcement
    - **tool_permissions** (object): Tool name to permission mapping
      - Values: "allow", "deny", "approve"
- **reset_existing** (boolean, optional): Reset existing configuration

### Response (Success)

```json
{
  "jsonrpc": "2.0",
  "result": {
    "status": "success",
    "message": "Agent configured successfully",
    "configuration_applied": {
      "workspace": { /* applied workspace config */ },
      "model": { /* applied model config */ },
      "tools": { /* applied tool config */ },
      "session": { /* applied session config */ },
      "permissions": { /* applied permission config */ }
    },
    "warnings": [],
    "timestamp": "2025-12-25T10:30:00Z"
  },
  "id": 1
}
```

### Response (Error)

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

## 2. kaiak/generate_fix

Process migration incidents with Goose agent. Creates or reuses session, streams progress, and waits for completion.

### Request

**Method**: `workspace/executeCommand`
**Command**: `kaiak/generate_fix`

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

### Parameters

- **session_id** (string, required): Client-generated UUID for session
- **incidents** (array, required): Migration incidents to process
  - **id** (string): Unique incident identifier
  - **rule_id** (string): Static analysis rule identifier
  - **message** (string): Brief incident description
  - **description** (string): Detailed incident explanation
  - **effort** (string): Estimated fix effort ("trivial", "low", "medium", "high")
  - **severity** (string): Issue severity ("info", "warning", "error", "critical")
- **migration_context** (object, optional): Additional migration context
  - **source_technology** (string): Source technology/version
  - **target_technology** (string): Target technology/version
  - **migration_hints** (array): Human-provided hints
  - **constraints** (array): Migration constraints
  - **preferences** (object): User preferences
- **options** (object, optional): Processing options
  - **auto_apply_safe_fixes** (boolean): Auto-apply low-risk fixes
  - **max_processing_time** (number): Timeout in seconds
  - **parallel_processing** (boolean): Enable parallel incident processing
  - **include_explanations** (boolean): Include detailed explanations

### Response (Success)

```json
{
  "jsonrpc": "2.0",
  "result": {
    "request_id": "req-550e8400-e29b-41d4-a716-446655440001",
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "status": "completed",
    "incident_count": 1,
    "completed_at": "2025-12-25T10:35:45Z"
  },
  "id": 2
}
```

### Possible Status Values

- **completed**: Agent finished successfully
- **failed**: Agent failed to process
- **cancelled**: Processing was cancelled by user

### Response (Error - Session In Use)

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32016,
    "message": "Session is currently in use by another client",
    "data": {
      "session_id": "550e8400-e29b-41d4-a716-446655440000",
      "in_use_since": "2025-12-25T10:30:00Z"
    }
  },
  "id": 2
}
```

## 3. kaiak/delete_session

Delete agent session and cleanup associated resources.

### Request

**Method**: `workspace/executeCommand`
**Command**: `kaiak/delete_session`

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

### Parameters

- **session_id** (string, required): Session identifier to delete
- **force** (boolean, optional): Force deletion even if session is active
- **cleanup_files** (boolean, optional): Remove temporary files

### Response (Success)

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
    "deleted_at": "2025-12-25T10:40:00Z"
  },
  "id": 3
}
```

### Response (Error)

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

During `kaiak/generate_fix` processing, the server sends real-time notifications. All notifications have no `id` field (JSON-RPC notifications).

### Progress Updates

**Method**: `kaiak/stream/progress`

```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/stream/progress",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "request_id": "req-550e8400-e29b-41d4-a716-446655440001",
    "message_id": "msg-1",
    "timestamp": "2025-12-25T10:35:05Z",
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

### AI Response Streaming

**Method**: `kaiak/stream/ai_response`

```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/stream/ai_response",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "request_id": "req-550e8400-e29b-41d4-a716-446655440001",
    "message_id": "msg-2",
    "timestamp": "2025-12-25T10:35:10Z",
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
    "timestamp": "2025-12-25T10:35:15Z",
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

### User Interaction Requests

**Method**: `kaiak/stream/user_interaction`

```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/stream/user_interaction",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "request_id": "req-550e8400-e29b-41d4-a716-446655440001",
    "message_id": "msg-4",
    "timestamp": "2025-12-25T10:35:20Z",
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

### File Modification Proposals

**Method**: `kaiak/stream/file_modification`

```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/stream/file_modification",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "request_id": "req-550e8400-e29b-41d4-a716-446655440001",
    "message_id": "msg-5",
    "timestamp": "2025-12-25T10:35:25Z",
    "content": {
      "proposal_id": "proposal-1",
      "file_path": "src/main/java/com/example/DataConverter.java",
      "operation": "modify",
      "diff": "@@ -15,7 +15,7 @@\n-import javax.xml.bind.DatatypeConverter;\n+import java.util.Base64;",
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
    "timestamp": "2025-12-25T10:35:30Z",
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

### System Status Updates

**Method**: `kaiak/stream/system`

```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/stream/system",
  "params": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "request_id": "req-550e8400-e29b-41d4-a716-446655440001",
    "message_id": "msg-7",
    "timestamp": "2025-12-25T10:35:35Z",
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

- **-32700**: Parse error
- **-32600**: Invalid request
- **-32601**: Method not found
- **-32602**: Invalid params
- **-32603**: Internal error

### Kaiak-Specific Error Codes

- **-32001**: Session creation failed
- **-32002**: Workspace access denied
- **-32003**: Session not found
- **-32004**: Session already terminated
- **-32005**: Session not ready
- **-32006**: Agent initialization failed
- **-32007**: Request not found
- **-32008**: Request already completed
- **-32009**: Interaction not found
- **-32010**: Interaction already responded
- **-32011**: Response validation failed
- **-32012**: File modification failed
- **-32013**: Tool execution timeout
- **-32014**: Configuration error
- **-32015**: Resource exhausted
- **-32016**: Session in use (concurrent access prevention)

## Session Lifecycle

1. **Configuration**: Client calls `kaiak/configure` to set up agent
2. **Session Creation**: Automatic when `kaiak/generate_fix` is called with new session_id
3. **Processing**: Agent processes incidents, streams real-time notifications
4. **Completion**: Agent finishes, returns final response
5. **Cleanup**: Optional cleanup via `kaiak/delete_session`

## Concurrent Access Control

- Only one client can use a session at a time
- Attempts to use an active session return error `-32016`
- Sessions automatically release after completion or timeout
- Use client-generated UUIDs for session IDs

## Complete Workflow Example

### 1. Configure Agent

```json
{
  "jsonrpc": "2.0",
  "method": "workspace/executeCommand",
  "params": {
    "command": "kaiak/configure",
    "arguments": [{
      "configuration": {
        "workspace": {
          "working_dir": "/home/user/project",
          "include_patterns": ["**/*.java"],
          "exclude_patterns": ["target/**"]
        },
        "model": {
          "provider": "openai",
          "model": "gpt-4"
        },
        "tools": {
          "enabled_extensions": ["developer"],
          "planning_mode": false
        },
        "permissions": {
          "tool_permissions": {
            "read_file": "allow",
            "write_file": "approve"
          }
        }
      }
    }]
  },
  "id": 1
}
```

### 2. Generate Fixes

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
          "id": "dep-1",
          "rule_id": "deprecated-api",
          "message": "Replace Collections.sort() with List.sort()",
          "description": "Collections.sort() is deprecated in Java 8+",
          "effort": "trivial",
          "severity": "warning"
        }
      ]
    }]
  },
  "id": 2
}
```

### 3. Monitor Progress

Client receives streaming notifications:

```json
{ "jsonrpc": "2.0", "method": "kaiak/stream/progress", "params": { /* progress data */ } }
{ "jsonrpc": "2.0", "method": "kaiak/stream/ai_response", "params": { /* AI response */ } }
{ "jsonrpc": "2.0", "method": "kaiak/stream/tool_call", "params": { /* tool execution */ } }
{ "jsonrpc": "2.0", "method": "kaiak/stream/user_interaction", "params": { /* approval request */ } }
```

### 4. Clean Up

```json
{
  "jsonrpc": "2.0",
  "method": "workspace/executeCommand",
  "params": {
    "command": "kaiak/delete_session",
    "arguments": [{
      "session_id": "550e8400-e29b-41d4-a716-446655440000"
    }]
  },
  "id": 3
}
```

## Integration Notes

- Use LSP client libraries for seamless transport handling
- Handle streaming notifications asynchronously
- Implement proper error recovery for network interruptions
- Respect tool permission configurations for security
- Generate UUIDs client-side for session management

## Migration from Legacy API

If migrating from the old multi-endpoint API:

- `kaiak/session/create` → `kaiak/configure`
- `kaiak/session/terminate` → `kaiak/delete_session`
- `kaiak/fix/generate` → `kaiak/generate_fix`
- All methods now use `workspace/executeCommand` pattern
- Session creation is automatic on first `kaiak/generate_fix` call
- Configuration is per-agent rather than per-session

This API provides a simplified, secure, and efficient interface for code migration workflows powered by the Goose AI agent.
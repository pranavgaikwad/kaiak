# Quickstart: Client-to-Server Notifications

**Date**: 2025-12-30
**Feature**: Client-to-Server Notifications

## Overview

This quickstart guide demonstrates how to use the new client-to-server notification feature in Kaiak. This feature allows clients connected via Unix sockets to send notifications back to the server for validation and routing.

## Prerequisites

- Kaiak server running with socket transport
- Active agent session (from a previous `kaiak/generate_fix` request)
- Client connected to server via Unix socket

## Basic Usage

### 1. Start Kaiak Server with Socket Transport

```bash
# Start server with Unix socket
kaiak serve --socket /tmp/kaiak.sock

# In another terminal, connect the CLI client
kaiak connect /tmp/kaiak.sock
```

### 2. Create an Agent Session

```bash
# Generate a fix to create an active session (session ID will be returned)
kaiak generate-fix --params-json '{
  "incidents": [{
    "id": "test-incident",
    "rule_id": "example-rule",
    "message": "Test incident for notification demo",
    "severity": "info"
  }],
  "agent_config": {
    "workspace": {"working_dir": "/path/to/project"},
    "model": {"provider": "openai", "model_id": "gpt-4"}
  }
}'

# Note the returned session_id for use in notifications
```

### 3. Send Client Notifications

#### User Input Notification

```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/client/user_message",
  "params": {
    "session_id": "your-session-id-here",
    "message_type": "user_input",
    "timestamp": "2025-12-30T10:30:00Z",
    "payload": {
      "text": "Yes, proceed with the suggested changes",
      "context": "approval_request"
    }
  }
}
```

#### Control Signal Notification

```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/client/user_message",
  "params": {
    "session_id": "your-session-id-here",
    "message_type": "control_signal",
    "timestamp": "2025-12-30T10:32:00Z",
    "payload": {
      "action": "pause",
      "reason": "user_requested_pause"
    }
  }
}
```

> Note that payload is a freeform json.

### 4. Handle Responses and Errors

#### Success Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "success": true,
    "message": "Notification received and validated",
    "notification_id": "notif-abc123"
  }
}
```
#### Invalid Session Error

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32602,
    "message": "Session ID 'invalid-session' not found or invalid"
  }
}
```

## Advanced Usage

### Error Handling and Retries

The client automatically handles connection failures with retry logic:

```
Connection lost → Queue notification → Retry with backoff → User feedback
                                    ↓
                  100ms → 500ms → 2s → Permanent failure notification
```


### Size Limits

- **Maximum notification size**: 1MB (including all JSON overhead)
- **Validation**: Performed before JSON parsing to prevent resource exhaustion
- **Error**: Returns parse error (-32700) if size exceeded

## Testing and Validation

### Test Notification Receipt

```bash
# Send a simple test notification
echo '{
  "jsonrpc": "2.0",
  "method": "kaiak/client/user_message",
  "params": {
    "session_id": "test-session",
    "message_type": "user_input",
    "timestamp": "'$(date -u +%Y-%m-%dT%H:%M:%SZ)'",
    "payload": {"text": "test message"}
  }
}' | kaiak send-notification
```

### Verify Session Validation

```bash
# Test with invalid session ID (should return error)
echo '{
  "jsonrpc": "2.0",
  "method": "kaiak/client/user_message",
  "params": {
    "session_id": "nonexistent-session",
    "message_type": "user_input",
    "timestamp": "'$(date -u +%Y-%m-%dT%H:%M:%SZ)'",
    "payload": {"text": "test"}
  }
}' | kaiak send-notification
```

## Integration Examples

### IDE Extension Integration

```typescript
// Example TypeScript integration for VS Code extension
class KaiakNotificationSender {
  async sendUserInput(sessionId: string, text: string): Promise<boolean> {
    const notification = {
      jsonrpc: "2.0",
      method: "kaiak/client/user_message",
      params: {
        session_id: sessionId,
        message_type: "user_input",
        timestamp: new Date().toISOString(),
        payload: { text, source: "vscode_extension" }
      }
    };

    try {
      const response = await this.sendNotification(notification);
      return response.result?.success === true;
    } catch (error) {
      console.error("Failed to send notification:", error);
      return false;
    }
  }
}
```

### Command Line Integration

```bash
#!/bin/bash
# Script to send user feedback to active Kaiak session

KAIAK_SOCKET="/tmp/kaiak.sock"
SESSION_ID="$1"
MESSAGE="$2"

if [[ -z "$SESSION_ID" || -z "$MESSAGE" ]]; then
  echo "Usage: $0 <session_id> <message>"
  exit 1
fi

echo "{
  \"jsonrpc\": \"2.0\",
  \"method\": \"kaiak/client/user_message\",
  \"params\": {
    \"session_id\": \"$SESSION_ID\",
    \"message_type\": \"user_input\",
    \"timestamp\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\",
    \"payload\": {\"text\": \"$MESSAGE\", \"source\": \"cli_script\"}
  }
}" | socat - UNIX-CONNECT:$KAIAK_SOCKET
```

This quickstart provides the essential information needed to start using client-to-server notifications in Kaiak.
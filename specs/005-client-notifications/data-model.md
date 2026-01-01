# Data Model: Client-to-Server Notifications

**Date**: 2025-12-30
**Feature**: Client-to-Server Notifications

## Overview

This feature extends the existing Kaiak JSON-RPC infrastructure to support client-originated notifications. The data model leverages existing JSON-RPC types and patterns established in the codebase.

## Core Entities

### Client Notification

**Description**: Represents a notification sent from client to server via socket connection.

**Fields**:
- `session_id`: String - Required. Identifies the target agent session for routing validation
- `message_type`: String - Required. Categorizes the notification content (e.g., "user_input", "status_update", "control_signal")
- `timestamp`: String (ISO 8601) - Required. When the notification was created
- `payload`: JsonValue - Optional. Unstructured JSON containing the actual user input or message data

**Validation Rules**:
- `session_id`: Must be non-empty, alphanumeric with hyphens/underscores
- `message_type`: Must be one of predefined types or match pattern `^[a-z_]+$`
- `timestamp`: Must be valid ISO 8601 format
- `payload`: Must not exceed 1MB when serialized
- Total notification size: Maximum 1MB

**JSON-RPC Representation**:
```json
{
  "jsonrpc": "2.0",
  "method": "kaiak/client/user_message",
  "params": {
    "session_id": "goose-session-abc123",
    "message_type": "user_input",
    "timestamp": "2025-12-30T10:30:00Z",
    "payload": {
      "text": "Yes, proceed with the changes",
      "context": "approval_request"
    }
  }
}
```

## Error Handling

### Validation Errors

**Size Limit Exceeded**:
- Error Code: -32600 (Invalid Request)
- Message: "Notification payload exceeds 1MB size limit"
- Recovery: User must reduce payload size

**Invalid Session ID**:
- Error Code: -32602 (Invalid params)
- Message: "Session ID not found or invalid"
- Recovery: Verify session exists and is active

**Malformed Notification**:
- Error Code: -32700 (Parse error)
- Message: "Invalid JSON-RPC notification format"
- Recovery: Check notification structure and retry

### Transport Errors

**Connection Failure**:
- Behavior: Queue notification for brief retry
- Retry Strategy: 3 attempts with exponential backoff (100ms, 500ms, 2s)
- User Feedback: "Connection lost, retrying..." then "Failed to send notification"

## Implementation Notes

### Existing Infrastructure Reuse

- **JsonRpcNotification**: Existing struct in `src/jsonrpc/protocol.rs` can be used for notification structure
- **Error Types**: Leverage existing `JsonRpcError` and `KaiakError` mappings
- **Transport Layer**: Extend existing socket transport in `src/client/transport.rs`
- **Session Management**: Use existing session tracking without modification

### Storage Requirements

**Transient Storage Only**:
- No persistent storage required for notifications
- Retry queue is in-memory only with 10-second timeout
- Session validation uses existing session management infrastructure

**Memory Considerations**:
- Maximum retry queue size: 10 notifications per client
- Notification size limit prevents memory exhaustion
- Rate limiting prevents queue overflow

This data model maintains consistency with the existing Kaiak architecture while providing the minimal structure needed for client-to-server notification functionality.
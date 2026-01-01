 # Research: Client-to-Server Notifications

**Date**: 2025-12-30
**Feature**: Client-to-Server Notifications

## Current Kaiak Architecture Analysis

### JSON-RPC Infrastructure

**Decision**: Leverage existing JSON-RPC infrastructure in `src/jsonrpc/protocol.rs`
**Rationale**: Well-established JSON-RPC 2.0 implementation with comprehensive error handling and notification patterns already exists
**Alternatives considered**: Building separate notification system - rejected due to code duplication and architectural inconsistency

**Key Existing Structures**:
- `JsonRpcNotification`: Server-to-client notifications (can be adapted for client-to-server)
- `JsonRpcRequest`/`JsonRpcResponse`: Request-response pattern
- Comprehensive error mapping with specific error codes

### Transport Implementation

**Decision**: Extend existing client transport in `src/client/transport.rs`
**Rationale**: Current implementation already supports Unix domain sockets with LSP-style framing and error handling
**Alternatives considered**: New transport layer - rejected due to architectural complexity and duplication

**Existing Transport Features**:
- Unix domain socket communication
- LSP-style Content-Length header framing
- Request-response matching with UUID
- Notification handling via callback mechanism
- Connection state management in `~/.kaiak/connection`

### Method Registration Pattern

**Decision**: Follow existing method registration in `src/jsonrpc/methods.rs` using `KaiakRpc` trait
**Rationale**: Type-safe method definitions with consistent namespace (`kaiak/*`)
**Alternatives considered**: Direct method handling - rejected due to lack of type safety

**Implementation Pattern**:
```rust
// Method: kaiak/client/user_message
// Follows existing pattern: kaiak/generate_fix, kaiak/delete_session
```

## Notification Validation Strategy

### Size and Rate Limiting

**Decision**: Implement validation at transport layer before JSON parsing
**Rationale**: Early validation prevents resource exhaustion and follows security best practices
**Alternatives considered**: Application-layer validation only - rejected due to DoS vulnerability

**Implementation Approach**:
- 1MB payload size limit (validated at transport layer)
- 100 notifications/minute per client (token bucket algorithm)
- Early rejection with clear error messages

### JSON Schema Validation

**Decision**: Use serde validation with custom deserializers for basic structure validation
**Rationale**: Minimal dependency approach, leverages existing serde usage
**Alternatives considered**: jsonschema crate - rejected due to additional dependency and complexity for simple validation

**Validation Strategy**:
- Required fields: session_id, message_type, timestamp
- Optional unstructured payload as specified in requirements
- Session ID validation against existing session management

### Session-Based Routing

**Decision**: Leverage existing session management infrastructure
**Rationale**: Reuses established patterns, follows Principle VII (Goose API Primacy)
**Alternatives considered**: New session tracking - rejected due to duplication

**Routing Pattern**:
- Extract session_id from notification payload
- Validate session exists (but do not forward to agent per requirements)
- Log receipt for debugging/monitoring

## Error Handling and Recovery

### Connection Failure Handling

**Decision**: Implement brief retry queue with exponential backoff
**Rationale**: Balances reliability with simplicity, follows existing error handling patterns
**Alternatives considered**: Persistent queue - rejected due to complexity for notification-only feature

**Retry Strategy**:
- Maximum 3 retry attempts
- Exponential backoff: 100ms, 500ms, 2s
- Clear user feedback on permanent failure
- Queue timeout after 10 seconds

### User Feedback Patterns

**Decision**: Follow existing error reporting patterns in `protocol.rs`
**Rationale**: Consistent user experience, leverages established error codes
**Alternatives considered**: Custom error system - rejected due to consistency requirements

**Error Categories**:
- Transport errors: Connection failures, timeouts
- Validation errors: Size limits, malformed JSON
- Session errors: Invalid session ID, session not found

## Integration Approach

### Minimal Code Changes

**Decision**: Extend existing modules rather than creating new ones
**Rationale**: Follows Progressive Development principle, minimizes architectural impact
**Alternatives considered**: New notification module - rejected due to unnecessary complexity

**Extension Points**:
- `src/client/transport.rs`: Add notification sending capability
- `src/jsonrpc/server.rs`: Add client notification handling
- `src/jsonrpc/methods.rs`: Register `kaiak/client/user_message` method
- `src/handlers/`: Add client notification routing handler

### Testing Strategy

**Decision**: Integration tests focusing on socket communication flows
**Rationale**: Follows Testing Integrity principle (e2e → integration → unit)
**Alternatives considered**: Unit tests only - rejected due to constitution requirements

**Test Coverage**:
- Client-to-server notification sending
- Server notification receipt and validation
- Error scenarios (connection failures, size limits)
- Session ID validation

## Technology Decisions

### Dependencies

**Decision**: No new dependencies required
**Rationale**: Existing tokio, serde, and JSON-RPC infrastructure sufficient
**Alternatives considered**: Additional validation crates - rejected due to minimal dependency principle

### Performance Characteristics

**Decision**: Async notification handling to prevent blocking
**Rationale**: Consistent with existing tokio-based architecture
**Alternatives considered**: Synchronous handling - rejected due to performance implications

## Implementation Priority

1. **Phase 1**: Extend client transport for notification sending
2. **Phase 2**: Add server-side notification receipt and validation
3. **Phase 3**: Implement error handling and retry mechanisms

This research confirms that the client-to-server notification feature can be implemented with minimal architectural changes by extending existing, well-established patterns in the Kaiak codebase.
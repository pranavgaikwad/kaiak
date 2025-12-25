# Kaiak API Documentation

> **Note**: This documentation has been updated and moved to [../api-reference.md](../api-reference.md) which contains the complete, current API reference.

Complete API reference for the Kaiak Migration Server JSON-RPC interface.

## Overview

Kaiak provides a simplified JSON-RPC 2.0 API over LSP transport for all client communication. The API is designed for real-time streaming workflows with comprehensive error handling.

### Quick Reference

**API Methods** (via `workspace/executeCommand`):
- **kaiak/configure** - Configure agent workspace and settings
- **kaiak/generate_fix** - Generate fixes for migration incidents
- **kaiak/delete_session** - Clean up agent session

**Streaming Notifications**:
- **kaiak/stream/progress** - Progress updates during processing
- **kaiak/stream/ai_response** - AI model responses
- **kaiak/stream/tool_call** - Tool execution status
- **kaiak/stream/user_interaction** - User approval prompts
- **kaiak/stream/file_modification** - File change proposals
- **kaiak/stream/error** - Error notifications
- **kaiak/stream/system** - System status updates

### Basic Workflow

1. **Configure** the agent with workspace and model settings:
   ```json
   {
     "jsonrpc": "2.0",
     "method": "workspace/executeCommand",
     "params": {
       "command": "kaiak/configure",
       "arguments": [{ /* configuration */ }]
     },
     "id": 1
   }
   ```

2. **Generate fixes** for incidents (creates session automatically):
   ```json
   {
     "jsonrpc": "2.0",
     "method": "workspace/executeCommand",
     "params": {
       "command": "kaiak/generate_fix",
       "arguments": [{
         "session_id": "your-uuid",
         "incidents": [/* array of incidents */]
       }]
     },
     "id": 2
   }
   ```

3. **Monitor** real-time progress via streaming notifications

4. **Clean up** when done:
   ```json
   {
     "jsonrpc": "2.0",
     "method": "workspace/executeCommand",
     "params": {
       "command": "kaiak/delete_session",
       "arguments": [{ "session_id": "your-uuid" }]
     },
     "id": 3
   }
   ```

## Complete Documentation

For detailed information including:
- Complete parameter specifications
- Response formats and error codes
- Streaming notification details
- Session lifecycle management
- Integration examples
- Migration from legacy API

Please see the **[Complete API Reference](../api-reference.md)**.

## Transport

All messages use LSP message framing:
```
Content-Length: {byte_length}\r\n
\r\n
{json_message}
```

Supports stdio and Unix socket transport for enterprise-safe communication.

## Integration

- Use LSP client libraries for transport handling
- Handle streaming notifications asynchronously
- Implement error recovery for network interruptions
- Respect tool permissions for security
- Generate UUIDs client-side for session management

For examples and integration patterns, see:
- [Main README](../../README.md#first-fix-generation)
- [Complete API Reference](../api-reference.md)
- [Integration Tests](../../tests/integration/)
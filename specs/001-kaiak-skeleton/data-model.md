# Data Model: Kaiak Migration Server Skeleton

**Created**: 2025-12-22
**Input**: Extracted from feature specification entities and research findings

## Core Entities

### Fix Generation Request

Represents a request from the IDE to generate fixes for one or more incidents.

**Fields**:
- `id`: Unique request identifier (UUID)
- `session_id`: Associated session identifier
- `incidents`: Array of incident objects to process
- `workspace_path`: Root path of the workspace being analyzed
- `migration_context`: Additional context for migration prompts
- `preferences`: User preferences for fix generation
- `created_at`: Timestamp of request creation

**Validation Rules**:
- `id` must be valid UUID format
- `incidents` array must not be empty
- `workspace_path` must be absolute and exist
- `migration_context` is optional but if provided must be valid JSON
- `preferences` follows predefined schema

**Relationships**:
- Belongs to one AI Session
- Contains one or more Incidents
- Generates zero or more Stream Messages

**State Transitions**:
```
pending → processing → completed
                   → failed
                   → cancelled
```

---

### Incident

Represents a specific code issue identified by static analysis tools.

**Fields**:
- `id`: Unique incident identifier
- `rule_id`: Static analysis rule that triggered this incident
- `file_path`: Relative path to file containing the issue
- `line_number`: Line number where issue occurs
- `severity`: Issue severity level (error, warning, info)
- `description`: Human-readable description of the issue
- `message`: Human-readable detailed message explaining the issue
- `category`: Category classification (deprecated-api, security, performance, etc.)
- `metadata`: Additional tool-specific metadata

**Validation Rules**:
- `file_path` must be relative to workspace root
- `line_number` must be positive integer
- `severity` must be one of: error, warning, info
- `description` must be non-empty string
- `category` must match predefined classification system

**Relationships**:
- Belongs to one or more Fix Generation Requests
- May generate File Modification Proposals
- Associated with workspace file system

---

### AI Session

Manages the Goose agent lifecycle, configuration, and processing state.

**Fields**:
- `id`: Unique session identifier (UUID)
- `goose_session_id`: Corresponding Goose internal session ID
- `status`: Current session status
- `created_at`: Session creation timestamp
- `updated_at`: Last activity timestamp
- `configuration`: Session-specific configuration
- `active_request_id`: Currently processing request (optional)
- `message_count`: Number of messages exchanged
- `error_count`: Number of errors encountered

**Validation Rules**:
- `id` must be valid UUID format
- `status` must be valid session state
- `configuration` must conform to schema
- `message_count` and `error_count` must be non-negative

**State Transitions**:
```
created → initializing → ready → processing → completed
                              → error → ready (recoverable)
                              → terminated (unrecoverable)
```

**Relationships**:
- Contains multiple Fix Generation Requests
- Produces multiple Stream Messages
- Manages one Goose agent instance

---

### Stream Message

Real-time communication payload for progress updates, AI responses, or interaction requests.

**Fields**:
- `id`: Unique message identifier
- `session_id`: Associated session identifier
- `type`: Message type classification
- `timestamp`: Message creation time
- `content`: Message payload (variant based on type)
- `metadata`: Additional message context

**Message Types**:
- `progress`: Progress update with percentage and description
- `ai_response`: AI-generated text response
- `tool_call`: Tool execution request or result
- `thinking`: AI reasoning/thinking process
- `user_interaction`: Request for user input or confirmation
- `error`: Error notification
- `system`: System status or lifecycle events

**Content Variants**:

**Progress Content**:
- `percentage`: Completion percentage (0-100)
- `phase`: Current processing phase
- `description`: Human-readable progress description

**AI Response Content**:
- `text`: Generated response text
- `partial`: Boolean indicating if more text follows
- `confidence`: Response confidence score (optional)

**Tool Call Content**:
- `tool_name`: Name of tool being called
- `operation`: Tool operation (start, progress, complete, error)
- `parameters`: Tool-specific parameters
- `result`: Tool execution result (if complete)

**User Interaction Content**:
- `interaction_type`: Type of interaction (approval, choice, input)
- `prompt`: User-facing prompt text
- `options`: Available choices (for choice interactions)
- `default`: Default selection (optional)

**Validation Rules**:
- `type` must match predefined message types
- `content` must conform to type-specific schema
- `timestamp` must be valid ISO 8601 format

**Relationships**:
- Belongs to one AI Session
- May reference File Modification Proposals

---

### File Modification Proposal

Detailed description of proposed code changes requiring user approval.

**Fields**:
- `id`: Unique proposal identifier
- `session_id`: Associated session identifier
- `file_path`: Path to file being modified
- `original_content`: Current file content
- `proposed_content`: Proposed modified content
- `change_type`: Type of modification (edit, create, delete, rename)
- `description`: Human-readable description of changes
- `confidence`: AI confidence in the proposed changes
- `approval_status`: Current approval status
- `created_at`: Proposal creation time
- `approved_at`: Approval timestamp (optional)

**Change Types**:
- `edit`: Modify existing file content
- `create`: Create new file
- `delete`: Remove existing file
- `rename`: Change file name or location

**Approval Status**:
- `pending`: Awaiting user review
- `approved`: User approved changes
- `rejected`: User rejected changes
- `expired`: Approval timeout reached

**Validation Rules**:
- `file_path` must be within workspace boundaries
- `original_content` must match current file state (for edits)
- `confidence` must be between 0.0 and 1.0
- `change_type` must be valid enum value

**State Transitions**:
```
pending → approved → applied
       → rejected → discarded
       → expired → discarded
```

**Relationships**:
- Belongs to one AI Session
- May be referenced by Stream Messages
- Associated with File System operations

---

### User Interaction

Bidirectional communication for approvals, selections, and configuration during processing.

**Fields**:
- `id`: Unique interaction identifier
- `session_id`: Associated session identifier
- `type`: Interaction type (approval, choice, input, confirmation)
- `prompt`: User-facing prompt or question
- `request_data`: Type-specific request data
- `response_data`: User's response (optional until completed)
- `status`: Interaction status
- `timeout`: Maximum wait time for response
- `created_at`: Interaction creation time
- `responded_at`: Response timestamp (optional)

**Interaction Types**:
- `approval`: Yes/No approval decision
- `choice`: Select from multiple options
- `input`: Free-form text input
- `confirmation`: Acknowledge information

**Request Data Variants**:

**Approval Data**:
- `default_choice`: Default selection (approve/reject)
- `auto_approve`: Whether to auto-approve after timeout

**Choice Data**:
- `options`: Array of selectable options
- `multiple`: Whether multiple selections allowed
- `default_indices`: Default selected option indices

**Input Data**:
- `placeholder`: Input field placeholder text
- `validation`: Input validation rules
- `multiline`: Whether multiline input allowed

**Validation Rules**:
- `type` must be valid interaction type
- `request_data` must conform to type-specific schema
- `timeout` must be positive duration
- `response_data` must be validated against request requirements

**State Transitions**:
```
pending → responded → processed
       → timeout → expired
       → cancelled
```

**Relationships**:
- Belongs to one AI Session
- May trigger File Modification Proposals
- Generates Stream Messages for user communication

---

## Data Flow Relationships

```
Fix Generation Request
    ├── contains multiple Incidents
    ├── belongs to AI Session
    └── generates Stream Messages

AI Session
    ├── manages Goose agent lifecycle
    ├── contains multiple Fix Generation Requests
    ├── produces multiple Stream Messages
    └── coordinates User Interactions

Stream Messages
    ├── belong to AI Session
    ├── may reference File Modification Proposals
    └── communicate with IDE in real-time

File Modification Proposals
    ├── belong to AI Session
    ├── triggered by AI processing
    └── require User Interaction approval

User Interactions
    ├── belong to AI Session
    ├── may trigger File Modification Proposals
    └── generate responsive Stream Messages
```

## Persistence Strategy

**Session Data**: Leverages Goose's SQLite database for conversation history and session metadata
**Active State**: In-memory caching for real-time session data
**Proposals**: Temporary storage with cleanup after approval/rejection
**Messages**: Streaming-only (not persisted) for real-time communication

## Concurrency Model

**Session Isolation**: Each session operates independently with thread-safe access
**Message Streaming**: Non-blocking async streaming for real-time communication
**State Management**: Atomic updates using Arc<Mutex<>> for shared state
**Agent Coordination**: Leverages Goose's built-in concurrency controls
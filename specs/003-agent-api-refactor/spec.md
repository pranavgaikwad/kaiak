# Feature Specification: Agent API Refactor for Goose Integration

**Feature Branch**: `003-agent-api-refactor`
**Created**: 2025-12-24
**Status**: Draft
**Input**: User description: "Refactor Kaiak's agent API to properly integrate with Goose AI framework, simplify API surface, delegate session management to Goose, and correctly initialize agents with proper tool system integration."

## Clarifications

### Session 2025-12-24

- Q: Configuration data format for configure() endpoint → A: Structured JSON with nested sections (e.g., {"workspace": {...}, "model": {...}, "tools": {...}})
- Q: Session ID format and management → A: Client-generated UUIDs (clients create and pass UUIDs to the server)
- Q: Concurrent session access behavior → A: Block all concurrent access - only one client can use a session at a time
- Q: Event streaming protocol → A: Reuse existing JSON-RPC protocol and transport without changes

## User Scenarios & Testing

### User Story 1 - Simplify API Surface (Priority: P1)

The Kaiak server exposes a minimal set of API endpoints (configure, generate_fix, delete_session) that external clients can use to interact with the Goose agent. All unnecessary API endpoints and complex session management operations are removed.

**Why this priority**: This is the core architectural change that enables all other improvements. Without a simplified API surface, the system remains overly complex and difficult to maintain.

**Independent Test**: Can be fully tested by verifying that only the three specified endpoints are exposed, all other endpoints return appropriate errors, and the simplified API handles all core agent operations.

**Acceptance Scenarios**:

1. **Given** the Kaiak server is running, **When** a client calls configure() with valid parameters, **Then** the server accepts configuration and returns success status
2. **Given** the server is configured, **When** a client calls generate_fix() with session ID and incidents, **Then** the server processes the request using Goose agent
3. **Given** an active session exists, **When** a client calls delete_session() with session ID, **Then** the server removes the session and returns confirmation

---

### User Story 2 - Delegate Session Management to Goose (Priority: P1)

Kaiak server acts as a thin wrapper around Goose's native session management, automatically creating sessions when needed and leveraging Goose's SessionManager for all persistence and lifecycle operations.

**Why this priority**: Eliminating custom session management reduces complexity, improves reliability, and ensures consistency with Goose's expected patterns. This is essential for proper Goose integration.

**Independent Test**: Can be fully tested by verifying that sessions are created using Goose's SessionManager, session data persists through Goose's mechanisms, and no custom session persistence logic is involved.

**Acceptance Scenarios**:

1. **Given** a generate_fix request with new session ID, **When** the server processes the request, **Then** a new Goose session is created using SessionManager
2. **Given** a generate_fix request with existing session ID, **When** the server processes the request, **Then** the existing Goose session is reused
3. **Given** session deletion is requested, **When** the server processes delete_session(), **Then** Goose's session cleanup mechanisms are invoked

---

### User Story 3 - Implement Proper Goose Agent Initialization (Priority: P1)

The server correctly initializes Goose agents with appropriate configuration, default tools, custom tools, and planning mode settings according to Goose's expected patterns.

**Why this priority**: Without proper agent initialization, the Goose integration cannot function correctly. This ensures that agents have all necessary tools and operate according to intended permissions and capabilities.

**Independent Test**: Can be fully tested by creating agents through the API and verifying that they have access to expected tools, respect permission settings, and can operate in configured planning modes.

**Acceptance Scenarios**:

1. **Given** agent initialization is triggered, **When** the server creates a Goose agent, **Then** all default Goose tools are available and functional
2. **Given** custom tool permissions are configured, **When** the agent attempts tool operations, **Then** permission enforcement works as expected
3. **Given** planning mode is enabled, **When** the agent processes complex requests, **Then** Goose's planning capabilities are properly utilized

---

### User Story 4 - Stream Agent Events to Clients (Priority: P2)

The server forwards Goose agent events (tool calls, AI responses, processing status) to connected clients in real-time, enabling responsive user interfaces and monitoring capabilities.

**Why this priority**: While important for user experience, this depends on proper agent initialization and API structure being in place first. It enhances the core functionality rather than enabling it.

**Independent Test**: Can be fully tested by initiating agent processing and verifying that all relevant events are forwarded to clients with appropriate timing and detail.

**Acceptance Scenarios**:

1. **Given** an agent is processing incidents, **When** the agent makes tool calls, **Then** tool call events are streamed to the client
2. **Given** an agent generates responses, **When** AI responses are produced, **Then** response events are streamed to the client
3. **Given** an agent encounters errors, **When** error conditions occur, **Then** error events are streamed to the client with appropriate detail

---

### Edge Cases

- What happens when Goose session creation fails due to workspace access issues?
- How does the system handle agent initialization failures when required tools are unavailable?
- What occurs when streaming connections are interrupted during long-running agent operations?
- How does the system handle session locking when blocking concurrent access to the same session ID?
- What happens when agent tool permissions conflict with Goose's default tool capabilities?

## Requirements

### Functional Requirements

- **FR-001**: System MUST expose only three public API endpoints: configure(), generate_fix(), and delete_session()
- **FR-002**: System MUST remove all existing custom session creation and management API endpoints
- **FR-003**: System MUST delegate all session operations to Goose's SessionManager with SessionType::User
- **FR-004**: System MUST accept client-generated UUID session IDs and automatically create new Goose sessions when generate_fix() is called with unknown session IDs
- **FR-005**: System MUST reuse existing Goose sessions when generate_fix() is called with known session IDs and block concurrent access to ensure only one client can use a session at a time
- **FR-006**: System MUST initialize Goose agents using goose::agents::Agent with proper SessionConfig
- **FR-007**: System MUST provide all default Goose tools to initialized agents
- **FR-008**: System MUST maintain existing tool permission enforcement within the Goose tool framework
- **FR-009**: System MUST support easy addition of custom tools to Goose agent instances
- **FR-010**: System MUST enable configuration of Goose planning mode as required
- **FR-011**: System MUST stream agent events (tool calls, AI responses, status) in real-time to connected clients using existing JSON-RPC protocol and transport
- **FR-012**: System MUST handle user interactions including tool call approvals, denials, and free-form input
- **FR-013**: System MUST support reconfiguration through multiple configure() calls during operation
- **FR-014**: System MUST properly clean up Goose sessions when delete_session() is called
- **FR-015**: System MUST remove all custom session persistence logic and data structures
- **FR-016**: Documentation MUST be updated to reflect the new simplified API contract and Goose integration patterns
- **FR-017**: All tests related to removed API endpoints and custom session management MUST be removed
- **FR-018**: New tests MUST be added to cover Goose session management integration and agent initialization
- **FR-019**: Integration tests MUST verify proper Goose agent tool availability and permission enforcement
- **FR-020**: API documentation MUST clearly specify the three-endpoint interface and expected behavior patterns

### Key Entities

- **Agent Configuration**: Structured JSON object with nested sections for workspace settings, model provider details, tool permissions, and planning mode settings that control Goose agent behavior
- **Agent Session**: A Goose-managed session instance containing conversation history, context, and state for a specific agent interaction
- **Migration Incident**: Input data representing code issues that require agent processing and resolution
- **Agent Event**: Real-time notifications from Goose agents including tool calls, responses, and status updates
- **User Interaction**: Requests for client input during agent processing, including approvals and contextual information

## Success Criteria

### Measurable Outcomes

- **SC-001**: API surface area is reduced to exactly three endpoints with all other endpoints removed
- **SC-002**: Zero custom session persistence logic remains in the codebase after refactor
- **SC-003**: All agent sessions are created and managed exclusively through Goose's SessionManager
- **SC-004**: Agent initialization completes successfully with all default Goose tools available
- **SC-005**: Custom tool integration works seamlessly within Goose's tool framework
- **SC-006**: Real-time event streaming delivers agent updates to clients within 100ms via existing JSON-RPC transport
- **SC-007**: User interaction handling maintains existing approval/denial workflow functionality
- **SC-008**: Server startup time improves by at least 30% due to simplified architecture
- **SC-009**: Memory usage is reduced by at least 20% by eliminating redundant session management
- **SC-010**: 100% of existing tool permission enforcement continues to work after refactor
- **SC-011**: All outdated tests are removed and test suite execution time decreases by at least 25%
- **SC-012**: Test coverage for new Goose integration paths reaches at least 90%
- **SC-013**: API documentation completeness score reaches 100% for all three endpoints
- **SC-014**: Zero broken references remain in documentation after API endpoint removal

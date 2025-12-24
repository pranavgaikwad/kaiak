# Tasks: Agent API Refactor for Goose Integration

**Input**: Design documents from `/specs/003-agent-api-refactor/`
**Prerequisites**: plan.md, spec.md (user stories), research.md, data-model.md, contracts/jsonrpc-api.md

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3, US4)
- Include exact file paths in descriptions

## Path Conventions

- **Single Rust project**: `src/`, `tests/` at repository root
- Paths assume Rust project structure from plan.md

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and Goose dependency integration

- [ ] T001 Add Goose dependency to Cargo.toml as git dependency from github.com/block/goose
- [ ] T002 [P] Update existing dependencies (tower-lsp, tokio, serde, anyhow, tracing) to versions compatible with Goose
- [ ] T003 [P] Configure cargo clippy rules and formatting for Goose integration patterns
- [ ] T004 [P] Update .gitignore to exclude Goose session database files and temporary artifacts

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [ ] T005 Create src/models/mod.rs with public exports for AgentConfiguration, AgentSession, MigrationIncident, AgentEventNotification, and UserInteractionRequest per data-model.md
- [ ] T006 [P] Implement AgentConfiguration struct in src/models/configuration.rs with nested sections (workspace, model, tools, session, permissions)
- [ ] T007 [P] Implement MigrationIncident struct in src/models/incidents.rs with simplified fields (id, rule_id, message, description, effort, severity)
- [ ] T008 [P] Implement AgentEventNotification and related event types in src/models/events.rs for real-time streaming
- [ ] T009 [P] Implement UserInteractionRequest and response types in src/models/interactions.rs for tool approvals
- [ ] T010 Create src/handlers/mod.rs with public exports for configure, generate_fix, and delete_session handlers
- [ ] T011 Update src/server.rs to route only three commands (kaiak/configure, kaiak/generate_fix, kaiak/delete_session) via execute_command pattern
- [ ] T012 Create src/agents/mod.rs with GooseAgentManager struct for centralizing Goose agent operations
- [ ] T013 Remove all existing custom session management code from src/session.rs and replace with Goose SessionManager wrapper
- [ ] T014 Update error handling in src/lib.rs to include Goose integration specific errors (-32016 for session in use)

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Simplify API Surface (Priority: P1) 🎯 MVP

**Goal**: Expose only three API endpoints (configure, generate_fix, delete_session) and remove all unnecessary endpoints

**Independent Test**: Verify that only the three specified endpoints are exposed, all other endpoints return method not found errors, and the simplified API handles all core agent operations

### Implementation for User Story 1

- [ ] T015 [P] [US1] Implement configure() handler in src/handlers/configure.rs accepting AgentConfiguration and returning success/error status
- [ ] T016 [P] [US1] Implement generate_fix() handler in src/handlers/generate_fix.rs accepting session_id and incidents array
- [ ] T017 [P] [US1] Implement delete_session() handler in src/handlers/delete_session.rs accepting session_id and cleanup options
- [ ] T018 [US1] Update execute_command routing in src/server.rs to reject all commands except the three approved endpoints
- [ ] T019 [US1] Remove all deprecated handler modules and update src/handlers/mod.rs exports accordingly
- [ ] T020 [US1] Update JSON-RPC error codes in src/server.rs to return -32601 (Method not found) for removed endpoints
- [ ] T021 [US1] Add input validation for all three endpoints using serde validation patterns

**Checkpoint**: At this point, User Story 1 should be fully functional and testable independently

---

## Phase 4: User Story 2 - Delegate Session Management to Goose (Priority: P1)

**Goal**: Replace custom session management with Goose SessionManager, automatically creating sessions when needed

**Independent Test**: Verify that sessions are created using Goose's SessionManager, session data persists through Goose mechanisms, and no custom session persistence logic remains

### Implementation for User Story 2

- [ ] T022 [P] [US2] Create GooseSessionWrapper in src/agents/session_wrapper.rs implementing session creation with SessionManager::create_session()
- [ ] T023 [P] [US2] Implement session lookup logic in src/agents/session_wrapper.rs using SessionManager::get_session()
- [ ] T024 [P] [US2] Implement session deletion logic in src/agents/session_wrapper.rs using SessionManager::delete_session()
- [ ] T025 [US2] Integrate session wrapper into generate_fix handler in src/handlers/generate_fix.rs for create-or-reuse pattern
- [ ] T026 [US2] Integrate session wrapper into delete_session handler in src/handlers/delete_session.rs for cleanup operations
- [ ] T027 [US2] Add session locking mechanism in src/agents/session_wrapper.rs to prevent concurrent access (-32016 error)
- [ ] T028 [US2] Remove all custom session persistence code and update src/session.rs to only contain Goose delegation logic
- [ ] T029 [US2] Add session validation for client-generated UUIDs using uuid crate in src/models/configuration.rs

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently

---

## Phase 5: User Story 3 - Implement Proper Goose Agent Initialization (Priority: P1)

**Goal**: Correctly initialize Goose agents with proper configuration, default tools, custom tools, and planning mode

**Independent Test**: Verify that agents have access to expected tools, respect permission settings, and can operate in configured planning modes

### Implementation for User Story 3

- [ ] T030 [P] [US3] Create GooseAgentManager in src/agents/goose_integration.rs with Agent::new() initialization patterns
- [ ] T031 [P] [US3] Implement model provider setup in src/agents/goose_integration.rs using create_with_named_model() and agent.update_provider()
- [ ] T032 [P] [US3] Implement SessionConfig creation in src/agents/goose_integration.rs with session_id, max_turns, and retry_config mapping
- [ ] T033 [P] [US3] Add default tool configuration in src/agents/goose_integration.rs using ExtensionConfig::stdio() for developer, todo, extensionmanager
- [ ] T034 [US3] Implement permission enforcement wrapper in src/agents/goose_integration.rs mapping tool_permissions to Goose's permission system
- [ ] T035 [US3] Add custom tool support in src/agents/goose_integration.rs using ExtensionConfig for MCP extensions
- [ ] T036 [US3] Implement planning mode configuration in src/agents/goose_integration.rs based on AgentConfiguration.tools.planning_mode
- [ ] T037 [US3] Integrate GooseAgentManager into generate_fix handler in src/handlers/generate_fix.rs for agent creation and execution
- [ ] T038 [US3] Add agent initialization error handling in src/agents/goose_integration.rs with proper error codes (-32006)

**Checkpoint**: At this point, User Stories 1, 2, AND 3 should all work independently

---

## Phase 6: User Story 4 - Stream Agent Events to Clients (Priority: P2)

**Goal**: Forward Goose agent events to connected clients in real-time for responsive user interfaces

**Independent Test**: Verify that all relevant events are forwarded to clients with appropriate timing and detail during agent processing

### Implementation for User Story 4

- [ ] T039 [P] [US4] Create event streaming handler in src/agents/event_streaming.rs mapping Goose AgentEvent to Kaiak notification formats
- [ ] T040 [P] [US4] Implement progress notification mapping in src/agents/event_streaming.rs for kaiak/stream/progress method
- [ ] T041 [P] [US4] Implement AI response notification mapping in src/agents/event_streaming.rs for kaiak/stream/ai_response method
- [ ] T042 [P] [US4] Implement tool call notification mapping in src/agents/event_streaming.rs for kaiak/stream/tool_call method
- [ ] T043 [P] [US4] Implement user interaction notification mapping in src/agents/event_streaming.rs for kaiak/stream/user_interaction method
- [ ] T044 [P] [US4] Implement file modification notification mapping in src/agents/event_streaming.rs for kaiak/stream/file_modification method
- [ ] T045 [P] [US4] Implement error notification mapping in src/agents/event_streaming.rs for kaiak/stream/error method
- [ ] T046 [P] [US4] Implement system notification mapping in src/agents/event_streaming.rs for kaiak/stream/system method
- [ ] T047 [US4] Integrate event streaming into generate_fix handler in src/handlers/generate_fix.rs using agent.reply() stream
- [ ] T048 [US4] Add user interaction response handling in src/agents/event_streaming.rs for tool approval workflows
- [ ] T049 [US4] Implement proper stream cleanup and error recovery in src/agents/event_streaming.rs

**Checkpoint**: All user stories should now be independently functional

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories and final cleanup

- [ ] T050 [P] Remove custom session management tests from tests/integration/session_crud.rs per plan.md removed tests section
- [ ] T051 [P] Remove deprecated endpoint tests from tests/integration/old_endpoints.rs per plan.md removed tests section
- [ ] T052 [P] Update test configuration files to exclude removed test modules and ensure clean test execution
- [ ] T053 [P] Create integration tests in tests/integration/goose_session.rs for SessionManager integration validation
- [ ] T054 [P] Create integration tests in tests/integration/agent_lifecycle.rs for agent initialization and tool availability
- [ ] T055 [P] Create integration tests in tests/integration/api_endpoints.rs for three-endpoint API validation
- [ ] T056 [P] Create integration tests in tests/integration/event_streaming.rs for Goose event to Kaiak notification mapping
- [ ] T057 [P] Update development setup instructions in README.md for Goose dependency requirements
- [ ] T058 [P] Add performance benchmarking in tests/benchmark/ for startup time and memory usage improvements
- [ ] T059 [P] Update README.md API documentation section to specify three-endpoint interface per FR-020
- [ ] T060 [P] Update API examples in docs/ to demonstrate configure(), generate_fix(), and delete_session() usage patterns per FR-020
- [ ] T061 [P] Create JSON-RPC API reference documentation in docs/api-reference.md based on contracts/jsonrpc-api.md per FR-020
- [ ] T062 Code cleanup: Remove all unused imports and dead code from refactored modules
- [ ] T063 Run cargo clippy and fix all warnings for final code quality validation
- [ ] T064 Validate quickstart.md examples work with implemented three-endpoint API

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-6)**: All depend on Foundational phase completion
  - User Stories 1, 2, 3 are P1 priority and should be completed first
  - User Story 4 is P2 priority and can be done after P1 stories
  - User stories can proceed in parallel (if staffed)
  - Or sequentially in priority order (P1 → P1 → P1 → P2)
- **Polish (Phase 7)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 3 (P1)**: Can start after Foundational (Phase 2) - Integrates with US1 and US2 but independently testable
- **User Story 4 (P2)**: Can start after Foundational (Phase 2) - Integrates with US3 for agent events but independently testable

### Within Each User Story

- Tasks marked [P] within a story can run in parallel
- Core implementation before integration
- Handler implementation before routing integration
- Story complete before moving to next priority

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel
- All Foundational tasks marked [P] can run in parallel (within Phase 2)
- Once Foundational phase completes, User Stories 1-3 can start in parallel (if team capacity allows)
- All tasks within a user story marked [P] can run in parallel
- Different user stories can be worked on in parallel by different team members

---

## Parallel Example: User Story 1

```bash
# Launch all parallel tasks for User Story 1 together:
Task: "Implement configure() handler in src/handlers/configure.rs"
Task: "Implement generate_fix() handler in src/handlers/generate_fix.rs"
Task: "Implement delete_session() handler in src/handlers/delete_session.rs"
```

---

## Implementation Strategy

### MVP First (User Stories 1-3 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1 (Simplify API Surface)
4. Complete Phase 4: User Story 2 (Delegate Session Management)
5. Complete Phase 5: User Story 3 (Proper Agent Initialization)
6. **STOP and VALIDATE**: Test all three P1 user stories work together
7. Deploy/demo if ready

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → Deploy/Demo (Basic API!)
3. Add User Story 2 → Test independently → Deploy/Demo (Session Management!)
4. Add User Story 3 → Test independently → Deploy/Demo (Full Agent Integration!)
5. Add User Story 4 → Test independently → Deploy/Demo (Complete!)
6. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 (API endpoints)
   - Developer B: User Story 2 (Session delegation)
   - Developer C: User Story 3 (Agent initialization)
3. Stories complete and integrate independently
4. Team works on User Story 4 together (event streaming)

---

## Summary

- **Total Tasks**: 64 tasks across 7 phases
- **Task Count per User Story**:
  - US1: 7 tasks (API surface simplification)
  - US2: 8 tasks (Session management delegation)
  - US3: 9 tasks (Agent initialization)
  - US4: 11 tasks (Event streaming)
- **Parallel Opportunities**: 44 tasks marked [P] can run concurrently
- **Independent Test Criteria**: Each user story has clear validation criteria
- **Suggested MVP Scope**: User Stories 1-3 (P1 priority) provide core functionality
- **Format Validation**: All tasks follow required checklist format with ID, labels, and file paths

## Notes

- [P] tasks = different files, no dependencies within the phase
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Focus on Goose integration patterns from research.md and concrete examples from quickstart.md
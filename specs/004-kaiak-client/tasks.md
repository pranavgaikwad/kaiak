# Tasks: Kaiak Client Implementation

**Input**: Design documents from `/specs/004-kaiak-client/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3, US4)
- Include exact file paths in descriptions

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and configuration unification foundation

- [ ] T001 Delete src/config/ directory and consolidate all configuration logic
- [ ] T002 [P] Create src/logging.rs by moving logging setup from src/config/logging.rs
- [ ] T003 [P] Create src/models/client.rs for ClientConnection and ClientState entities
- [ ] T004 [P] Create src/client/ module directory structure

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [ ] T005 Move src/config/settings.rs content to src/models/configuration.rs with unified ServerConfig structure
- [ ] T006 [P] Implement ConfigurationHierarchy in src/models/configuration.rs with precedence loading (CLI > user config > defaults)
- [ ] T007 [P] Add ClientError variants to src/models/errors.rs for connection and validation failures
- [ ] T008 [P] Create src/client/transport.rs with JsonRpcClient for Unix socket communication
- [ ] T009 Fix handler wiring in src/server/server.rs to connect existing handlers to server transport
- [ ] T010 [P] Implement ClientState persistence methods (load, save, connect, disconnect) in src/models/client.rs
- [ ] T011 [P] Add JSON-RPC client request/response handling in src/client/transport.rs

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Server Management (Priority: P1) 🎯 MVP

**Goal**: Developers can start and configure a Kaiak server instance with various configuration options to support different deployment scenarios and development workflows.

**Independent Test**: Can be fully tested by starting a server with different configuration options and verifying it responds to health checks, delivering a functional server ready for client connections.

### Implementation for User Story 1

- [ ] T012 [P] [US1] Update src/cli/commands/serve.rs to use unified ServerConfig and ConfigurationHierarchy
- [ ] T013 [P] [US1] Add --socket option support for Unix socket transport in src/cli/commands/serve.rs
- [ ] T014 [US1] Update src/main.rs CLI argument parsing to support new serve command options with configuration precedence
- [ ] T015 [US1] Implement server startup with InitConfig validation and BaseConfig loading in src/cli/commands/serve.rs
- [ ] T016 [US1] Add configuration file loading from ~/.kaiak/server.conf with TOML format support
- [ ] T017 [US1] Add CLI override processing to ensure CLI arguments take precedence over config file settings

**Checkpoint**: At this point, User Story 1 should be fully functional and testable independently

---

## Phase 4: User Story 2 - Client Connection Management (Priority: P2)

**Goal**: Developers need to connect a client to an existing Kaiak server and maintain that connection state for subsequent operations without having to specify connection details repeatedly.

**Independent Test**: Can be fully tested by establishing a connection to a running server, verifying connection state persistence, and confirming the connection can be cleanly disconnected.

### Implementation for User Story 2

- [ ] T018 [P] [US2] Create src/cli/commands/connect.rs with connection establishment and validation
- [ ] T019 [P] [US2] Create src/cli/commands/disconnect.rs with connection cleanup and state removal
- [ ] T020 [US2] Implement client state persistence in ~/.kaiak/client.state with JSON format
- [ ] T021 [US2] Add connection validation with socket existence and server communication checks
- [ ] T022 [US2] Update src/main.rs to add connect and disconnect subcommands to CLI parser
- [ ] T023 [US2] Implement user-friendly error messages for connection failures with retry suggestions

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently

---

## Phase 5: User Story 3 - Remote Procedure Execution (Priority: P3)

**Goal**: Developers need to execute Kaiak procedures remotely through the client, passing parameters via files or inline JSON, to perform AI-powered code analysis and fixes.

**Independent Test**: Can be fully tested by connecting to a server and successfully executing each procedure type (generate_fix, configure, delete_session) with various input methods.

### Implementation for User Story 3

- [ ] T024 [P] [US3] Create src/cli/commands/client_ops.rs with remote configure procedure implementation
- [ ] T025 [P] [US3] Implement remote generate_fix procedure with AgentConfig handling in src/cli/commands/client_ops.rs
- [ ] T026 [P] [US3] Implement remote delete_session procedure with session ID conversion in src/cli/commands/client_ops.rs
- [ ] T027 [US3] Add --input and --input-json argument parsing support for all procedure commands
- [ ] T028 [US3] Update src/main.rs to add configure, generate_fix, and delete_session subcommands
- [ ] T029 [US3] Implement JSON-RPC request formatting and response handling for all three procedures
- [ ] T030 [US3] Add connection requirement validation for all remote procedure commands

**Checkpoint**: All user stories should now be independently functional

---

## Phase 6: User Story 4 - Global CLI Features (Priority: P4)

**Goal**: Developers need access to standard CLI utilities like version information, logging configuration, and shell completion to integrate Kaiak into their development workflows effectively.

**Independent Test**: Can be fully tested by verifying each global option works correctly regardless of client/server state.

### Implementation for User Story 4

- [ ] T031 [P] [US4] Add --version global option with build information display in src/main.rs
- [ ] T032 [P] [US4] Add --log-level and --log-file global options with logging configuration in src/main.rs
- [ ] T033 [P] [US4] Implement --completion option for shell completion script generation in src/main.rs
- [ ] T034 [US4] Update global option handling to work across all commands (serve, connect, client operations)
- [ ] T035 [US4] Add help text and usage examples for all new CLI commands and options

**Checkpoint**: Complete CLI feature set is now available

---

## Phase 7: Integration & Testing

**Purpose**: Comprehensive testing and integration validation

- [ ] T036 [P] Create tests/test_client.rs with integration tests for all client-server communication scenarios
- [ ] T037 [P] Add configuration precedence tests validating CLI > user config > defaults hierarchy
- [ ] T038 [P] Add connection state persistence tests across terminal sessions
- [ ] T039 [P] Add error handling tests for connection failures and invalid inputs
- [ ] T040 [P] Add end-to-end workflow tests covering server startup, client connection, and remote procedures
- [ ] T041 [P] Update existing tests to ensure compatibility with configuration unification changes
- [ ] T042 [P] Add performance tests to validate <30s server startup and <2s client command execution requirements

---

## Phase 8: Polish & Documentation

**Purpose**: Final improvements and documentation updates

- [ ] T043 [P] Update README.md with client-server workflow examples and installation instructions
- [ ] T044 [P] Update CLI help text with comprehensive examples and troubleshooting guidance
- [ ] T045 [P] Create configuration examples for ~/.kaiak/server.conf TOML format
- [ ] T046 [P] Add troubleshooting documentation for common connection and configuration issues
- [ ] T047 [P] Run quickstart.md validation scenarios to ensure documented workflows work correctly
- [ ] T048 Code cleanup and remove any remaining placeholder implementations identified in research

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-6)**: All depend on Foundational phase completion
  - User stories can then proceed in parallel (if staffed)
  - Or sequentially in priority order (P1 → P2 → P3 → P4)
- **Integration & Testing (Phase 7)**: Depends on desired user stories being complete
- **Polish (Phase 8)**: Depends on all implementation phases being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P2)**: Can start after Foundational (Phase 2) - Independent of other stories but may integrate with US1
- **User Story 3 (P3)**: Requires US2 (client connection) to be functional for remote procedures
- **User Story 4 (P4)**: Can start after Foundational (Phase 2) - Independent of other stories

### Within Each User Story

- Configuration tasks before CLI implementation
- Core models and transport before command implementations
- Command logic before CLI integration
- Error handling and validation after core functionality

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel (T002, T003, T004)
- All Foundational tasks marked [P] can run in parallel within Phase 2
- Models, transport, and CLI command files can be developed in parallel within each user story
- Different user stories can be worked on in parallel by different team members after foundational completion

---

## Parallel Example: User Story 1

```bash
# Launch all parallel tasks for User Story 1 together:
Task: "Update src/cli/commands/serve.rs to use unified ServerConfig and ConfigurationHierarchy"
Task: "Add --socket option support for Unix socket transport in src/cli/commands/serve.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T004)
2. Complete Phase 2: Foundational (T005-T011) - CRITICAL - blocks all stories
3. Complete Phase 3: User Story 1 (T012-T017)
4. **STOP and VALIDATE**: Test server management independently
5. Deploy/demo server management capabilities

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → Deploy/Demo (Server management MVP!)
3. Add User Story 2 → Test independently → Deploy/Demo (Client connection added)
4. Add User Story 3 → Test independently → Deploy/Demo (Remote procedures added)
5. Add User Story 4 → Test independently → Deploy/Demo (Complete CLI features)
6. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 (Server Management)
   - Developer B: User Story 2 (Client Connection)
   - Developer C: User Story 4 (Global CLI Features) - can work in parallel with others
3. User Story 3 (Remote Procedures) starts after User Story 2 completion
4. Stories complete and integrate independently

---

## Notes

- **[P]** tasks = different files, no dependencies on incomplete work
- **[Story]** label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Configuration unification in Phase 2 is critical for all subsequent work
- Client-server communication requires both transport layer and CLI command implementation
- User Story 3 depends on User Story 2 for connection management
- Commit after each task or logical group of related tasks
- Stop at any checkpoint to validate story independently
- All JSON-RPC communication follows contracts/jsonrpc-api.json specification

**Total Tasks**: 48 tasks across 8 phases
**Parallel Opportunities**: 32 tasks can run in parallel within their phases
**MVP Scope**: Phases 1-3 (User Story 1 - Server Management) - 17 tasks
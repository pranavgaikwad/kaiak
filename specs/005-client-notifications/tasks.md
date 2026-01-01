# Tasks: Client-to-Server Notifications

**Input**: Design documents from `/specs/005-client-notifications/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Tests are OPTIONAL for this feature - integration tests will cover socket communication flows as specified in the constitution.

**Organization**: Tasks are grouped by user story for clear traceability and focused implementation.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Single project**: `src/`, `tests/` at repository root
- Paths assume existing Kaiak codebase structure per plan.md

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and validation

- [ ] T001 Validate existing Kaiak JSON-RPC infrastructure in src/jsonrpc/protocol.rs
- [ ] T002 [P] Verify existing socket transport capabilities in src/client/transport.rs
- [ ] T003 [P] Review existing session management integration points

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before user story implementation

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [ ] T004 Create client notification handler structure in src/handlers/client_notifications.rs
- [ ] T005 Register kaiak/client/user_message method in src/jsonrpc/methods.rs
- [ ] T006 Update handler module exports in src/handlers/mod.rs
- [ ] T007 Implement notification validation framework using existing JSON-RPC error patterns

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Interactive User Input During Agent Processing (Priority: P1) 🎯 MVP

**Goal**: Enable client to send notifications to server via socket with validation and session ID routing

### Implementation for User Story 1

- [ ] T008 [P] [US1] Extend JsonRpcClient in src/client/transport.rs to add send_notification method
- [ ] T009 [P] [US1] Implement notification parameter validation in src/handlers/client_notifications.rs
- [ ] T010 [US1] Add client notification handler to JSON-RPC server in src/jsonrpc/server.rs
- [ ] T011 [US1] Implement session ID validation using existing agent session management
- [ ] T012 [US1] Add notification size validation (1MB)
- [ ] T013 [US1] Implement retry queue and exponential backoff for connection failures
- [ ] T015 [US1] Add comprehensive error handling for all validation scenarios
- [ ] T016 [US1] Add logging for client notification operations using existing patterns
- [ ] T017 [US1] Create integration test for client-to-server notification flow in tests/integration/

**Checkpoint**: At this point, User Story 1 should be fully functional - clients can send notifications to server with validation

---

## Phase 4: Polish & Cross-Cutting Concerns

**Purpose**: Improvements and final validation

- [ ] T018 [P] Validate quickstart.md examples work with implemented notification system
- [ ] T020 Code cleanup and ensure consistent error message formats
- [ ] T022 Security review of notification validation and session handling
- [ ] T023 Documentation update to add the new notification type

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Foundational phase completion
- **Polish (Phase 4)**: Depends on User Story 1 completion

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories

### Within User Story 1

- Client transport extension (T008) can run parallel with notification validation (T009)
- Validation framework (T007) must complete before notification validation (T009)
- Server integration (T010) depends on client extension (T008) and validation (T009)
- Session validation (T011) can run parallel with server integration (T010)
- Rate limiting (T012) depends on validation framework (T007)
- Error handling (T015) depends on all validation components (T009, T011, T012)
- Integration tests (T017) depend on complete feature implementation

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel
- All Foundational tasks can run sequentially (small, focused tasks)
- Within User Story 1: T008 and T009 can run in parallel
- T011 can run parallel with T010
- All Polish tasks marked [P] can run in parallel

---

## Parallel Example: User Story 1

```bash
# Launch client and validation work in parallel:
Task: "Extend JsonRpcClient in src/client/transport.rs to add send_notification method"
Task: "Implement notification parameter validation in src/handlers/client_notifications.rs"

# After validation framework is ready, these can run in parallel:
Task: "Add client notification handler to JSON-RPC server in src/jsonrpc/server.rs"
Task: "Implement session ID validation using existing agent session management"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: Verify client can send notifications and server receives/validates them
5. Ready for deployment/demo

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Complete User Story 1 → Deploy/Demo (MVP!)
3. This feature has only one user story - ready for use

### Testing Strategy

- Integration tests focus on socket communication flows per constitution
- Test notification sending, validation, error handling, and session routing
- Validate quickstart examples work end-to-end
- Performance testing for rate limiting and throughput

---

## Notes

- [P] tasks = different files, no dependencies on incomplete tasks
- [US1] label maps task to specific user story for traceability
- Feature extends existing infrastructure with minimal architectural changes
- Leverages existing JSON-RPC patterns and session management
- No new dependencies required - uses tokio, serde, and existing infrastructure
- Notification routing validates session ID but does not forward to agents per requirements
- Focus on essential notification infrastructure only
# Implementation Tasks: Agent Implementation

**Feature**: Agent Implementation (002)
**Branch**: `002-agent-implementation`
**Generated**: 2025-12-23
**Input**: [spec.md](./spec.md), [plan.md](./plan.md)

---

## Task Organization

**User Stories Priority**:
- **P1**: Basic Agent Processing (Foundational capability)
- **P2**: Real-time Response Streaming (Essential UX)
- **P3**: Tool Call Execution and Monitoring (Full functionality)

**Execution Strategy**: Sequential by priority group, with parallel execution within groups where dependencies allow.

---

## P1: Basic Agent Processing Tasks

### T001 - Enhance Goose Agent Integration [P1]
**Story**: P1 - Basic Agent Processing
**Depends**: None
**Parallel**: Can run with T002 (incident handling)

**Description**: Implement actual Goose Agent integration in existing skeleton

**Files**:
- `src/goose/agent.rs` - Replace simulated processing with real Goose agent calls
- `src/goose/session.rs` - Wire actual Goose Agent in GooseSessionWrapper
- `Cargo.toml` - Verify goose dependency configuration

**Acceptance**:
- [X] GooseSessionWrapper.initialize() creates actual Goose Agent instance
- [X] Agent processing uses real Goose agent instead of simulation
- [X] Agent state properly tracked through session lifecycle
- [X] Error handling preserves existing graceful degradation patterns

---

### T002 - Implement Incident-to-Prompt Conversion [P1]
**Story**: P1 - Basic Agent Processing
**Depends**: None
**Parallel**: Can run with T001 (agent integration)

**Description**: Convert incident data to natural language prompts for Goose agent

**Files**:
- `src/goose/prompts.rs` - Implement format_incident_prompt() method
- `src/models/incident.rs` - Validate incident structure compatibility

**Acceptance**:
- [X] format_incident_prompt() converts incident array to coherent natural language
- [X] Prompt includes file paths, line numbers, issue descriptions, and context
- [X] Format follows Goose agent input expectations
- [X] Handles edge cases (empty incidents, malformed data)

---

### T003 - Wire Agent Processing Pipeline [P1]
**Story**: P1 - Basic Agent Processing
**Depends**: T001, T002
**Parallel**: None (requires both previous tasks)

**Description**: Connect incident processing through Goose agent workflow

**Files**:
- `src/goose/agent.rs` - Update process_fix_request() to use real agent
- `src/handlers/fix_generation.rs` - Verify integration with existing handlers

**Acceptance**:
- [X] process_fix_request() sends formatted prompts to Goose agent
- [X] Agent responses properly captured and structured
- [X] Session state updates reflect actual processing progress
- [X] Error scenarios handled with meaningful messages (FR-006)

---

### T004 - Basic Integration Test Implementation [P1]
**Story**: P1 - Basic Agent Processing
**Depends**: T003
**Parallel**: None (requires working pipeline)

**Description**: Create comprehensive end-to-end test with TestProvider integration

**Files**:
- `tests/integration/goose_integration.rs` - Implement comprehensive e2e test
- `tests/fixtures/` - Create sample incidents and test workspace
- `tests/recordings/` - Set up TestProvider recording infrastructure

**Acceptance**:
- [X] Test creates agent session with sample incidents
- [X] Processes incidents through complete Goose agent workflow
- [X] Verifies agent responses and file modifications on disk
- [X] Uses TestProvider for CI/PR compatibility (recording/replay)
- [X] Achieves target processing time <30s (SC-001)

---

## P2: Real-time Response Streaming Tasks

### T005 - Implement Goose Event Bridge [P2]
**Story**: P2 - Real-time Response Streaming
**Depends**: T001 (requires agent integration)
**Parallel**: Can run with T006 (message conversion)

**Description**: Bridge Goose agent events to Kaiak streaming system

**Files**:
- `src/goose/monitoring.rs` - Implement GooseEventBridge for event subscription
- `src/goose/session.rs` - Connect event bridge to agent sessions

**Acceptance**:
- [ ] GooseEventBridge subscribes to Goose agent event stream
- [ ] Events properly forwarded to message callback infrastructure
- [ ] Event subscription lifecycle matches agent session lifecycle
- [ ] Handles connection interruptions gracefully

---

### T006 - Implement Event-to-Message Conversion [P2]
**Story**: P2 - Real-time Response Streaming
**Depends**: None
**Parallel**: Can run with T005 (event bridge)

**Description**: Convert Goose agent events to Kaiak streaming messages

**Files**:
- `src/goose/monitoring.rs` - Implement convert_goose_event() method
- `src/models/messages.rs` - Verify message format compatibility

**Acceptance**:
- [ ] AgentEvent::Message converts to MessageContent::AiResponse
- [ ] AgentEvent::ToolCall converts to MessageContent::ToolCall
- [ ] AgentEvent::Thinking maps to appropriate message types
- [ ] Unsupported events logged for feature gap documentation (FR-010)

---

### T007 - Wire Streaming Infrastructure [P2]
**Story**: P2 - Real-time Response Streaming
**Depends**: T005, T006
**Parallel**: None (requires event bridge and conversion)

**Description**: Connect Goose events to existing streaming handlers

**Files**:
- `src/handlers/streaming.rs` - Connect to Goose event stream
- `src/goose/session.rs` - Set up message callback integration

**Acceptance**:
- [ ] Streaming messages delivered in real-time during processing
- [ ] Message timestamps and sequence numbers properly maintained
- [ ] Streaming latency <500ms target achieved (SC-002)
- [ ] Connection state properly managed throughout session lifecycle

---

### T008 - Streaming Integration Tests [P2]
**Story**: P2 - Real-time Response Streaming
**Depends**: T007, T004 (requires basic integration)
**Parallel**: None (requires working streaming)

**Description**: Validate streaming performance and reliability

**Files**:
- `tests/integration/streaming.rs` - Enhance existing streaming tests
- `tests/integration/goose_integration.rs` - Add streaming validation to e2e test

**Acceptance**:
- [ ] Streaming events captured throughout agent processing
- [ ] Latency measurements verify <500ms target (SC-002)
- [ ] Progress updates received in real-time
- [ ] Final completion message properly delivered

---

## P3: Tool Call Execution and Monitoring Tasks

### T009 - Implement Tool Call Interception [P3]
**Story**: P3 - Tool Call Execution and Monitoring
**Depends**: T001 (requires agent integration)
**Parallel**: Can run with T010 (tool safety)

**Description**: Capture and intercept Goose agent tool calls

**Files**:
- `src/goose/agent.rs` - Implement handle_goose_tool_call() method
- `src/goose/session.rs` - Wire tool call event handling

**Acceptance**:
- [ ] Goose tool calls properly intercepted and captured
- [ ] Tool call parameters and metadata preserved
- [ ] Tool execution state tracked through completion
- [ ] Tool results properly returned to Goose agent

---

### T010 - Integrate Tool Safety Infrastructure [P3]
**Story**: P3 - Tool Call Execution and Monitoring
**Depends**: None
**Parallel**: Can run with T009 (tool interception)

**Description**: Wire existing safety infrastructure with Goose tool calls

**Files**:
- `src/goose/agent.rs` - Use existing create_safe_tool_call() method
- `src/handlers/interactions.rs` - Verify approval workflow compatibility

**Acceptance**:
- [ ] SafeToolCallResult::Allowed tools execute directly
- [ ] SafeToolCallResult::InterceptedForApproval triggers user interaction
- [ ] Approval workflow properly integrated with Goose agent
- [ ] Tool call safety rules enforced consistently

---

### T011 - Tool Result Management [P3]
**Story**: P3 - Tool Call Execution and Monitoring
**Depends**: T009, T010
**Parallel**: None (requires tool interception and safety)

**Description**: Manage tool execution results and feedback to agent

**Files**:
- `src/goose/agent.rs` - Implement send_tool_result_to_goose() method
- `src/models/messages.rs` - Ensure tool result message compatibility

**Acceptance**:
- [ ] Tool execution results properly formatted for Goose agent
- [ ] Success/failure status clearly communicated
- [ ] Tool output captured and logged for monitoring
- [ ] Agent workflow continues appropriately after tool completion

---

### T012 - Tool Call Integration Tests [P3]
**Story**: P3 - Tool Call Execution and Monitoring
**Depends**: T011, T008 (requires tool management and streaming)
**Parallel**: None (requires complete tool infrastructure)

**Description**: Validate complete tool call workflow

**Files**:
- `tests/integration/goose_integration.rs` - Add tool call validation to e2e test
- `tests/fixtures/` - Create incidents that trigger tool usage

**Acceptance**:
- [ ] Tool calls captured and logged during agent processing (SC-004)
- [ ] Tool execution completes with proper result capture
- [ ] Approval workflow tested when tool interception triggered
- [ ] File modifications properly applied and tracked

---

## Cross-Cutting Tasks

### T013 - Feature Gap Documentation [Cross-Cutting]
**Story**: All Stories (FR-010, FR-011)
**Depends**: T005, T006 (requires event observation)
**Parallel**: Can run with other implementation tasks

**Description**: Document Goose-to-IDE message format differences

**Files**:
- `docs/goose_ide_compatibility.md` - Create comprehensive compatibility analysis
- `src/goose/monitoring.rs` - Log unsupported features during processing

**Acceptance**:
- [ ] Advanced Goose features identified and catalogued
- [ ] Message format differences documented with examples
- [ ] IDE enhancement requirements clearly specified
- [ ] Session support gaps documented for future implementation

---

### T014 - TestProvider Infrastructure [Cross-Cutting]
**Story**: All Stories (FR-009)
**Depends**: None
**Parallel**: Can run with all implementation tasks

**Description**: Set up comprehensive recording/replay testing infrastructure

**Files**:
- `tests/integration/goose_integration.rs` - Implement TestProvider configuration
- `tests/recordings/` - Create and manage recording files
- `.github/workflows/` - Configure CI for replay-only mode

**Acceptance**:
- [ ] TestProvider records real model interactions in development
- [ ] CI/PR tests replay recordings without requiring API keys
- [ ] Recording safety guards prevent accidental recording in CI
- [ ] Test success rate achieves 95% target across scenarios (SC-003)

---

### T015 - Performance Validation [Cross-Cutting]
**Story**: All Stories (SC-001, SC-002)
**Depends**: T012 (requires complete implementation)
**Parallel**: None (requires working system)

**Description**: Validate performance metrics and success criteria

**Files**:
- `tests/integration/goose_integration.rs` - Add performance measurement
- `src/goose/monitoring.rs` - Implement performance tracking

**Acceptance**:
- [ ] Processing time consistently <30s for simple cases (SC-001)
- [ ] Streaming latency consistently <500ms (SC-002)
- [ ] Performance metrics logged and tracked
- [ ] Success rate validation across varied scenarios (SC-003)

---

## Execution Plan

### Phase 1: Foundation (P1 Tasks)
**Execute**: T001 || T002 → T003 → T004
**Timeline**: Core agent integration and basic workflow
**Validation**: Basic agent processing works end-to-end

### Phase 2: Streaming (P2 Tasks)
**Execute**: (T005 || T006) → T007 → T008
**Timeline**: Real-time event streaming implementation
**Validation**: Live progress updates during agent processing

### Phase 3: Tools (P3 Tasks)
**Execute**: (T009 || T010) → T011 → T012
**Timeline**: Complete tool call workflow
**Validation**: File modifications executed and tracked

### Phase 4: Polish (Cross-Cutting Tasks)
**Execute**: T013 || T014 || T015
**Timeline**: Documentation, testing infrastructure, validation
**Validation**: All success criteria met, feature gaps documented

---

## Success Validation

**SC-001**: Processing time <30s → Measured in T004, T015
**SC-002**: Streaming latency <500ms → Measured in T008, T015
**SC-003**: 95% test success rate → Validated in T014, T015
**SC-004**: Tool call capture 100% → Validated in T012
**SC-005**: Error handling 100% → Validated throughout all tasks
**SC-006**: Goose compatibility demonstrated → Validated in T004
**SC-007**: Feature gap documentation → Delivered in T013

---

## Risk Mitigation

**Goose API Changes**: Use git dependency with pinned commit until stable
**TestProvider Reliability**: Implement fallback recording mechanisms
**Streaming Performance**: Monitor latency in real-time, implement buffering if needed
**Tool Safety Integration**: Thoroughly test approval workflows before file modifications
**CI/PR Compatibility**: Ensure recordings are comprehensive and deterministic

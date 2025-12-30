<!--
  SYNC IMPACT REPORT
  Version change: 1.0.1 → 2.0.0
  Modified principles:
    - I. User Experience First → removed progress feedback requirements
    - II. Testing Integrity → removed test-first requirement
    - VI. Progressive Development → removed independent testability requirement
    - VII. Goose API Primacy → new principle added
  Added sections: VII. Goose API Primacy
  Removed sections: Quality Assurance > Progress Transparency
  Templates requiring updates:
    ✅ .specify/templates/tasks-template.md - removed test-first and independent testing references
    ✅ .specify/templates/spec-template.md - updated independent testing requirements to verification notes
    ✅ .specify/templates/plan-template.md - Constitution Check section aligns
  Follow-up TODOs: None
-->

# Kaiak Constitution

## Core Principles

### I. User Experience First

All features MUST prioritize user experience through clear error messages and comprehensive logging. Error messages MUST be actionable and include sufficient context for debugging. Every user-facing interaction MUST be thoroughly logged for troubleshooting support.

### II. Testing Integrity (NON-NEGOTIABLE)

Integration and end-to-end tests MUST take precedence over unit tests. All critical user journeys MUST have corresponding integration tests. Unit tests MUST be written for complex functions and core business logic only. The testing pyramid prioritizes e2e → integration → unit testing coverage.

### III. Enterprise-Safe Communication

All inter-process communication MUST use enterprise-safe IPC mechanisms: Unix domain sockets, named pipes, or stdio. Network ports are FORBIDDEN to ensure compatibility with corporate firewall and security policies. Communication protocols MUST be bidirectional, streaming-capable, and resilient to process failures with automatic reconnection capabilities.

### IV. Code Quality Standards

Source code MUST maintain high quality through automated linting, formatting, and static analysis. Comments MUST be added only for complex functions and critical business logic - trivial code MUST remain self-documenting. Code reviews MUST verify compliance with quality standards. All code MUST follow Rust best practices and idiomatic patterns.

### V. Continuous Integration

All pull requests MUST pass automated testing via GitHub Actions before merge approval. CI pipelines MUST include testing, linting, security scanning, and code quality checks. Broken CI builds MUST be treated as high-priority issues requiring immediate resolution. No manual testing gates may substitute for automated CI validation.

### VI. Progressive Development

Features MUST be developed incrementally with each increment delivering measurable user value. Complex features MUST be broken into smaller, valuable user stories. Development MUST follow the principle of building working software over comprehensive documentation.

### VII. Goose API Primacy

All functionality MUST leverage existing Goose APIs and capabilities wherever possible. Creating new implementations that duplicate Goose functionality is FORBIDDEN unless absolutely necessary for integration requirements. When Goose provides a solution, it MUST be used instead of building custom alternatives. New functionality MUST be justified against available Goose capabilities with clear rationale for why Goose's approach is insufficient.

## Development Standards

### Rust Implementation Requirements

- All server components MUST be implemented in Rust using stable toolchain
- Error handling MUST use Result types and avoid panics in production code
- Dependencies MUST be minimal and justify their inclusion
- Performance-critical code MUST include benchmarks and performance regression tests
- Memory safety MUST be verified through compiler checks and additional tooling when needed

### Code Organization

- Server architecture MUST follow modular design with clear separation of concerns
- Public APIs MUST be well-documented with examples
- Internal modules MUST have clear ownership and single responsibility
- Configuration MUST be externalized and environment-aware
- Dependency injection MUST be used for testability

### IPC Architecture Requirements

- Communication channels MUST handle message serialization/deserialization transparently
- Protocols MUST support streaming for large data transfers
- Connection management MUST implement heartbeat and automatic reconnection
- Message formats MUST be versioned for backward compatibility
- Error propagation across IPC boundaries MUST preserve context and stack traces

## Quality Assurance

### Error Handling Standards

Error messages MUST include context, suggested actions, and relevant error codes. Errors MUST be logged with sufficient detail for reproduction and debugging. User-facing errors MUST be translated to plain language while preserving technical details in logs. Recovery suggestions MUST be provided when possible.

### Observability Requirements

Structured logging MUST be implemented throughout the system using consistent formats. Key business operations MUST emit metrics for monitoring. Distributed tracing MUST be implemented for multi-service interactions. Log levels MUST be appropriate: ERROR for failures, WARN for degraded conditions, INFO for business events, DEBUG for troubleshooting.

## Governance

Constitution supersedes all other development practices and team agreements. Amendments require documentation of rationale, team review, and migration plan for existing code. All pull requests MUST verify constitutional compliance before approval. Violations of NON-NEGOTIABLE principles MUST be rejected regardless of business pressure. Complexity introduced must be explicitly justified against simpler alternatives.

Development teams MUST reference this constitution during planning, implementation, and review phases. Project architectural decisions MUST be evaluated against these principles. Any deviation from constitutional requirements MUST be documented with rationale and remediation timeline.

For additional background and detailed context regarding this project, refer to [./context.md](./context.md).

**Version**: 2.0.0 | **Ratified**: 2025-12-23 | **Last Amended**: 2025-12-30
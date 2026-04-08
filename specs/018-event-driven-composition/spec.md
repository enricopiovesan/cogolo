# Feature Specification: Event-Driven Composition

**Feature Branch**: `018-event-driven-composition`  
**Created**: 2026-03-30  
**Status**: Draft  
**Input**: Issue `#36`, the approved foundation, event-contract, and workflow-traversal slices, plus the agreed decisions for governed event-driven workflow progression.

## Purpose

This specification defines the first governed event-driven composition slice for Traverse.

It narrows the broader future event-orchestration direction into one focused model for:

- governed internal event-triggered workflow progression
- explicit event-driven workflow edge semantics
- exact event identity and version references on waiting edges
- deterministic consumption and wake ordering
- simple payload matching predicates
- dedicated trace evidence for event-triggered wake-up decisions

This slice exists so Traverse can advance already-running workflow executions from governed internal events without introducing external brokers, event-created executions, direct-capability waiting semantics, or a new top-level runtime state model.

## User Scenarios and Testing

### User Story 1 - Advance a Waiting Workflow from a Governed Internal Event (Priority: P1)

As a platform developer, I want a running workflow to advance when a governed internal event matches a declared event-driven edge so that event-based orchestration remains explicit and deterministic.

**Why this priority**: Event-driven composition is the missing governed bridge between workflow structure and internal event progression.

**Independent Test**: A developer can derive one valid workflow execution waiting on one event-driven edge and show that one matching governed internal event advances the workflow exactly once from this spec alone.

**Acceptance Scenarios**:

1. **Given** a running workflow execution is waiting on one event-driven edge, **When** the exact declared event id and version are emitted internally, **Then** the workflow advances along that edge exactly once.
2. **Given** a waiting workflow edge references an event that is not emitted, **When** unrelated internal events occur, **Then** the workflow does not advance.
3. **Given** a matching event is consumed for one waiting edge, **When** the same event record is considered again for that same edge, **Then** it does not trigger a second progression.

### User Story 2 - Wake Multiple Eligible Workflows Deterministically (Priority: P1)

As a platform developer or reviewer, I want one governed event to wake multiple eligible waiting workflow edges in deterministic order so that event-driven progression remains explainable even across more than one workflow instance.

**Why this priority**: A single-consumer event model would be too restrictive for realistic workflow orchestration, but nondeterministic wake order would weaken governance.

**Independent Test**: A developer can derive one event that matches more than one eligible waiting edge and verify deterministic ordering and per-edge exact-once consumption from this spec alone.

**Acceptance Scenarios**:

1. **Given** two or more waiting workflow edges are eligible for the same emitted event, **When** the event is processed, **Then** all eligible edges wake in deterministic governed order.
2. **Given** one eligible waiting edge consumes the event, **When** another eligible waiting edge is processed, **Then** that edge may also consume the same event record once for its own progression.
3. **Given** cross-workflow wake order is reviewed after execution, **When** trace evidence is inspected, **Then** the ordering decision is explicit and reconstructable.

### User Story 3 - Preserve Explainable Event-Driven Decisions Without Reopening the Top-Level State Machine (Priority: P2)

As a reviewer or future UI/MCP consumer, I want event-driven progression to appear in dedicated workflow and trace evidence without adding new top-level runtime waiting states so that event-driven behavior remains inspectable without destabilizing the narrower runtime state-machine slice.

**Why this priority**: We need explainability for event-driven workflow behavior, but we intentionally do not want to broaden the governed top-level runtime state model in this slice.

**Independent Test**: A developer can derive one event-driven progression trace showing event match, predicate evaluation, edge selection, and deterministic wake ordering while the top-level runtime state model remains unchanged.

**Acceptance Scenarios**:

1. **Given** a matching event wakes a waiting workflow edge, **When** the trace is inspected, **Then** the wake-up appears as dedicated governed decision evidence.
2. **Given** a workflow is waiting on an event-driven edge, **When** the runtime state model is inspected, **Then** no new top-level waiting state is required by this slice.
3. **Given** a matching event payload fails the declared simple predicate, **When** trace evidence is inspected, **Then** the rejection is explicit and machine-readable.

## Scope

In scope:

- governed internal events only
- progression of already-running workflow executions only
- event-driven workflow edges as explicit workflow edge types
- exact event id and version references
- exact-once event consumption per waiting edge
- simple field-equality payload predicates
- deterministic multi-workflow wake ordering
- dedicated event-driven decision evidence in traces
- deterministic fixture validation expectations

Out of scope:

- external broker or infrastructure integration
- external event sources
- event-created new executions
- direct capability waiting semantics
- compound event conditions
- richer filter-expression languages
- top-level runtime waiting states

## Requirements

### Functional Requirements

- **FR-001**: The event-driven composition slice MUST govern runtime progression only for already-running workflow executions.
- **FR-002**: This slice MUST use governed internal Traverse events only and MUST NOT define external event sources or broker integration.
- **FR-003**: Event-driven progression in this slice MUST apply to workflow executions only and MUST NOT add waiting semantics to direct capability execution.
- **FR-004**: Event-driven workflow progression MUST be modeled as explicit workflow edge types rather than as runtime-only hidden subscription rules.
- **FR-005**: Every event-driven workflow edge MUST reference exactly one governed event id and one governed event version.
- **FR-006**: Event-family or loose event matching MUST NOT be used in this slice.
- **FR-007**: Every event-driven workflow edge MAY include simple field-equality predicates over the validated event payload.
- **FR-008**: This slice MUST NOT define compound event conditions or richer filter-expression languages.
- **FR-009**: A matching event record MUST be consumable exactly once per waiting workflow edge.
- **FR-010**: The same event record MUST NOT trigger the same waiting edge more than once.
- **FR-011**: One emitted event record MAY wake multiple eligible waiting workflow edges across workflow executions.
- **FR-012**: When one event wakes multiple eligible waiting workflow edges, the wake order MUST be deterministic under governed ordering rules.
- **FR-013**: Event-driven progression MUST emit dedicated governed decision evidence rather than relying on top-level state history alone.
- **FR-014**: Dedicated event-driven decision evidence MUST include at least:
  - matched event identity
  - matched event version
  - waiting edge identity
  - predicate result
  - wake ordering information when more than one eligible edge exists
- **FR-015**: This slice MUST keep the top-level runtime state machine unchanged and MUST NOT add explicit waiting states to that slice.
- **FR-016**: Event-driven progression failures or non-matches MUST remain explainable through workflow and trace evidence without changing the top-level runtime state model.
- **FR-017**: The event-driven composition model MUST remain machine-readable enough for deterministic fixture validation of wake-up, no-match, duplicate-consumption rejection, and multi-workflow ordering behavior.
- **FR-018**: Approved implementation under this spec MUST be validated against this governing spec before merge.

### Key Entities

- **Event-Driven Workflow Edge**: A workflow edge that is activated by one governed internal event rather than direct sequential traversal.
- **Waiting Workflow Edge Context**: The machine-readable runtime representation of one running workflow edge awaiting one exact event.
- **Event Match Record**: One machine-readable record explaining whether an emitted event matched one waiting edge.
- **Event Wake Decision**: One governed decision-evidence record showing that an emitted event advanced one waiting workflow edge.
- **Event Consumption Record**: The machine-readable evidence that one event record was consumed once for one waiting edge.

## Non-Functional Requirements

- **NFR-001 Determinism**: Event matching, predicate evaluation, per-edge consumption, and multi-workflow wake ordering MUST be deterministic for the same event and waiting-edge set.
- **NFR-002 Explainability**: Event-driven progression MUST be explainable from governed workflow and trace evidence without undocumented runtime heuristics.
- **NFR-003 Compatibility**: This slice MUST extend workflow and trace behavior without reopening the governed top-level runtime state model.
- **NFR-004 Maintainability**: Internal event progression, external integrations, new request ingress, and richer event filtering MUST remain separate slices.
- **NFR-005 Portability**: The governed event-driven model MUST remain independent from any specific message broker or delivery transport.
- **NFR-006 Testability**: Future implementation under this slice MUST be structured enough for deterministic fixture coverage of match, no-match, duplicate rejection, and ordered wake behavior.

## Non-Negotiable Quality Gates

- **QG-001**: No event-driven runtime implementation may merge under this slice if it creates new executions from emitted events.
- **QG-002**: No event-driven runtime implementation may merge under this slice if it uses loose event-family matching or undeclared event references.
- **QG-003**: No event-driven runtime implementation may merge under this slice if one waiting edge can be advanced more than once by the same event record.
- **QG-004**: No event-driven runtime implementation may merge under this slice if multi-workflow wake ordering is nondeterministic.
- **QG-005**: Approved implementation under this slice MUST include deterministic fixture coverage for event wake-up, no-match, duplicate-consumption rejection, and multi-workflow ordering cases.

## Success Criteria

- **SC-001**: A running workflow can advance from one governed internal event through one explicit event-driven edge.
- **SC-002**: One event can wake multiple eligible waiting workflow edges in deterministic governed order.
- **SC-003**: Event-driven progression is reconstructable through dedicated decision evidence in the trace.
- **SC-004**: The event-driven composition backlog slice can move from `needs-spec` toward implementation planning under one focused governing artifact.

## Governing Relationship

This specification is governed by:

- `001-foundation-v0-1`
- `003-event-contracts`
- `007-workflow-registry-traversal`
- constitution version `1.2.0`

This specification is intentionally aligned with the dedicated runtime-state-machine and event-registry slices that are being developed in parallel, but it does not require those future spec ids to exist before this slice can be reviewed.

This specification, once approved, is intended to govern future implementation in:

- future event-driven runtime orchestration code
- future workflow event-progression validation and trace tooling

# Feature Specification: Governed Workflow Reliability

**Status**: Approved
**Canonical governing ID**: `129-governed-workflow-reliability`
**Extends**: `109-runtime-workflow-proposals`, `111-durable-dynamic-orchestration`
**Decision evidence**: Traverse #1235, 2026-09-06

## Purpose

Provide bounded, explicit retry and sequential compensation for a reviewed
workflow proposal. This is not a distributed transaction facility.

## Requirements

- **FR-001**: A workflow step MAY declare a retry policy only when its contract
  declares it idempotent. The policy MUST state a finite attempt bound and a
  deterministic backoff schedule.
- **FR-002**: A compensation action MUST be an explicit, pinned workflow step
  with declared inputs and its own contract/policy validation. It runs only
  after a successfully completed forward step requires recovery.
- **FR-003**: The runtime MUST execute forward steps sequentially. On terminal
  failure it MUST compensate eligible completed steps in reverse completion
  order; it MUST NOT claim atomic commit or rollback.
- **FR-004**: The reviewed canonical proposal binds retry and compensation
  declarations. A proposer, planner, or caller MUST NOT alter them afterwards.
- **FR-005**: Validation MUST reject unbounded retries, non-idempotent retry,
  missing or cyclic compensation, unsupported atomic semantics, exhausted
  approval/budget, and invalid compensation mappings before execution.
- **FR-006**: An interruption MUST resume from durable checkpoint evidence.
  An effectful step MUST NOT be retried or replayed absent its declared
  idempotency and remaining retry authority.
- **FR-007**: The trace MUST emit redacted, ordered evidence for each forward
  attempt, retry, compensation, skipped compensation, and terminal outcome.

## Acceptance Scenarios

1. A retryable idempotent step fails once then succeeds within its bound; the
   trace records both attempts and no compensation runs.
2. A later terminal failure compensates earlier eligible completed steps in
   reverse order and reports any compensation failure distinctly.
3. A proposal requesting atomic semantics, unbounded retry, or compensation
   for an undeclared target is rejected before a forward step executes.
4. Restart after an interruption uses checkpoint evidence and never silently
   repeats a non-idempotent completed effect.

## Quality Gates

- **QG-001**: Unit and contract tests cover every FR-005 rejection category.
- **QG-002**: Integration tests cover success-after-retry, reverse-order
  compensation, compensation failure, and interruption recovery.
- **QG-003**: Runtime coverage, lint, and spec-alignment gates pass.

## Out of Scope

- Distributed atomic transactions, two-phase commit, and exactly-once effects.
- Implicit retries, implicit compensations, or planner-selected recovery.
- Compensation of an external effect without an explicitly governed capability.

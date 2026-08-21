# Durable Dynamic Orchestration (P3)

**Status**: Approved
**Canonical governing ID**: `111-durable-dynamic-orchestration`  
**Version**: 0.1.0  
**Depends on**: approved P1 and P2 specs and a production-approved host-owned
durable trace/state substrate.

## Purpose

Define durable checkpoint, wait, recovery, cancellation, retry, and explicit
compensation semantics for governed dynamic workflow executions.

## Requirements

- **FR-001**: A resumable execution MUST persist an authenticated,
  secret-free checkpoint bound to its proposal, manifest, registry, policy,
  and authorization snapshots before reporting a wait or completed effect.
- **FR-002**: Recovery MUST either resume from a valid checkpoint exactly as
  governed or fail closed; it MUST never re-plan or silently re-resolve an
  unpinned dependency.
- **FR-003**: Waits for events, schedules, or human approvals MUST have
  bounded lifetime, cancellation, ownership, and stable wake-up evidence.
- **FR-004**: Retry requires declared retryability, bounded attempts/backoff,
  idempotency key behavior, budget accounting, and evidence.
- **FR-005**: Compensation MUST be explicit, capability-contract declared,
  authorization-checked, ordered, bounded, and separately traceable. It is
  not a claim of distributed atomicity.

## Acceptance scenarios

1. A waiting execution restarts from a valid checkpoint without changing its
   proposal or resolved artifact identities.
2. A corrupt, expired, or mismatched checkpoint fails closed.
3. A retryable connector failure honors declared attempt and budget limits.
4. A compensation failure is recorded as a distinct terminal state.

## Out of scope

Unbounded durable queues, hidden provider retry behavior, and automatic
compensation for capabilities without a declared compensation contract.

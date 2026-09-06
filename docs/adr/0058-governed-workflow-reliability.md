# ADR-0058: Explicit Sequential Workflow Recovery

- Status: Accepted
- Date: 2026-09-06
- Governing spec: `129-governed-workflow-reliability`
- Extends: `109-runtime-workflow-proposals`, `111-durable-dynamic-orchestration`
- Related issue: #1235

## Context

Governed proposals can execute multi-step workflows but intentionally lack
retry, compensation, and transaction semantics. Retrying an effect without
idempotency or claiming rollback across external systems would violate the
runtime's explainability and trust boundaries.

## Decision

Use bounded retries only for contract-declared idempotent steps. Model recovery
as explicit compensating capability steps, executed sequentially in reverse
completion order after a terminal forward failure. Bind all recovery behavior
into the reviewed proposal and record it in redacted durable trace evidence.

## Consequences

The runtime gains a useful, auditable recovery model but never promises
distributed atomicity. A failed compensation remains a visible terminal
condition for operator handling; the runtime does not invent a further action.

## Alternatives Considered

- Retries only: rejected because it cannot recover completed earlier effects.
- Distributed atomic transactions: rejected because external effects and hosts
  cannot honestly provide one common atomicity guarantee in this slice.
- Implicit best-effort rollback: rejected because it would hide authority and
  safety decisions from review and trace evidence.

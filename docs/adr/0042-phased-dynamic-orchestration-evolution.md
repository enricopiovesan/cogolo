# ADR-0042: Evolve Dynamic Composition Through Bounded Parallel, Durable, and Promotion Phases

- Status: Accepted
- Governing specs: `108-governed-runtime-workflow-composition`, `110-bounded-parallel-workflow-scheduling`, `111-durable-dynamic-orchestration`, `112-governed-workflow-promotion`
- Related issue: #1089

## Context

The desired UMA workflow experience includes more than one serial request:
parallel branches, waits, retries, compensation, and reusable learnings may
be required. Delivering all of them together would create an unbounded new
scheduler, storage, and authorization surface.

## Decision

Treat P1 sequential proposal execution as the first compatible slice of a
documented north star. Add P2 only with deterministic bounded scheduling; add
P3 only with host-owned durable checkpoints, explicit retry/compensation and
recovery semantics; add P4 only via export and normal human-reviewed
workflow/app publication. No phase becomes available solely because a prior
phase exists.

## Consequences

The complete destination is visible and ticketed now, while each implementation
remains independently spec-approved, testable, and reversible. Current docs
must never describe P2-P4 as shipped before their gates are met.

## Alternatives considered

- Define only P1: rejected because the product architecture would drift.
- Build a full general workflow engine at once: rejected as too broad for
  deterministic security, performance, persistence, and verification controls.

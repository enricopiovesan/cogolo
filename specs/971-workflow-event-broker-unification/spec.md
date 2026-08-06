# Feature Specification: Unify Workflow Event-Driven Edges With EventBroker

**Status**: Approved
**Canonical governing ID**: `099-workflow-event-broker-unification`
**Version**: 1.1.0
**Extends**: `018-event-driven-composition`, `207-event-broker`
**Input**: Issue #971; ADR-0036; `/brainstorm` session recorded as Decision 50 in `docs/decision-log.md`.

**Amendment (2026-08-06, v1.0.0 -> v1.1.0)**: A pre-implementation happy/unhappy-path
audit found FR-007 only covered "event type not registered in the catalog" — it left
`EventBroker` being unreachable/unavailable at the moment a workflow tries to register
a waiting-edge subscription undefined, a distinct failure mode from an unregistered
type. FR-008 below closes it. No existing FR text changed. Approved by the repo owner
the same day this gap was raised.

## Purpose

Let a waiting workflow event-driven edge advance from any event published to
`EventBroker` — including events from other workflow executions, other
capabilities, or external publishers — rather than only events declared in
the same execution's own node output, as `018-event-driven-composition`
currently requires. This spec supersedes `018`'s explicit "without
introducing external brokers" scope cut while preserving its deterministic
ordering and exact-once-consumption guarantees.

## Capability Boundary

This spec governs how a waiting workflow edge subscribes to and consumes
`EventBroker` events. It does not change `EventBroker`'s own delivery or
catalog semantics (`207`), does not change how a capability emits an event
(governed separately by `098-capability-event-host-abi`, issue #969), and
does not include the implementation itself (tracked separately as issue
#972).

## Requirements

- **FR-001**: A workflow edge with `WorkflowEdgeTrigger::Event` MUST
  register a subscription with `EventBroker` for its declared
  `event_ref` (`event_id` + `version`), rather than only being evaluated
  against the current execution's own node output.
- **FR-002**: An event published by `EventBroker` — regardless of which
  workflow execution, capability, or external publisher produced it — MUST
  be eligible to advance any waiting edge whose `event_ref` and predicate
  match, preserving `018`'s existing simple field-equality payload
  predicate model.
- **FR-003**: `018`'s deterministic multi-workflow wake ordering guarantee
  MUST be preserved: when one event is eligible for more than one waiting
  edge (across one or more workflow executions), all eligible edges MUST
  wake in deterministic, explainable order.
- **FR-004**: `018`'s exact-once-per-edge consumption guarantee MUST be
  preserved: one event record MUST NOT trigger the same waiting edge more
  than once, even when sourced from a durable/replayed broker stream.
- **FR-005**: `EventDrivenEvaluationOutcome` and its trace evidence MUST
  record which `EventBroker` subscription and cursor produced a given
  wake-up decision, so the wake-up remains explainable and reconstructable
  from trace evidence alone (preserving `018`'s explainability requirement,
  now against a real broker rather than a single synchronous pass).
- **FR-006**: This unification MUST NOT introduce a new top-level runtime
  waiting state, matching `018`'s original constraint that event-driven
  progression stays within the existing state model.
- **FR-007**: A waiting edge whose subscription cannot be established (for
  example, an unregistered event type in `EventBroker`'s catalog) MUST
  surface a stable, machine-readable error rather than silently never
  waking.
- **FR-008** (v1.1.0): If `EventBroker` is unreachable or internally failing
  at the moment a workflow attempts to register a waiting-edge subscription
  (distinct from FR-007's "event type not registered" case), the workflow
  execution MUST surface a stable, retryable error rather than silently
  never waking or crashing the execution.

## Acceptance Scenarios

1. Given workflow A is waiting on an event-driven edge, when workflow B (a
   separate execution) or a standalone capability publishes the exact
   declared event to `EventBroker`, then workflow A's edge advances.
2. Given two waiting edges across two different workflow executions are
   both eligible for one published event, when the event is processed, then
   both wake in deterministic order and each edge consumes the event
   exactly once.
3. Given a matching event is replayed from `EventBroker`'s durable journal
   after a restart, when a waiting edge that already consumed that event
   record is re-evaluated, then it does not advance a second time.
4. Given a wake-up occurs, when trace evidence is inspected, then it
   identifies the `EventBroker` subscription and cursor responsible for the
   advancement, in addition to `018`'s existing event-match/predicate/
   edge-selection evidence.
5. Given `EventBroker` is unreachable when a workflow attempts to register a
   waiting-edge subscription, when the attempt is made, then a stable,
   retryable error is surfaced and the workflow does not crash or hang
   silently.

## Out of Scope

- Changes to how a capability emits an event (`098-capability-event-host-abi`,
  issue #969).
- `EventBroker`'s own delivery, durability, or catalog semantics (`207`).
- Introducing a new top-level runtime waiting state.

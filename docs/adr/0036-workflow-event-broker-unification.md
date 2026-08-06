# ADR-0036: Unify Workflow Event-Driven Edges With `EventBroker`

- Status: Accepted
- Governing spec: `099-workflow-event-broker-unification`

## Context

`018-event-driven-composition` deliberately scoped workflow event-driven
edges to advance "without introducing external brokers, event-created
executions, direct-capability waiting semantics" — a considered design
choice, not a placeholder. In the current implementation
(`workflows.rs::evaluate_event_driven_edges`), a waiting workflow edge can
only advance from an event extracted from the *same* workflow execution's
own node output (`emitted_events(output: &Value)` reads an `emitted_events`
array out of a capability's JSON output). There is no connection to
`EventBroker`, no cross-workflow delivery, no cross-process delivery, and
no durability — the whole mechanism is a synchronous, in-memory pattern
match scoped to one execution.

This is a third, separate event mechanism in Traverse alongside `EventBroker`
and the capability output-JSON convention being replaced by
`098-capability-event-host-abi` (issue #969). Traverse has no production
workflows depending on `018`'s current scope cut today
(`docs/decision-log.md` Decision 48).

## Decision

Unify workflow event-driven edge advancement with the real `EventBroker`.
A waiting edge subscribes to `EventBroker` the same way any other consumer
does, so it can advance from an event published by another workflow,
another capability, or an external publisher — not only from the same
execution's own node output. This reverses `018`'s explicit scope cut,
justified by Decision 48 (no back-compat tax pre-production) and by this
being the one place in the runtime where UMA's cross-system event-driven
composition model was structurally impossible under the prior design.
`018`'s existing deterministic multi-workflow wake ordering and
exact-once-per-edge consumption guarantees are preserved, now enforced
against a real broker instead of a single synchronous evaluation pass.

## Consequences

Workflow execution gains a dependency on `EventBroker`'s delivery and
durability characteristics — a waiting edge's advancement now depends on
broker availability, not just in-process state. `EventDrivenEvaluationOutcome`
and its trace evidence must be extended to record which broker subscription
and cursor produced the wake-up, so the existing explainability guarantees
from `018` (dedicated wake-up decision evidence) survive the change.
Cross-workflow and cross-process wake ordering must be proven deterministic
under the broker's own delivery model, not assumed from the old single-pass
implementation.

## Alternatives Considered

- Keep `018`'s synchronous, single-execution-scoped model as deliberately
  separate — this was the recommended option going into the decision
  (workflow-edge advancement is arguably simpler without cross-process
  delivery/ordering/durability concerns), but the user chose unification,
  citing that it's the one remaining place event-driven composition can't
  cross a workflow or process boundary, which is core to UMA's model and to
  what capability authors will expect once `098`'s imperative ABI exists.
- Introduce a fourth, purpose-built broker just for workflow-edge wake-ups —
  rejected: this would recreate the exact fragmentation this whole
  brainstorm session exists to close.

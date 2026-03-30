# Data Model: Event-Driven Composition

## Purpose

This document defines the implementation-tight data model for the `018-event-driven-composition` slice.

It focuses on event-driven workflow edges, waiting-edge contexts, exact event matching, exact-once per-edge consumption, and dedicated trace-visible decision evidence.

## 1. Event-Driven Workflow Edge

Represents one workflow edge that advances on one governed internal event.

### Required Fields

- `edge_id`
- `trigger`
- `event_ref`

### Optional Fields

- `predicate`

### Shape

```json
{
  "edge_id": "assess_to_validate_on_summary_assessed",
  "trigger": "event",
  "event_ref": {
    "event_id": "expedition.conditions.summary-assessed",
    "event_version": "1.0.0"
  },
  "predicate": {
    "field": "payload.severity",
    "equals": "normal"
  }
}
```

### Rules

- `trigger` must equal `event`
- exactly one event reference is allowed per event-driven edge in this slice
- the event reference must use exact event id and version values

## 2. Event Reference

Represents the exact governed event identity required by one waiting edge.

### Required Fields

- `event_id`
- `event_version`

### Rules

- loose family matching is not allowed in this slice

## 3. Simple Predicate

Represents the optional payload filter on an event-driven edge.

### Required Fields

- `field`
- `equals`

### Shape

```json
{
  "field": "payload.team_ready",
  "equals": true
}
```

### Rules

- predicates are optional
- predicates are limited to simple field-equality matching in this slice
- richer expression languages are out of scope

## 4. Waiting Workflow Edge Context

Represents one active workflow edge awaiting one exact event.

### Required Fields

- `workflow_execution_id`
- `edge_id`
- `from_node_id`
- `to_node_id`
- `event_ref`

### Optional Fields

- `predicate`

### Shape

```json
{
  "workflow_execution_id": "wf_exec_123",
  "edge_id": "assess_to_validate_on_summary_assessed",
  "from_node_id": "assess_conditions",
  "to_node_id": "validate_readiness",
  "event_ref": {
    "event_id": "expedition.conditions.summary-assessed",
    "event_version": "1.0.0"
  },
  "predicate": {
    "field": "payload.severity",
    "equals": "normal"
  }
}
```

### Rules

- waiting contexts apply to running workflows only in this slice
- direct capability waiting is out of scope

## 5. Event Match Record

Represents the result of evaluating one emitted event against one waiting workflow edge.

### Required Fields

- `event_id`
- `event_version`
- `edge_id`
- `match_result`
- `recorded_at`

### Optional Fields

- `predicate_result`
- `rejection_reason`

### Match Result Enum

- `matched`
- `not_matched`
- `already_consumed`

### Shape

```json
{
  "event_id": "expedition.conditions.summary-assessed",
  "event_version": "1.0.0",
  "edge_id": "assess_to_validate_on_summary_assessed",
  "match_result": "matched",
  "predicate_result": "passed",
  "recorded_at": "2026-03-30T00:00:00Z"
}
```

### Rules

- one event record may be evaluated against multiple waiting edges
- the same event record must not match the same edge more than once after consumption

## 6. Event Wake Decision

Represents the dedicated governed decision evidence for one event-driven progression.

### Required Fields

- `decision_type`
- `event_id`
- `event_version`
- `edge_id`
- `workflow_execution_id`
- `wake_order`
- `result`
- `recorded_at`

### Shape

```json
{
  "decision_type": "event_wake",
  "event_id": "expedition.conditions.summary-assessed",
  "event_version": "1.0.0",
  "edge_id": "assess_to_validate_on_summary_assessed",
  "workflow_execution_id": "wf_exec_123",
  "wake_order": 1,
  "result": "taken",
  "recorded_at": "2026-03-30T00:00:01Z"
}
```

### Rules

- event-driven wake-up must appear as dedicated trace-visible decision evidence
- `wake_order` must be deterministic when one event wakes multiple edges

## 7. Event Consumption Record

Represents the per-edge exact-once consumption evidence for one matched event.

### Required Fields

- `event_id`
- `event_version`
- `edge_id`
- `workflow_execution_id`
- `consumed_at`

### Shape

```json
{
  "event_id": "expedition.conditions.summary-assessed",
  "event_version": "1.0.0",
  "edge_id": "assess_to_validate_on_summary_assessed",
  "workflow_execution_id": "wf_exec_123",
  "consumed_at": "2026-03-30T00:00:01Z"
}
```

### Rules

- consumption is exact-once per event record per waiting edge
- the same event record may still be consumed once by another eligible waiting edge

## 8. Ordering Rules

- one event may wake multiple eligible waiting edges
- wake order across eligible waiting edges must be deterministic
- event-driven wake ordering must be reconstructable from decision evidence

## 9. Runtime State Relationship

### Rules

- this slice does not add new top-level runtime states
- waiting semantics live in workflow and trace evidence, not in the top-level runtime state machine

## 10. Validation Rules

- event-driven edges must use exact event references
- one triggering event per edge only
- simple field-equality predicates only
- exact-once consumption per edge
- deterministic fixture coverage must cover:
  - wake-up success
  - no-match
  - duplicate-consumption rejection
  - multi-workflow deterministic ordering

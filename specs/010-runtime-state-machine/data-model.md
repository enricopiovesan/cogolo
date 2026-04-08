# Data Model: Runtime State Machine

## Purpose

This document defines the implementation-tight artifacts for the `010-runtime-state-machine` slice.

It governs the runtime state values, transition rules, state events, and terminal result mapping that later runtime implementation slices must follow.

## 1. Runtime State Enum

Represents the canonical externally visible runtime state values.

### Enum Values

- `idle`
- `loading_registry`
- `ready`
- `discovering`
- `evaluating_constraints`
- `selecting`
- `executing`
- `emitting_events`
- `completed`
- `error`

### Rules

- `idle` exists before registry loading begins and after runtime construction before initialization.
- `loading_registry` is runtime-wide rather than request-scoped.
- `ready` means the runtime can accept a request.
- `discovering`, `evaluating_constraints`, `selecting`, `executing`, and `emitting_events` are request-scoped active states.
- `completed` and `error` are terminal for one execution attempt.
- New request attempts begin from `ready`; they do not continue from a previous terminal state.

## 2. Runtime Transition Rule

Represents one allowed state transition.

### Required Fields

- `from`
- `to`
- `reason_code`

### Shape

```json
{
  "from": "ready",
  "to": "discovering",
  "reason_code": "request_started"
}
```

### Allowed Transition Set

```json
[
  { "from": "idle", "to": "loading_registry", "reason_code": "runtime_initialization_started" },
  { "from": "loading_registry", "to": "ready", "reason_code": "registry_loaded" },
  { "from": "loading_registry", "to": "error", "reason_code": "registry_load_failed" },
  { "from": "ready", "to": "discovering", "reason_code": "request_started" },
  { "from": "discovering", "to": "evaluating_constraints", "reason_code": "candidates_collected" },
  { "from": "discovering", "to": "error", "reason_code": "no_match" },
  { "from": "evaluating_constraints", "to": "selecting", "reason_code": "constraints_evaluated" },
  { "from": "evaluating_constraints", "to": "error", "reason_code": "constraint_validation_failed" },
  { "from": "selecting", "to": "executing", "reason_code": "candidate_selected" },
  { "from": "selecting", "to": "error", "reason_code": "selection_failed" },
  { "from": "executing", "to": "emitting_events", "reason_code": "execution_succeeded_with_events" },
  { "from": "executing", "to": "completed", "reason_code": "execution_succeeded" },
  { "from": "executing", "to": "error", "reason_code": "execution_failed" },
  { "from": "emitting_events", "to": "completed", "reason_code": "events_emitted" },
  { "from": "emitting_events", "to": "error", "reason_code": "event_emission_failed" },
  { "from": "completed", "to": "ready", "reason_code": "execution_closed" },
  { "from": "error", "to": "ready", "reason_code": "execution_closed" }
]
```

### Rules

- The transition set above is exhaustive for this slice.
- Any implementation attempt to expose a different externally visible transition is a spec violation.
- `completed -> ready` and `error -> ready` are runtime lifecycle resets between requests, not continuation of the same execution attempt.

## 3. Runtime State Event

Represents one emitted runtime state transition visible to consumers.

### Required Fields

- `kind`
- `schema_version`
- `event_id`
- `request_id`
- `execution_id`
- `state`
- `entered_at`
- `details`

### Shape

```json
{
  "kind": "runtime_state_event",
  "schema_version": "1.0.0",
  "event_id": "rse_20260330_0001",
  "request_id": "req_20260330_0001",
  "execution_id": "exec_20260330_0001",
  "state": "executing",
  "entered_at": "2026-03-30T00:00:00Z",
  "details": {
    "transition_reason": "candidate_selected",
    "capability_id": "expedition.planning.assemble-expedition-plan",
    "capability_version": "1.0.0"
  }
}
```

### Rules

- `details` must remain structured JSON.
- `state` must be one of the governed runtime state values.
- `transition_reason` must correspond to one of the governed `reason_code` values for the transition into `state`.

## 4. Runtime Transition Record

Represents a trace-ready record of one state transition.

### Required Fields

- `from_state`
- `to_state`
- `reason_code`
- `occurred_at`

### Optional Fields

- `request_id`
- `execution_id`
- `details`

### Shape

```json
{
  "from_state": "selecting",
  "to_state": "executing",
  "reason_code": "candidate_selected",
  "occurred_at": "2026-03-30T00:00:00Z",
  "request_id": "req_20260330_0001",
  "execution_id": "exec_20260330_0001",
  "details": {
    "selected_capability_id": "expedition.planning.assemble-expedition-plan",
    "selected_capability_version": "1.0.0"
  }
}
```

### Rules

- Transition records may be embedded in traces or validation evidence.
- `details` must not replace the canonical `from_state`, `to_state`, and `reason_code` fields.

## 5. Runtime Terminal Result Mapping

Represents the governed relationship between terminal state and normalized result status.

### Shape

```json
{
  "completed": "completed",
  "error": "error"
}
```

### Rules

- `completed` maps to normalized runtime result status `completed`.
- `error` maps to normalized runtime result status `error`.
- No other runtime state may be treated as terminal in this slice.

## 6. Runtime State Validation Evidence

Represents machine-readable validation output for state-machine conformance.

### Required Fields

- `kind`
- `schema_version`
- `governing_spec`
- `validated_at`
- `status`
- `checked_states`
- `checked_transitions`
- `violations`

### Shape

```json
{
  "kind": "runtime_state_machine_validation",
  "schema_version": "1.0.0",
  "governing_spec": "010-runtime-state-machine",
  "validated_at": "2026-03-30T00:00:00Z",
  "status": "passed",
  "checked_states": [
    "idle",
    "loading_registry",
    "ready",
    "discovering",
    "evaluating_constraints",
    "selecting",
    "executing",
    "emitting_events",
    "completed",
    "error"
  ],
  "checked_transitions": [
    "ready->discovering",
    "discovering->evaluating_constraints",
    "evaluating_constraints->selecting",
    "selecting->executing",
    "executing->completed"
  ],
  "violations": []
}
```

### Rules

- `status` values:
  - `passed`
  - `failed`
- `violations` entries must carry enough structured detail to explain which unexpected state or transition was observed.

## 7. Workflow-backed Execution Notes

This slice does not define a separate workflow-only runtime state machine.

### Rules

- Workflow-backed execution uses the same runtime state values.
- Workflow traversal detail remains trace or workflow-evidence detail, not a replacement runtime state set.
- A workflow-backed execution that emits post-node events may still use `emitting_events` before `completed`.

## 8. Implementation Notes

- `idle` and `ready` may be represented by runtime-wide controller state outside one execution attempt, but externally visible events must still conform to this slice.
- Existing request and workflow slices continue to govern request shape and workflow traversal shape; this slice becomes the authoritative reference for runtime lifecycle semantics.

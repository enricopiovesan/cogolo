# Data Model: Runtime Request and Local Execution Model

## Purpose

This document defines the implementation-tight runtime artifacts for the `006-runtime-request-execution` slice.

It focuses on the single-capability local execution path for `cogolo-runtime`.

## 1. Runtime Request

Represents one execution request submitted to the runtime.

### Required Fields

- `kind`
- `schema_version`
- `request_id`
- `intent`
- `input`
- `lookup`
- `context`
- `governing_spec`

### Shape

```json
{
  "kind": "runtime_request",
  "schema_version": "1.0.0",
  "request_id": "req_...",
  "intent": {
    "capability_id": "content.comments.create-comment-draft",
    "capability_version": "1.0.0",
    "intent_key": "content.comments.create-comment-draft"
  },
  "input": {},
  "lookup": {
    "scope": "prefer_private",
    "allow_ambiguity": false
  },
  "context": {
    "requested_target": "local",
    "correlation_id": "corr_...",
    "caller": "cli"
  },
  "governing_spec": "006-runtime-request-execution"
}
```

### Notes

- `intent.capability_id` and `intent.intent_key` may coexist, but at least one must be present.
- `capability_version` is optional unless exact version targeting is required.
- `allow_ambiguity` is included only to make the failure rule explicit; `v0.1` still requires `false`.

## 2. Runtime Intent

Represents the executable intent boundary inside a runtime request.

### Required Fields

At least one of:

- `capability_id`
- `intent_key`

### Optional Fields

- `capability_version`

### Rules

- If `capability_id` and `capability_version` are both present, the runtime must treat the request as exact targeting.
- If only `intent_key` is present, the runtime must perform candidate discovery.
- If both exact targeting and intent lookup are present, exact targeting wins.

## 3. Runtime Lookup Options

Represents deterministic lookup preferences.

### Required Fields

- `scope`
- `allow_ambiguity`

### Enum Values

`scope`:

- `public_only`
- `prefer_private`

### Rules

- `allow_ambiguity` must be `false` in this slice.
- `prefer_private` means the runtime looks in private scope first and public scope second.

## 4. Runtime Execution Context

Represents request-scoped metadata used during filtering and tracing.

### Required Fields

- `requested_target`

### Optional Fields

- `correlation_id`
- `caller`
- `metadata`

### Enum Values

`requested_target`:

- `local`

### Rules

- Any non-`local` target must be rejected for this slice.

## 5. Runtime Candidate

Represents one candidate capability under consideration during discovery.

### Required Fields

- `scope`
- `capability_id`
- `capability_version`
- `artifact_ref`
- `implementation_kind`
- `lifecycle`
- `reason`

### Notes

- `reason` should explain why the candidate exists in the set, such as `exact_match` or `intent_match`.
- Candidate records in traces may include later rejection details.

## 6. Runtime State

Represents the runtime state machine values for this slice.

### Enum Values

- `loading_registry`
- `ready`
- `discovering`
- `evaluating_constraints`
- `selecting`
- `executing`
- `completed`
- `error`

### Rules

- `ready` exists outside a single request, but request execution traces should begin once the runtime enters `discovering`.
- `completed` and `error` are terminal states for one execution attempt.

## 7. Runtime State Event

Represents one emitted state-machine transition.

### Required Fields

- `kind`
- `schema_version`
- `execution_id`
- `request_id`
- `state`
- `timestamp`
- `details`

### Shape

```json
{
  "kind": "runtime_state_event",
  "schema_version": "1.0.0",
  "execution_id": "exec_...",
  "request_id": "req_...",
  "state": "executing",
  "timestamp": "2026-03-27T00:00:00Z",
  "details": {
    "capability_id": "content.comments.create-comment-draft",
    "capability_version": "1.0.0"
  }
}
```

### Rules

- State events must be emitted in deterministic order.
- `details` must remain structured JSON.

## 8. Runtime Trace

Represents the explainability artifact for one execution attempt.

### Required Fields

- `kind`
- `schema_version`
- `trace_id`
- `execution_id`
- `request_id`
- `governing_spec`
- `request`
- `candidate_collection`
- `selection`
- `execution`
- `result`

### Shape

```json
{
  "kind": "runtime_trace",
  "schema_version": "1.0.0",
  "trace_id": "trace_...",
  "execution_id": "exec_...",
  "request_id": "req_...",
  "governing_spec": "006-runtime-request-execution",
  "request": {},
  "candidate_collection": {
    "lookup_scope": "prefer_private",
    "candidates": [],
    "rejected_candidates": []
  },
  "selection": {
    "status": "selected",
    "selected_capability_id": "content.comments.create-comment-draft",
    "selected_capability_version": "1.0.0"
  },
  "execution": {
    "placement_target": "local",
    "status": "succeeded",
    "artifact_ref": "artifact:create-comment-draft:1.0.0"
  },
  "result": {
    "status": "completed"
  }
}
```

## 9. Candidate Collection Record

Represents discovery output inside the trace.

### Required Fields

- `lookup_scope`
- `candidates`
- `rejected_candidates`

### Rejected Candidate Shape

- `capability_id`
- `capability_version`
- `scope`
- `reason`

### Rejection Reasons

- `wrong_scope`
- `not_runnable_locally`
- `lifecycle_not_runnable`
- `input_contract_invalid`
- `artifact_missing`
- `superseded_by_private_overlay`
- `not_selected_after_ordering`

## 10. Selection Record

Represents the outcome of deterministic runtime selection.

### Required Fields

- `status`

### Optional Fields

- `selected_capability_id`
- `selected_capability_version`
- `failure_reason`
- `remaining_candidates`

### Enum Values

`status`:

- `selected`
- `no_match`
- `ambiguous`
- `invalid_request`

### Rules

- `remaining_candidates` is required when `status = ambiguous`.
- `failure_reason` is required for all non-selected states.

## 11. Execution Record

Represents the concrete execution attempt.

### Required Fields

- `placement_target`
- `status`

### Optional Fields

- `artifact_ref`
- `started_at`
- `completed_at`
- `output_digest`
- `failure_reason`

### Enum Values

`status`:

- `not_started`
- `succeeded`
- `failed`

### Failure Reasons

- `contract_input_invalid`
- `artifact_missing`
- `artifact_not_runnable`
- `execution_failed`
- `contract_output_invalid`

## 12. Runtime Result

Represents the terminal output returned by the runtime.

### Required Fields

- `kind`
- `schema_version`
- `execution_id`
- `request_id`
- `status`
- `trace_ref`

### Optional Fields

- `output`
- `error`

### Enum Values

`status`:

- `completed`
- `error`

### Error Shape

- `code`
- `message`
- `details`

### Error Codes

- `request_invalid`
- `capability_not_found`
- `capability_ambiguous`
- `capability_not_runnable`
- `artifact_missing`
- `execution_failed`
- `output_validation_failed`

## 13. Execution Identifier Rules

### `request_id`

- caller-supplied and preserved end to end

### `execution_id`

- runtime-produced stable identifier for one attempt
- must be reused across state events, trace, and runtime result

### `trace_id`

- trace-specific identifier
- must appear in the terminal runtime result as `trace_ref`

## 14. Deterministic Ordering Rules

Candidate ordering for this slice should be:

1. lookup scope precedence
2. capability id lexicographic order
3. semantic version descending
4. scope ordering as the final tie-breaker

### Rules

- deterministic ordering must happen before ambiguity evaluation
- ambiguity means more than one candidate remains eligible after deterministic filtering
- ordering alone must not silently choose one candidate when multiple still remain eligible

## 15. Runtime Eligibility Rules

For this slice, a candidate is runnable only when:

- lifecycle is runtime-eligible
- implementation kind is `Executable`
- artifact metadata exists
- artifact binary metadata exists
- contract execution metadata is compatible with local execution
- requested target is `local`

## 16. Evidence Linkage

The runtime trace and runtime result must be linkable to:

- the governing spec id
- the selected capability registration
- the selected artifact reference
- emitted runtime state events

This makes the slice suitable for later CI, UI, and MCP consumers.

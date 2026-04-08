# Data Model: Browser Runtime Subscription

## Purpose

This document defines the implementation-tight artifacts for the `013-browser-runtime-subscription` slice.

It governs the browser-facing subscription request and ordered message stream that wrap already-governed runtime artifacts.

## 1. Browser Runtime Subscription Request

Represents one machine-readable request to materialize browser-facing messages for one runtime outcome.

### Required Fields

- `kind`
- `schema_version`
- `governing_spec`

### Exactly-One Required Selector

- `request_id`
- `execution_id`

### Shape

```json
{
  "kind": "browser_runtime_subscription_request",
  "schema_version": "1.0.0",
  "governing_spec": "013-browser-runtime-subscription",
  "request_id": "req_20260331_0001"
}
```

### Rules

- `kind` MUST equal `browser_runtime_subscription_request`.
- `schema_version` MUST equal `1.0.0`.
- `governing_spec` MUST equal `013-browser-runtime-subscription`.
- Exactly one of `request_id` or `execution_id` MUST be present and non-empty.
- Providing both selectors or neither selector is invalid.

## 2. Browser Runtime Subscription Error Message

Represents one terminal validation or targeting failure for the browser subscription surface.

### Required Fields

- `kind`
- `schema_version`
- `sequence`
- `code`
- `message`

### Shape

```json
{
  "kind": "browser_runtime_subscription_error",
  "schema_version": "1.0.0",
  "sequence": 0,
  "code": "invalid_request",
  "message": "subscription request must include request_id or execution_id"
}
```

### Error Code Enum

- `invalid_request`
- `not_found`
- `unsupported_operation`

### Rules

- Invalid or mismatched requests MUST return exactly one error message.
- Error streams MUST NOT include additional lifecycle, state, trace, or terminal messages.
- `sequence` MUST be `0` for single-message error streams.

## 3. Browser Runtime Subscription Lifecycle Message

Represents one lifecycle boundary of the browser-facing stream.

### Required Fields

- `kind`
- `schema_version`
- `sequence`
- `request_id`
- `execution_id`
- `status`

### Shape

```json
{
  "kind": "browser_runtime_subscription_lifecycle",
  "schema_version": "1.0.0",
  "sequence": 0,
  "request_id": "req_20260331_0001",
  "execution_id": "exec_20260331_0001",
  "status": "subscription_established"
}
```

### Status Enum

- `subscription_established`
- `stream_completed`

### Rules

- `subscription_established` MUST be the first message in any valid stream.
- `stream_completed` MUST be the last message in any valid stream.

## 4. Browser Runtime Subscription State Message

Represents one browser-facing wrapper around a governed runtime state event.

### Required Fields

- `kind`
- `schema_version`
- `sequence`
- `state_event`

### Shape

```json
{
  "kind": "browser_runtime_subscription_state",
  "schema_version": "1.0.0",
  "sequence": 1,
  "state_event": {
    "kind": "runtime_state_event",
    "schema_version": "1.0.0",
    "event_id": "rse_20260331_0001",
    "request_id": "req_20260331_0001",
    "execution_id": "exec_20260331_0001",
    "state": "executing",
    "entered_at": "2026-03-31T00:00:00Z",
    "details": {
      "transition_reason": "candidate_selected"
    }
  }
}
```

### Rules

- `state_event` MUST embed the already-governed runtime state event shape from `010-runtime-state-machine`.
- State messages MUST preserve runtime state-event ordering exactly.

## 5. Browser Runtime Subscription Trace Artifact Message

Represents one browser-facing wrapper around the governed runtime trace.

### Required Fields

- `kind`
- `schema_version`
- `sequence`
- `trace`

### Shape

```json
{
  "kind": "browser_runtime_subscription_trace_artifact",
  "schema_version": "1.0.0",
  "sequence": 4,
  "trace": {
    "kind": "runtime_trace",
    "schema_version": "1.0.0",
    "trace_id": "trace_exec_20260331_0001"
  }
}
```

### Rules

- `trace` MUST embed the governed runtime trace artifact as-is.
- Exactly one trace artifact message MUST appear in each valid stream.

## 6. Browser Runtime Subscription Terminal Message

Represents one browser-facing wrapper around the governed runtime terminal result.

### Required Fields

- `kind`
- `schema_version`
- `sequence`
- `result`

### Shape

```json
{
  "kind": "browser_runtime_subscription_terminal",
  "schema_version": "1.0.0",
  "sequence": 5,
  "result": {
    "kind": "runtime_result",
    "schema_version": "1.0.0",
    "request_id": "req_20260331_0001",
    "execution_id": "exec_20260331_0001",
    "status": "completed"
  }
}
```

### Rules

- `result` MUST embed the governed runtime result artifact as-is.
- Exactly one terminal message MUST appear in each valid stream.

## 7. Browser Runtime Subscription Message Union

Represents the complete ordered stream message set.

### Allowed Variants

- `error`
- `lifecycle`
- `state`
- `trace_artifact`
- `stream_terminal`

### Valid Ordered Stream Shape

```json
[
  {
    "kind": "browser_runtime_subscription_lifecycle",
    "schema_version": "1.0.0",
    "sequence": 0,
    "request_id": "req_20260331_0001",
    "execution_id": "exec_20260331_0001",
    "status": "subscription_established"
  },
  {
    "kind": "browser_runtime_subscription_state",
    "schema_version": "1.0.0",
    "sequence": 1,
    "state_event": { "kind": "runtime_state_event" }
  },
  {
    "kind": "browser_runtime_subscription_trace_artifact",
    "schema_version": "1.0.0",
    "sequence": 2,
    "trace": { "kind": "runtime_trace" }
  },
  {
    "kind": "browser_runtime_subscription_terminal",
    "schema_version": "1.0.0",
    "sequence": 3,
    "result": { "kind": "runtime_result" }
  },
  {
    "kind": "browser_runtime_subscription_lifecycle",
    "schema_version": "1.0.0",
    "sequence": 4,
    "request_id": "req_20260331_0001",
    "execution_id": "exec_20260331_0001",
    "status": "stream_completed"
  }
]
```

### Rules

- `sequence` values MUST be monotonic and gap-free within one valid stream.
- Valid streams MUST use this order:
  1. `subscription_established`
  2. zero or more `state`
  3. one `trace_artifact`
  4. one `stream_terminal`
  5. `stream_completed`
- Error streams MUST contain only one `error` message.

## 8. Targeting Rules

Represents how a browser subscription request matches a runtime outcome.

### Rules

- If `request_id` is supplied, it MUST equal `runtime_result.request_id`.
- If `execution_id` is supplied, it MUST equal `runtime_result.execution_id`.
- A syntactically valid request that does not match the supplied outcome MUST emit a single `not_found` error message.

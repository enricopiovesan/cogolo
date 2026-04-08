# Data Model: Downstream Integration Validation and Release Readiness

## Purpose

This document defines the implementation-tight artifacts for the `020-downstream-integration-validation` slice.

It governs the validation paths, quickstart structure, acceptance evidence, and release-readiness records required for the first external-consumer Traverse release.

## 1. Downstream Validation Path Record

Represents one governed downstream validation path.

### Required Fields

- `path_id`
- `kind`
- `consumer_name`
- `required_surfaces`
- `setup_source`
- `validation_steps`
- `expected_terminal_outcome`
- `failure_modes`

### Shape

```json
{
  "path_id": "youaskm3_browser_runtime_validation",
  "kind": "browser_runtime",
  "consumer_name": "youaskm3",
  "required_surfaces": [
    "browser.runtime.request",
    "browser.runtime.subscription",
    "browser.runtime.terminal_result"
  ],
  "setup_source": "quickstart.first_app_flow",
  "validation_steps": [
    "start_supported_runtime_host",
    "start_local_browser_adapter",
    "start_browser_consumer",
    "submit_governed_request",
    "observe_ordered_runtime_updates",
    "observe_terminal_trace_and_result"
  ],
  "expected_terminal_outcome": "completed",
  "failure_modes": [
    "adapter_startup_failure",
    "structured_terminal_error"
  ]
}
```

### Rules

- `kind` must be one of:
  - `browser_runtime`
  - `mcp_consumption`
- `required_surfaces` must reference public consumer surfaces defined under `019-downstream-consumer-contract`.
- At least one `browser_runtime` record and one `mcp_consumption` record must exist for this slice.

## 2. Agent-Followable Quickstart Record

Represents the required structure of the first quickstart.

### Required Fields

- `quickstart_id`
- `document_path`
- `supported_consumers`
- `sections`
- `expected_artifacts`
- `known_failures`

### Shape

```json
{
  "quickstart_id": "first_app_flow",
  "document_path": "quickstart.md",
  "supported_consumers": [
    "human",
    "codex",
    "claude",
    "cursor"
  ],
  "sections": [
    "prerequisites",
    "setup",
    "run",
    "validate",
    "expected_outputs",
    "known_failures"
  ],
  "expected_artifacts": [
    "runtime_state_stream",
    "terminal_result",
    "trace_output"
  ],
  "known_failures": [
    "adapter_connection_failure",
    "structured_execution_failure"
  ]
}
```

### Rules

- `document_path` for v0.1 must resolve to `quickstart.md`.
- `sections` must contain all required sections listed above.
- `supported_consumers` must include both `human` and at least one agent identifier.

## 3. Acceptance Evidence Record

Represents deterministic end-to-end evidence for the first supported downstream flow.

### Required Fields

- `kind`
- `schema_version`
- `flow_id`
- `validated_at`
- `status`
- `steps_verified`
- `terminal_outcome`
- `failure_path_verified`

### Shape

```json
{
  "kind": "acceptance_evidence",
  "schema_version": "1.0.0",
  "flow_id": "first_external_consumer_flow",
  "validated_at": "2026-04-03T00:00:00Z",
  "status": "passed",
  "steps_verified": [
    "runtime_started",
    "adapter_connected",
    "request_submitted",
    "ordered_updates_observed",
    "trace_visible",
    "terminal_result_observed"
  ],
  "terminal_outcome": "completed",
  "failure_path_verified": true
}
```

### Rules

- `status` must be one of:
  - `passed`
  - `failed`
- `failure_path_verified` must be explicit; it must not be implied.
- `steps_verified` must remain machine-readable and reviewable in CI.

## 4. Release Readiness Record

Represents the explicit readiness state for the first external-consumer release.

### Required Fields

- `release_id`
- `consumer_class`
- `required_inputs`
- `blockers`
- `status`

### Shape

```json
{
  "release_id": "app_consumable_v0_1",
  "consumer_class": "external_app_first_consumer",
  "required_inputs": [
    "quickstart.first_app_flow",
    "youaskm3_browser_runtime_validation",
    "youaskm3_mcp_validation",
    "first_external_consumer_flow_acceptance"
  ],
  "blockers": [
    "browser_validation_missing",
    "mcp_validation_missing",
    "quickstart_missing",
    "acceptance_evidence_missing"
  ],
  "status": "blocked"
}
```

### Rules

- `status` must be one of:
  - `blocked`
  - `ready`
- `ready` is only valid when every required input exists and every blocker is resolved.
- `required_inputs` must reference records governed by this slice or by `019-downstream-consumer-contract`.

## 5. Validation Failure Record

Represents one expected failure mode under the first downstream validation model.

### Required Fields

- `failure_id`
- `path_id`
- `classification`
- `detection_rule`
- `expected_surface_behavior`

### Shape

```json
{
  "failure_id": "adapter_connection_failure",
  "path_id": "youaskm3_browser_runtime_validation",
  "classification": "setup_failure",
  "detection_rule": "adapter connection attempt returns structured failure payload",
  "expected_surface_behavior": "no undocumented retry loop; failure remains machine-readable"
}
```

### Rules

- Each validation path must define at least one expected failure record.
- Failure records must describe public-surface behavior, not internal logs alone.

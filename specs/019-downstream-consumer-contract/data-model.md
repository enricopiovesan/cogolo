# Data Model: Downstream Consumer Contract

## Purpose

This document defines the implementation-tight artifacts for the `019-downstream-consumer-contract` slice.

It governs how Traverse describes public app-facing surfaces, the first supported consumer flow, compatibility expectations, and release-readiness evaluation for external app use.

## 1. Public Consumer Surface Record

Represents one Traverse surface that an external app may intentionally depend on.

### Required Fields

- `surface_id`
- `kind`
- `version`
- `stability_level`
- `entrypoint`
- `consumer_guarantees`
- `internal_non_goals`

### Shape

```json
{
  "surface_id": "browser.runtime.subscription",
  "kind": "browser_subscription",
  "version": "1.0.0",
  "stability_level": "governed_public",
  "entrypoint": {
    "type": "adapter",
    "identifier": "local_browser_adapter"
  },
  "consumer_guarantees": [
    "ordered_runtime_updates",
    "machine_readable_terminal_outcome",
    "trace_visibility"
  ],
  "internal_non_goals": [
    "internal_crate_layout",
    "private_helper_modules",
    "undocumented_message_types"
  ]
}
```

### Rules

- `surface_id` must be stable and unique within the consumer contract.
- `kind` must be one of:
  - `browser_request`
  - `browser_subscription`
  - `browser_terminal_result`
  - `mcp_substrate`
  - `quickstart_flow`
  - `acceptance_flow`
- `stability_level` must be one of:
  - `governed_public`
  - `documented_preview`
- A `governed_public` surface is part of the downstream app contract.
- A downstream app must not depend on anything listed in `internal_non_goals`.

## 2. Supported Consumer Flow Record

Represents one approved end-to-end app-consumption path.

### Required Fields

- `flow_id`
- `consumer_type`
- `required_surfaces`
- `startup_steps`
- `execution_steps`
- `observation_steps`
- `failure_expectations`

### Shape

```json
{
  "flow_id": "first_browser_app_consumption",
  "consumer_type": "browser_hosted_app",
  "required_surfaces": [
    "browser.runtime.subscription",
    "browser.runtime.request",
    "browser.runtime.terminal_result",
    "quickstart.first_app_flow",
    "acceptance.first_app_flow"
  ],
  "startup_steps": [
    "start_runtime_host",
    "start_local_browser_adapter",
    "start_browser_app"
  ],
  "execution_steps": [
    "submit_governed_request",
    "observe_runtime_updates",
    "observe_terminal_result"
  ],
  "observation_steps": [
    "render_runtime_state_stream",
    "render_trace_artifact",
    "render_terminal_outcome"
  ],
  "failure_expectations": [
    "adapter_connection_failure_is_structured",
    "terminal_execution_failure_is_machine_readable"
  ]
}
```

### Rules

- `required_surfaces` must reference only defined public consumer surface ids.
- `consumer_type` for v0.1 must include at least `browser_hosted_app`.
- `failure_expectations` must remain machine-readable and reviewable.

## 3. Consumer Compatibility Rule

Represents one governed compatibility expectation for a public consumer surface.

### Required Fields

- `surface_id`
- `rule_id`
- `guarantee`
- `change_policy`

### Shape

```json
{
  "surface_id": "browser.runtime.subscription",
  "rule_id": "ordered_messages_are_stable",
  "guarantee": "Runtime state, trace, and terminal messages remain ordered and machine-readable for one execution.",
  "change_policy": "intentional_versioned_change_only"
}
```

### Rules

- `change_policy` must be one of:
  - `intentional_versioned_change_only`
  - `documented_preview_change_allowed`
- `governed_public` surfaces must use `intentional_versioned_change_only`.

## 4. Consumer Release Blocker Record

Represents one explicit blocker for claiming “app-consumable v0.1”.

### Required Fields

- `blocker_id`
- `description`
- `required_issue_or_artifact`
- `status`

### Shape

```json
{
  "blocker_id": "quickstart_exists",
  "description": "A first app-consumable quickstart exists and matches the real supported flow.",
  "required_issue_or_artifact": "#122",
  "status": "pending"
}
```

### Rules

- `status` must be one of:
  - `pending`
  - `satisfied`
- v0.1 app-consumable readiness must not be claimed while any blocker is `pending`.
- At minimum, blocker records must exist for:
  - browser adapter transport
  - live browser app path
  - quickstart
  - end-to-end acceptance validation

## 5. Consumer Validation Evidence

Represents proof that one downstream app path uses governed public surfaces successfully.

### Required Fields

- `kind`
- `schema_version`
- `consumer_name`
- `validated_flow_id`
- `validated_at`
- `status`
- `surfaces_used`
- `observed_terminal_outcome`

### Shape

```json
{
  "kind": "consumer_validation_evidence",
  "schema_version": "1.0.0",
  "consumer_name": "youaskm3",
  "validated_flow_id": "first_browser_app_consumption",
  "validated_at": "2026-04-03T00:00:00Z",
  "status": "passed",
  "surfaces_used": [
    "browser.runtime.request",
    "browser.runtime.subscription",
    "browser.runtime.terminal_result"
  ],
  "observed_terminal_outcome": "completed"
}
```

### Rules

- `consumer_name` for the first real validation path may be `youaskm3`.
- `status` must be one of:
  - `passed`
  - `failed`
- Validation evidence must be reviewable in CI or through documented repo artifacts.
- Validation evidence must not rely on undocumented internal-only surfaces.

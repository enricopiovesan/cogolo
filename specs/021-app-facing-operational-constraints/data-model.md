# Data Model: App-Facing Operational Constraints

## Purpose

This document defines the implementation-tight artifacts for the `021-app-facing-operational-constraints` slice.

It governs the first app-consumable performance baseline, the first app-facing safety-boundary rules, and the explicit v0.1 operational non-goals.

## 1. Performance Baseline Record

Represents the narrow measurable performance expectations for the first app-consumable path.

### Required Fields

- `baseline_id`
- `scope`
- `time_to_first_update_ms`
- `ordered_update_delivery_expectation`
- `end_to_end_usability_expectation`
- `measurement_context`

### Shape

```json
{
  "baseline_id": "first_app_flow_v0_1",
  "scope": "local_first_consumer_path",
  "time_to_first_update_ms": 1000,
  "ordered_update_delivery_expectation": "subsequent runtime updates should remain interactive and not stall the supported local browser flow",
  "end_to_end_usability_expectation": "the supported local first-consumer flow should feel interactive for one user-triggered execution",
  "measurement_context": "local supported developer setup for the first external consumer path"
}
```

### Rules

- `scope` for v0.1 must remain narrow and local-first.
- `time_to_first_update_ms` must be a measurable target, not only prose.
- `ordered_update_delivery_expectation` and `end_to_end_usability_expectation` may remain qualitative, but must still be explicit.

## 2. Safety Boundary Rule

Represents one explicit browser- or MCP-facing safety requirement.

### Required Fields

- `rule_id`
- `surface_kind`
- `requirement`
- `violation_example`
- `validation_expectation`

### Shape

```json
{
  "rule_id": "browser_path_preserves_governed_validation",
  "surface_kind": "browser_runtime",
  "requirement": "The app-facing browser path must not bypass governed validation before execution begins.",
  "violation_example": "a browser adapter directly triggers execution through an undocumented private helper path",
  "validation_expectation": "review or automated checks confirm that the public path goes through governed runtime validation"
}
```

### Rules

- `surface_kind` must be one of:
  - `browser_runtime`
  - `mcp_consumption`
- At least one rule must exist for `browser_runtime` and one for `mcp_consumption`.

## 3. Operational Non-Goal Record

Represents one explicit v0.1 operational constraint that is intentionally not promised yet.

### Required Fields

- `non_goal_id`
- `statement`
- `why_deferred`

### Shape

```json
{
  "non_goal_id": "full_auth_and_identity",
  "statement": "Traverse v0.1 does not guarantee full authentication and identity management for app-facing browser or MCP consumers.",
  "why_deferred": "the first release is focused on one governed local-first consumer path rather than a full production security architecture"
}
```

### Rules

- At minimum, records must exist for:
  - `full_auth_and_identity`
  - `multi_tenant_hardening`
  - `remote_deployment_security_guarantees`

## 4. Operational Validation Evidence

Represents one reviewable result for a performance or safety check.

### Required Fields

- `kind`
- `schema_version`
- `subject_id`
- `validated_at`
- `status`
- `method`
- `notes`

### Shape

```json
{
  "kind": "operational_validation_evidence",
  "schema_version": "1.0.0",
  "subject_id": "first_app_flow_v0_1",
  "validated_at": "2026-04-03T00:00:00Z",
  "status": "passed",
  "method": "documented local measurement and smoke validation",
  "notes": [
    "time to first runtime update stayed within the v0.1 baseline",
    "no governed-validation bypass was observed on the browser path"
  ]
}
```

### Rules

- `status` must be one of:
  - `passed`
  - `failed`
- `subject_id` may reference either a performance baseline or a safety boundary rule.
- Validation evidence must remain reviewable through CI, deterministic local commands, or documented release review.

# Data Model: Browser-Hosted MCP Consumer Model

## Purpose

This document defines the implementation-tight conceptual entities for the `023-browser-hosted-mcp-consumer-model` slice.

It governs the browser-hosted boundary between a downstream app like `youaskm3` and Traverse public surfaces without redefining runtime semantics.

## 1. Browser-Hosted Consumer Model

Represents one governed browser-facing consumption contract.

### Required Properties

- `consumer_model_id`
- `summary`
- `supported_public_surfaces`
- `transport_assumptions`
- `packaging_assumptions`
- `unsupported_assumptions`
- `compatibility_rules`

### Rules

- The model MUST identify which Traverse surfaces are intentionally public for browser-hosted use.
- The model MUST distinguish supported browser-hosted consumption from internal-only implementation details.
- The model MUST keep transport assumptions explicit rather than implied.

## 2. Supported Public Surface Set

Represents the Traverse surfaces a browser-hosted app may intentionally depend on.

### Required Properties

- `surface_id`
- `surface_type`
- `description`
- `governing_spec`
- `stability_expectation`

### Rules

- The surface set MUST include the browser-hosted path, the local adapter relation, and the dedicated MCP server relation at a governed level.
- The surface set MUST be version-aware and document what the app can rely on without guessing.

## 3. Published Consumer Bundle

Represents the versioned Traverse artifact set that a browser-hosted app depends on.

### Required Properties

- `bundle_id`
- `version`
- `summary`
- `governing_specs`
- `included_surface_groups`
- `compatibility_notes`

### Rules

- The bundle MUST be publishable as a versioned contract boundary.
- The bundle MUST not require downstream apps to import internal repository structure directly.

## 4. Browser-Hosted Downstream App Profile

Represents one browser-hosted consumer such as `youaskm3`.

### Required Properties

- `app_name`
- `hosted_context`
- `consumer_bundle_version`
- `expected_public_surfaces`
- `validation_expectations`

### Rules

- The app profile MUST remain generic enough for future browser-hosted downstream apps.
- The profile MUST not encode app-specific UX or branding decisions.

## 5. Compatibility Rule

Represents the stability promise for browser-hosted consumption.

### Required Properties

- `rule_id`
- `summary`
- `stable_surfaces`
- `versioning_expectation`
- `breaking_change_policy`

### Rules

- Compatibility MUST be described in terms of documented public surfaces.
- Unversioned drift MUST be treated as unsupported for browser-hosted consumers.

## 6. Unsupported Assumption

Represents a browser-hosted expectation explicitly out of scope for the first slice.

### Required Properties

- `assumption_id`
- `category`
- `description`
- `reason_out_of_scope`

### Rules

- Unsupported assumptions MUST include remote deployment, multi-tenant guarantees, and auth behavior unless a future governed slice adds them.
- Unsupported assumptions MUST be explicit so downstream app teams do not infer them from silence.

## 7. Release Readiness Boundary

Represents the minimum governed boundary a browser-hosted downstream app must satisfy before release readiness can be claimed.

### Required Properties

- `readiness_id`
- `required_docs`
- `required_validation` 
- `required_public_surfaces`
- `blocked_conditions`

### Rules

- Release readiness MUST reference the governed docs/specs rather than private repo knowledge.
- The boundary MUST be narrow enough that future implementation tickets can be created directly from it.

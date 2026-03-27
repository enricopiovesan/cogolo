# Data Model: Traverse Foundation v0.1

## Purpose

This document defines the initial artifact and runtime data shapes for `Foundation v0.1`.

It is intended to make the spec directly implementable.

## Versioning Rule

All governed artifacts below are versioned.

At minimum, `Foundation v0.1` treats the following as version-sensitive:

- feature specs
- capability contracts
- event contracts
- workflow definitions
- runtime + MCP surfaces

## 1. Governing Spec Record

Represents the spec that implementation must align with.

### Required Fields

- `id`
- `version`
- `status`
- `title`
- `path`
- `approved_at`
- `immutable`

### Notes

- `status` should distinguish draft from implementation-governing approval.
- once approved for implementation, `immutable` must be `true`
- later revisions should produce a new version or successor artifact

## 2. Capability Contract

Represents one portable business capability.

### Required Fields

- `id`
- `namespace`
- `name`
- `version`
- `lifecycle`
- `owner`
- `summary`
- `inputs`
- `outputs`
- `side_effects`
- `emits`
- `consumes`
- `permissions`
- `execution`
- `policies`
- `dependencies`
- `provenance`

### Execution Shape

`execution` should support:

- `binary_format` such as `wasm`
- `entrypoint`
- `preferred_targets`
- `constraints`

### Notes

- `Foundation v0.1` only executes with a `local` executor
- target metadata still exists to preserve future placement support

## 3. Event Contract

Represents an event as a first-class governed artifact.

### Required Fields

- `id`
- `namespace`
- `name`
- `version`
- `lifecycle`
- `owner`
- `summary`
- `schema`
- `classification`
- `publishers`
- `subscribers`
- `provenance`

### Notes

- event schema should be machine-validated
- semantic meaning matters in addition to payload structure

## 4. Workflow Definition

Represents deterministic graph-based traversal across capabilities.

### Required Fields

- `id`
- `name`
- `version`
- `lifecycle`
- `owner`
- `summary`
- `inputs`
- `outputs`
- `nodes`
- `edges`
- `events`
- `tags`

### Node Shape

Each node should identify:

- capability reference
- expected input mapping
- expected output mapping

### Edge Shape

Each edge should identify:

- source node
- target node
- trigger type such as direct or event-driven
- optional event reference

## 5. Registry Entry

Registry entries are the discoverable stored representation of governed artifacts.

### Common Fields

- `artifact_type`
- `id`
- `version`
- `lifecycle`
- `owner`
- `path`
- `checksum`
- `registered_at`

### Artifact Types

- `capability`
- `event`
- `workflow`

## 6. Runtime Request

Represents an invocation request to the runtime.

### Required Fields

- `request_id`
- `intent`
- `input`
- `context`
- `governing_spec`

### Context Shape

Context should support:

- execution target or local projection details
- policy-relevant attributes
- request metadata needed for deterministic evaluation

## 7. Runtime State Event

Represents a state machine transition visible to internal logic and subscribers.

### Required Fields

- `execution_id`
- `state`
- `timestamp`
- `request_id`
- `artifact_ref`
- `details`

### Expected States

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

## 8. Runtime Trace

Represents the explainable artifact of runtime decision and execution behavior.

### Required Fields

- `trace_id`
- `request_id`
- `governing_spec`
- `candidates`
- `constraint_filtering`
- `selection`
- `execution`
- `events`
- `result`

### Notes

- ambiguity failures must still produce a trace
- validation failures should produce evidence tied to the attempted artifact or request

## 9. Validation Evidence

Represents machine-readable proof that an artifact or change passed required validation.

### Required Fields

- `evidence_id`
- `artifact_ref`
- `governing_spec`
- `validation_type`
- `status`
- `produced_at`
- `details`

### Validation Types

- `spec_alignment`
- `contract_validation`
- `compatibility`
- `runtime_execution`
- `workflow_execution`
- `coverage`
- `ci_gate`

## 10. Decision Record Reference

Represents a link between implementation and architectural decision evidence.

### Required Fields

- `adr_id`
- `status`
- `path`

## 11. Exception Record

Represents an approved exception to normal rules.

### Required Fields

- `id`
- `rule`
- `reason`
- `scope`
- `owner`
- `risk`
- `mitigation`
- `review_date`
- `status`

## 12. Compatibility Record

Represents a machine-readable compatibility assessment for a versioned change.

### Required Fields

- `artifact_ref`
- `change_type`
- `compatible`
- `notes`

### Change Types

- `patch`
- `minor`
- `major`

## Open Questions for Implementation

- exact JSON schema strategy for contracts and workflows
- exact trace JSON shape for event-heavy workflows
- exact browser subscription payload format
- whether spec-governance metadata should live in a separate manifest or in each implementation area

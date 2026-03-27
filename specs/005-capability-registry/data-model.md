# Data Model: Cogolo Capability Registry

## Purpose

This document defines the exact `v0.1` capability registry data model and behavioral rules required by `005-capability-registry`.

It models the separation between:

- contract storage
- artifact metadata storage
- derived discovery index

## Registry Layers

The capability registry in `v0.1` consists of three logical layers:

1. **Contract Store**
   - immutable stored capability contract artifacts
2. **Artifact Store Metadata**
   - source and binary metadata records linked to published contract versions
3. **Discovery Index**
   - derived lookup-friendly entries built from the contract and artifact layers

## Scope Model

Supported registry scopes in `v0.1`:

- `public`
- `private`

Rules:

- private scope is a local/private overlay
- lookup in an application context MUST evaluate `private` before `public`
- scope does not change the contract schema

## Capability Registry Record

Represents the stored authoritative capability publication record.

### Required Fields

- `artifact_type`
- `scope`
- `id`
- `version`
- `lifecycle`
- `owner`
- `contract_path`
- `contract_digest`
- `implementation_kind`
- `artifact_ref`
- `registered_at`
- `provenance`
- `evidence`

### Field Rules

#### `artifact_type`

- required value: `capability`

#### `scope`

- enum:
  - `public`
  - `private`

#### `id`

- MUST match the published capability contract id

#### `version`

- MUST match the published capability contract version

#### `lifecycle`

- MUST match the published capability contract lifecycle

#### `contract_path`

- repository- or registry-relative path to the stored `contract.json`

#### `contract_digest`

- digest of the governed contract content

Rule:

- `(scope, id, version)` MUST be unique
- a duplicate `(scope, id, version)` with a different `contract_digest` MUST be rejected

#### `implementation_kind`

- enum:
  - `executable`
  - `workflow`

#### `artifact_ref`

- reference to a capability artifact record

Rules:

- executable capabilities MUST reference an executable artifact record
- composed capabilities MUST reference a workflow-backed artifact record

## Capability Artifact Record

Represents the implementation metadata linked to a published capability version.

### Required Fields

- `artifact_ref`
- `implementation_kind`
- `source`
- `binary`
- `workflow_ref`
- `digests`
- `provenance`

### Field Rules

#### `artifact_ref`

- unique identifier for the artifact record

#### `implementation_kind`

- enum:
  - `executable`
  - `workflow`

#### `source`

- type: object
- required fields:
  - `kind`
  - `location`

Allowed `kind` values in `v0.1`:

- `git`
- `local`

#### `binary`

- type: object
- required for `implementation_kind = executable`

Required fields:

- `format`
- `location`

Rules:

- `format` MUST equal `wasm` in `v0.1`

#### `workflow_ref`

- type: object
- required for `implementation_kind = workflow`

Required fields:

- `workflow_id`
- `workflow_version`

#### `digests`

- type: object
- required fields:
  - `source_digest`

Optional fields:

- `binary_digest`

#### `provenance`

- type: object
- required fields:
  - `source`
  - `author`
  - `created_at`

## Discovery Index Entry

Represents the derived lookup-friendly registry entry.

### Required Fields

- `scope`
- `id`
- `version`
- `lifecycle`
- `owner`
- `summary`
- `tags`
- `permissions`
- `emits`
- `consumes`
- `implementation_kind`
- `composability`
- `artifact_ref`
- `registered_at`

### `composability`

- type: object
- required: yes

Required fields:

- `kind`
- `patterns`
- `provides`
- `requires`

#### `composability.kind`

- enum:
  - `atomic`
  - `composite`

Rules:

- `atomic` implies `implementation_kind = executable`
- `composite` implies `implementation_kind = workflow`

#### `composability.patterns`

- type: array

Allowed values in `v0.1`:

- `sequential`
- `event-driven`
- `enrichment`
- `validation`
- `fan-out`
- `aggregation`

#### `composability.provides`

- type: array

Meaning:

- reusable downstream composition signals such as outputs, outcomes, or emitted events

#### `composability.requires`

- type: array

Meaning:

- upstream composition expectations such as required inputs, prior events, or prerequisite capability categories

## Version Compatibility Record

Represents the registry’s semver compatibility evaluation between a candidate version and the latest prior published version.

### Required Fields

- `capability_id`
- `previous_version`
- `candidate_version`
- `detected_change_class`
- `declared_bump`
- `result`
- `evidence_ref`

### `detected_change_class`

- enum:
  - `metadata_only`
  - `additive`
  - `breaking`
  - `unknown`

### `declared_bump`

- enum:
  - `patch`
  - `minor`
  - `major`

### Rules

- `metadata_only` changes MAY use `patch`
- `additive` changes MUST use at least `minor`
- `breaking` changes MUST use `major`
- `unknown` changes MUST fail closed in `v0.1` unless a reviewed exception process is later defined

## Lookup Rules

### Exact Lookup

Input:

- `scope_context`
- `id`
- `version`

Resolution order:

1. matching `private` record
2. matching `public` record

### Metadata Discovery

Queries may filter by:

- `owner.team`
- `lifecycle`
- `implementation_kind`
- `composability.kind`
- `composability.patterns`
- emitted event ids
- consumed event ids
- tags

Rules:

- results MUST be deterministic
- ties MUST be ordered lexicographically by `id`, then semver descending

## Registration Evidence

Successful registration MUST produce evidence containing:

- `evidence_id`
- `artifact_ref`
- `capability_id`
- `capability_version`
- `scope`
- `governing_spec_id`
- `validator_version`
- `produced_at`
- `result`

Rules:

- `result` MUST equal `passed` for successful registration evidence

## Semantic Rules

- The contract store is authoritative; the discovery index is derived.
- Contract schema is identical across public and private scope.
- Registry scope changes visibility and precedence, not contract semantics.
- A composed capability is a capability, not only a workflow.
- Artifact metadata must remain separate from the contract so hosting backends can evolve independently.
- Backstage may consume the discovery index later, but must not replace registry truth.

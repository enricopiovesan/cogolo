# Data Model: Cogolo Capability Contracts

## Purpose

This document defines the exact `v0.1` capability contract data model and validation rules required by `002-capability-contracts`.

It is intentionally implementation-tight so the first `cogolo-contracts` code can be written without inventing contract semantics during implementation.

## Artifact Identity

### Artifact Kind

`kind` MUST equal:

- `capability_contract`

### Schema Version

`schema_version` identifies the contract schema version, not the capability version.

For `v0.1`, supported value:

- `1.0.0`

### Capability Identity Rule

The capability identity fields are:

- `namespace`
- `name`
- `id`

Rules:

- `namespace` MUST be lowercase kebab-case segments separated by `.`
- `name` MUST be lowercase kebab-case
- `id` MUST equal `namespace.name`

Examples:

- namespace: `content.comments`
- name: `create-comment-draft`
- id: `content.comments.create-comment-draft`

## Top-Level Shape

```json
{
  "kind": "capability_contract",
  "schema_version": "1.0.0",
  "id": "content.comments.create-comment-draft",
  "namespace": "content.comments",
  "name": "create-comment-draft",
  "version": "0.1.0",
  "lifecycle": "draft",
  "owner": {
    "team": "cogolo-core",
    "contact": "enrico.piovesan10@gmail.com"
  },
  "summary": "Create a draft comment from validated request input.",
  "description": "Portable capability for creating a comment draft before persistence.",
  "inputs": {
    "schema": {
      "type": "object"
    }
  },
  "outputs": {
    "schema": {
      "type": "object"
    }
  },
  "preconditions": [
    {
      "id": "request-authenticated",
      "description": "Caller identity is already established."
    }
  ],
  "postconditions": [
    {
      "id": "draft-created",
      "description": "A draft payload is returned."
    }
  ],
  "side_effects": [
    {
      "kind": "memory_only",
      "description": "No durable side effect occurs in this capability."
    }
  ],
  "emits": [
    {
      "event_id": "content.comments.comment-draft-created",
      "version": "0.1.0"
    }
  ],
  "consumes": [],
  "permissions": [
    {
      "id": "comments.create"
    }
  ],
  "execution": {
    "binary_format": "wasm",
    "entrypoint": {
      "kind": "wasi-command",
      "command": "run"
    },
    "preferred_targets": [
      "local"
    ],
    "constraints": {
      "host_api_access": "none",
      "network_access": "forbidden",
      "filesystem_access": "none"
    }
  },
  "policies": [
    {
      "id": "default-comment-safety"
    }
  ],
  "dependencies": [],
  "provenance": {
    "source": "greenfield",
    "author": "enricopiovesan",
    "created_at": "2026-03-26T00:00:00Z"
  },
  "evidence": []
}
```

## Field Definitions

### `kind`

- type: string
- required: yes
- allowed value: `capability_contract`

### `schema_version`

- type: string
- required: yes
- allowed value in `v0.1`: `1.0.0`

### `id`

- type: string
- required: yes
- computed rule: MUST equal `namespace.name`

### `namespace`

- type: string
- required: yes
- format: dot-separated lowercase kebab-case segments
- regex: `^[a-z0-9]+(?:-[a-z0-9]+)*(?:\.[a-z0-9]+(?:-[a-z0-9]+)*)*$`

### `name`

- type: string
- required: yes
- format: lowercase kebab-case
- regex: `^[a-z0-9]+(?:-[a-z0-9]+)*$`

### `version`

- type: string
- required: yes
- format: semantic version `MAJOR.MINOR.PATCH`
- regex: `^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$`

### `lifecycle`

- type: string
- required: yes
- enum:
  - `draft`
  - `active`
  - `deprecated`
  - `retired`
  - `archived`

### `owner`

- type: object
- required: yes
- required fields:
  - `team`
  - `contact`

Rules:

- `team` MUST be a stable ownership identifier
- `contact` MUST be a non-empty contact string

### `summary`

- type: string
- required: yes
- min length: 10
- max length: 200

Rule:

- MUST describe one meaningful business action

### `description`

- type: string
- required: yes
- min length: 20

### `inputs`

- type: object
- required: yes
- required fields:
  - `schema`

Rule:

- `schema` MUST be a JSON-schema-like object shape suitable for machine validation

### `outputs`

- same rules as `inputs`

### `preconditions`

- type: array
- required: yes
- each item required fields:
  - `id`
  - `description`

Rules:

- item `id` values MUST be unique

### `postconditions`

- same rules as `preconditions`

### `side_effects`

- type: array
- required: yes
- minimum items: 1

Each item required fields:

- `kind`
- `description`

Allowed `kind` values in `v0.1`:

- `none`
- `memory_only`
- `event_emission`
- `external_call`
- `state_change`

### `emits`

- type: array
- required: yes

Each item required fields:

- `event_id`
- `version`

Rules:

- `(event_id, version)` pairs MUST be unique
- `version` MUST match semver format

### `consumes`

- same structure and uniqueness rules as `emits`

### `permissions`

- type: array
- required: yes

Each item required fields:

- `id`

Rules:

- permission `id` values MUST be unique

### `execution`

- type: object
- required: yes

Required fields:

- `binary_format`
- `entrypoint`
- `preferred_targets`
- `constraints`

#### `execution.binary_format`

- type: string
- required: yes
- allowed value in `v0.1`: `wasm`

#### `execution.entrypoint`

- type: object
- required: yes

Required fields:

- `kind`
- `command`

Allowed `kind` values in `v0.1`:

- `wasi-command`

Rules:

- `command` MUST be non-empty

#### `execution.preferred_targets`

- type: array
- required: yes
- minimum items: 1

Allowed values in `v0.1`:

- `local`
- `browser`
- `edge`
- `cloud`
- `worker`
- `device`

Rules:

- values MUST be unique
- validation MUST not reject non-`local` values
- runtime support in `v0.1` remains limited to `local`

#### `execution.constraints`

- type: object
- required: yes

Required fields:

- `host_api_access`
- `network_access`
- `filesystem_access`

Allowed values:

- `host_api_access`: `none`, `exception_required`
- `network_access`: `forbidden`, `required`
- `filesystem_access`: `none`, `sandbox_only`

Rules:

- if `host_api_access` equals `exception_required`, an approved portability exception reference MUST exist in `provenance`

### `policies`

- type: array
- required: yes

Each item required fields:

- `id`

Rules:

- policy `id` values MUST be unique

### `dependencies`

- type: array
- required: yes

Each item required fields:

- `artifact_type`
- `id`
- `version`

Allowed `artifact_type` values:

- `capability`
- `event`
- `policy`

Rules:

- `(artifact_type, id, version)` tuples MUST be unique
- `version` MUST match semver format

### `provenance`

- type: object
- required: yes

Required fields:

- `source`
- `author`
- `created_at`

Optional fields:

- `spec_ref`
- `adr_refs`
- `exception_refs`

Allowed `source` values:

- `greenfield`
- `brownfield-extracted`
- `ai-generated`
- `ai-assisted`

Rules:

- `spec_ref`, when present, MUST identify the governing implementation spec
- `exception_refs`, when present, MUST contain unique identifiers

### `evidence`

- type: array
- required: yes

Each item required fields:

- `evidence_id`
- `type`
- `status`

Allowed `type` values:

- `spec_alignment`
- `contract_validation`
- `compatibility`

Allowed `status` values:

- `passed`
- `failed`
- `superseded`

## Semantic Validation Rules

### Capability Boundary Rules

Validation MUST reject a contract when:

- `summary` or `description` indicates only a utility/helper concern
- the capability represents only CRUD storage mechanics with no business action
- the capability is described as a full application or subsystem instead of one business action
- `inputs`, `outputs`, and `side_effects` together do not form a meaningful business boundary

### Portability Rules

Validation MUST reject a contract when:

- `binary_format` is not `wasm`
- `execution.entrypoint.kind` is unsupported
- host-specific access is implied without exception metadata
- portability-sensitive execution constraints are missing

### Version and Immutability Rules

Validation against an existing published record MUST reject when:

- the same `id` and `version` already exist with different governed content
- the same `id` and `version` already exist and the lifecycle regression is not explicitly allowed by governance tooling

For `v0.1`, the governed content digest MUST be computed over all top-level fields except:

- `evidence`

### Lifecycle Rules

Allowed lifecycle meanings:

- `draft`: not publishable for runtime use
- `active`: eligible for runtime use
- `deprecated`: still valid but discouraged for new composition
- `retired`: no longer eligible for new runtime selection
- `archived`: retained as historical record only

For `v0.1` validation:

- new contracts may be authored in any lifecycle state
- only `active` and `deprecated` should be considered runtime-eligible downstream

## Validation Error Model

Each validation failure MUST produce:

- `code`
- `message`
- `path`
- `severity`

### Error Shape

```json
{
  "code": "invalid_semver",
  "message": "version must match MAJOR.MINOR.PATCH",
  "path": "$.version",
  "severity": "error"
}
```

### Initial Error Codes

- `missing_required_field`
- `invalid_literal`
- `invalid_format`
- `invalid_semver`
- `inconsistent_identity`
- `duplicate_item`
- `invalid_lifecycle`
- `invalid_capability_boundary`
- `unsupported_binary_format`
- `unsupported_entrypoint`
- `portability_exception_required`
- `immutable_version_conflict`
- `invalid_dependency_ref`

## Validation Evidence Model

Successful validation MUST produce:

- `evidence_id`
- `artifact_id`
- `artifact_version`
- `governing_spec`
- `validator_version`
- `status`
- `produced_at`

### Evidence Shape

```json
{
  "evidence_id": "evd_01HXYZ",
  "artifact_id": "content.comments.create-comment-draft",
  "artifact_version": "0.1.0",
  "governing_spec": "002-capability-contracts@0.1.0",
  "validator_version": "0.1.0",
  "status": "passed",
  "produced_at": "2026-03-26T00:00:00Z"
}
```

## Open Items Deferred from This Spec

- exact event contract schema
- exact workflow definition schema
- contract file hashing algorithm and canonical JSON normalization strategy
- registry persistence layout for published contract records

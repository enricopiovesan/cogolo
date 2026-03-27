# Data Model: Traverse Event Contracts

## Purpose

This document defines the exact `v0.1` event contract data model and validation rules required by `003-event-contracts`.

It is intentionally implementation-tight so the event contract portion of `traverse-contracts` can be written without inventing event semantics during implementation.

## Artifact Identity

### Artifact Kind

`kind` MUST equal:

- `event_contract`

### Schema Version

`schema_version` identifies the contract schema version, not the event version.

For `v0.1`, supported value:

- `1.0.0`

### Event Identity Rule

The event identity fields are:

- `namespace`
- `name`
- `id`

Rules:

- `namespace` MUST be lowercase kebab-case segments separated by `.`
- `name` MUST be lowercase kebab-case
- `id` MUST equal `namespace.name`

Examples:

- namespace: `content.comments`
- name: `comment-created`
- id: `content.comments.comment-created`

## Top-Level Shape

```json
{
  "kind": "event_contract",
  "schema_version": "1.0.0",
  "id": "content.comments.comment-created",
  "namespace": "content.comments",
  "name": "comment-created",
  "version": "0.1.0",
  "lifecycle": "draft",
  "owner": {
    "team": "traverse-core",
    "contact": "enrico.piovesan10@gmail.com"
  },
  "summary": "A comment has been created and is ready for downstream processing.",
  "description": "Governed event contract for newly created comments within the content comments domain.",
  "payload": {
    "schema": {
      "type": "object",
      "required": [
        "comment_id",
        "resource_id"
      ]
    },
    "compatibility": "backward-compatible"
  },
  "classification": {
    "domain": "content",
    "bounded_context": "comments",
    "event_type": "domain",
    "tags": [
      "comments",
      "notifications"
    ]
  },
  "publishers": [
    {
      "capability_id": "content.comments.persist-comment",
      "version": "0.1.0"
    }
  ],
  "subscribers": [
    {
      "capability_id": "content.comments.send-notification",
      "version": "0.1.0"
    }
  ],
  "policies": [
    {
      "id": "default-comment-publication"
    }
  ],
  "tags": [
    "comments",
    "created"
  ],
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
- allowed value: `event_contract`

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

- MUST describe the event's business meaning, not just its payload format

### `description`

- type: string
- required: yes
- min length: 20

### `payload`

- type: object
- required: yes
- required fields:
  - `schema`
  - `compatibility`

#### `payload.schema`

- type: object
- required: yes

Rule:

- MUST be a JSON-schema-like object shape suitable for machine validation

#### `payload.compatibility`

- type: string
- required: yes
- allowed values in `v0.1`:
  - `backward-compatible`
  - `forward-compatible`
  - `breaking`

### `classification`

- type: object
- required: yes
- required fields:
  - `domain`
  - `bounded_context`
  - `event_type`
  - `tags`

#### `classification.domain`

- type: string
- required: yes
- min length: 2

#### `classification.bounded_context`

- type: string
- required: yes
- min length: 2

#### `classification.event_type`

- type: string
- required: yes
- allowed values in `v0.1`:
  - `domain`
  - `integration`
  - `system`

#### `classification.tags`

- type: array
- required: yes
- minimum items: 1

Rules:

- values MUST be non-empty strings
- values MUST be unique

### `publishers`

- type: array
- required: yes
- minimum items: 1

Each item required fields:

- `capability_id`
- `version`

Rules:

- `(capability_id, version)` pairs MUST be unique
- `version` MUST match semver format

### `subscribers`

- type: array
- required: yes

Each item required fields:

- `capability_id`
- `version`

Rules:

- `(capability_id, version)` pairs MUST be unique
- `version` MUST match semver format

### `policies`

- type: array
- required: yes

Each item required fields:

- `id`

Rules:

- policy `id` values MUST be unique

### `tags`

- type: array
- required: yes
- minimum items: 1

Rules:

- values MUST be non-empty strings
- values MUST be unique

### `provenance`

- type: object
- required: yes
- required fields:
  - `source`
  - `author`
  - `created_at`

Allowed `source` values in `v0.1`:

- `greenfield`
- `brownfield`
- `ai-generated`
- `extracted`

Rules:

- `author` MUST be non-empty
- `created_at` MUST be an ISO 8601 UTC timestamp string

### `evidence`

- type: array
- required: yes

Each item required fields:

- `kind`
- `ref`

Rules:

- evidence entries are references to governed validation or approval records
- evidence entries MUST be unique by `(kind, ref)`

## Semantic Validation Rules

- An event contract MUST describe one governed event boundary, not a transport topic or implementation channel.
- An event contract MUST remain valid even if a specific runtime, broker, or delivery technology changes.
- An event contract MUST have a clear owner and MUST be rejected if ownership is absent or ambiguous.
- An event contract MUST include meaningful governance metadata and MUST be rejected if it is only a payload schema wrapper.
- An event contract with the same `id` and `version` as an existing published record MUST be rejected if the governed content digest differs.
- Validation MUST produce deterministic error ordering.

## Validation Error Model

Validation failures MUST be emitted as machine-readable records with:

- `code`
- `path`
- `severity`
- `message`

Expected code families:

- `structure.*`
- `identity.*`
- `version.*`
- `lifecycle.*`
- `classification.*`
- `publisher.*`
- `subscriber.*`
- `policy.*`
- `immutability.*`
- `governance.*`

## Validation Evidence Model

Successful validation MUST produce a machine-readable evidence record containing:

- `artifact_kind`
- `artifact_id`
- `artifact_version`
- `governing_spec_id`
- `governing_spec_version`
- `validator_version`
- `validated_at`
- `content_digest`
- `result`

Rules:

- `artifact_kind` MUST equal `event_contract`
- `result` MUST equal `passed` for successful validation evidence
- the evidence record MUST be stable enough for CI, registry, and runtime consumers

# Data Model: Event Registry

## Purpose

This document defines the implementation-tight artifacts for the `011-event-registry` slice.

It governs authoritative event storage, derived registry records, lookup semantics, lineage records, and registration evidence for event contracts.

## 1. Event Registration Request

Represents one event registration input boundary.

### Required Fields

- `scope`
- `contract`
- `governing_spec`

### Shape

```json
{
  "scope": "private",
  "contract": {
    "id": "expedition.planning.expedition-objective-captured",
    "version": "1.0.0"
  },
  "governing_spec": "011-event-registry"
}
```

### Rules

- `contract` refers to a validated event contract artifact loaded under `003-event-contracts`.
- `scope` values are defined by this slice and do not modify the event contract itself.

## 2. Event Registry Scope

Represents storage scope for event registrations.

### Enum Values

- `public`
- `private`

## 3. Event Lookup Policy

Represents deterministic scope resolution behavior.

### Enum Values

- `public_only`
- `prefer_private`

### Rules

- `public_only` searches only `public`.
- `prefer_private` searches `private` first, then `public`.

## 4. Event Registry Record

Represents one authoritative stored event version plus derived metadata.

### Required Fields

- `scope`
- `id`
- `version`
- `lifecycle`
- `owner`
- `summary`
- `classification`
- `publishers`
- `subscribers`
- `payload_schema_digest`
- `contract_digest`
- `contract_path`
- `validation_evidence`
- `registered_at`

### Optional Fields

- `tags`
- `policy_refs`
- `provenance`

### Shape

```json
{
  "scope": "private",
  "id": "expedition.planning.expedition-objective-captured",
  "version": "1.0.0",
  "lifecycle": "active",
  "owner": {
    "team": "traverse.examples",
    "contact": "team@traverse.local"
  },
  "summary": "Published when expedition planning objective capture is completed.",
  "classification": {
    "domain": "expedition.planning",
    "event_type": "domain_event",
    "stability": "stable"
  },
  "publishers": [
    "expedition.planning.capture-expedition-objective"
  ],
  "subscribers": [
    "expedition.planning.plan-expedition"
  ],
  "payload_schema_digest": "sha256:payload...",
  "contract_digest": "sha256:contract...",
  "contract_path": "contracts/examples/expedition/events/expedition-objective-captured/contract.json",
  "validation_evidence": {
    "governing_spec": "003-event-contracts",
    "status": "passed"
  },
  "registered_at": "2026-03-30T00:00:00Z",
  "tags": [
    "expedition",
    "planning"
  ],
  "policy_refs": [],
  "provenance": {
    "source": "examples"
  }
}
```

### Rules

- `contract_digest` is the governed-content digest used for immutability checks.
- `payload_schema_digest` may be derived from the normalized payload schema used by the event contract.
- `contract_path` identifies the authoritative stored artifact path, not just a source reference.

## 5. Event Lookup Result

Represents deterministic lookup output.

### Required Fields

- `status`
- `lookup_policy`
- `matches`

### Enum Values

`status`:

- `matched`
- `not_found`

### Shape

```json
{
  "status": "matched",
  "lookup_policy": "prefer_private",
  "matches": [
    {
      "scope": "private",
      "id": "expedition.planning.expedition-objective-captured",
      "version": "1.0.0"
    }
  ]
}
```

### Rules

- Exact lookup in this slice must return at most one match after lookup policy resolution.
- `not_found` must be explicit when no event exists for the requested identity/version under the lookup policy.

## 6. Event Lineage Record

Represents ordered published versions for one event id in one scope.

### Required Fields

- `scope`
- `id`
- `versions`

### Shape

```json
{
  "scope": "public",
  "id": "expedition.planning.expedition-objective-captured",
  "versions": [
    {
      "version": "1.0.0",
      "lifecycle": "active",
      "contract_digest": "sha256:contract-v1",
      "registered_at": "2026-03-30T00:00:00Z"
    },
    {
      "version": "1.1.0",
      "lifecycle": "active",
      "contract_digest": "sha256:contract-v1-1",
      "registered_at": "2026-04-05T00:00:00Z"
    }
  ]
}
```

### Rules

- `versions` must be ordered by semver ascending.
- Each entry remains immutable once published.

## 7. Event Registration Evidence

Represents machine-readable output from event registration validation.

### Required Fields

- `kind`
- `schema_version`
- `governing_spec`
- `status`
- `scope`
- `id`
- `version`
- `contract_digest`
- `validated_at`
- `checks`
- `violations`

### Shape

```json
{
  "kind": "event_registry_registration_evidence",
  "schema_version": "1.0.0",
  "governing_spec": "011-event-registry",
  "status": "passed",
  "scope": "private",
  "id": "expedition.planning.expedition-objective-captured",
  "version": "1.0.0",
  "contract_digest": "sha256:contract...",
  "validated_at": "2026-03-30T00:00:00Z",
  "checks": [
    "event_contract_valid",
    "scope_valid",
    "immutable_version_clear",
    "semver_progression_valid"
  ],
  "violations": []
}
```

### Rules

- `status` values:
  - `passed`
  - `failed`
- `violations` entries must identify the failed rule and enough structured detail to explain the rejection.

## 8. Event Registry Index Record

Represents a lightweight derived discovery projection.

### Required Fields

- `scope`
- `id`
- `version`
- `lifecycle`
- `summary`
- `publisher_count`
- `subscriber_count`
- `classification`
- `tags`

### Shape

```json
{
  "scope": "private",
  "id": "expedition.planning.expedition-objective-captured",
  "version": "1.0.0",
  "lifecycle": "active",
  "summary": "Published when expedition planning objective capture is completed.",
  "publisher_count": 1,
  "subscriber_count": 1,
  "classification": {
    "domain": "expedition.planning",
    "event_type": "domain_event",
    "stability": "stable"
  },
  "tags": [
    "expedition",
    "planning"
  ]
}
```

### Rules

- The index record is derived from the authoritative registry record and must not replace it.
- Consumers may use index records for discovery, but authoritative compatibility and immutability checks must still reference the full record.

## 9. Registration Semantics

### Rules

- Same `(scope, id, version, contract_digest)` may be re-registered idempotently.
- Same `(scope, id, version)` with a different `contract_digest` must fail.
- Same `(id, version)` may exist in both `public` and `private`.
- `prefer_private` resolution must choose `private` before `public`.

## 10. Implementation Notes

- This slice governs registry behavior only; it does not define how events are delivered at runtime.
- Runtime and workflow consumers should prefer derived event registry records for discovery and exact references, while preserving the authoritative contract artifact as source of truth.

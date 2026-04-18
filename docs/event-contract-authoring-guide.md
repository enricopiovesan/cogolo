# Event Contract Authoring Guide

This guide shows how to author a Traverse **event contract** from scratch.

Use the checked-in examples as living references:

- [`contracts/examples/expedition/events/`](../contracts/examples/expedition/events/)
- [`specs/003-event-contracts/data-model.md`](../specs/003-event-contracts/data-model.md)

## What An Event Contract Is

An event contract is the governed source of truth for an event type:

- identity (`id`, `version`)
- payload schema + compatibility signal (`payload.schema`, `payload.compatibility`)
- classification metadata (`classification`)
- publisher and subscriber edges (`publishers`, `subscribers`)

Traverse uses event contracts for validation, registry integrity, and workflow/event-driven composition.

## Minimal Working Template

This is a minimal event contract you can copy, edit, and validate locally.

```json
{
  "kind": "event_contract",
  "schema_version": "1.0.0",
  "id": "demo.echoed",
  "namespace": "demo",
  "name": "echoed",
  "version": "1.0.0",
  "lifecycle": "draft",
  "owner": { "team": "your-team", "contact": "you@example.com" },
  "summary": "A demo echo response was produced.",
  "description": "Minimal event contract used to validate authoring and registration wiring.",
  "payload": {
    "schema": {
      "type": "object",
      "required": ["message"],
      "properties": { "message": { "type": "string" } },
      "additionalProperties": false
    },
    "compatibility": "backward-compatible"
  },
  "classification": {
    "domain": "demo",
    "bounded_context": "core",
    "event_type": "domain",
    "tags": ["demo"]
  },
  "publishers": [
    { "capability_id": "demo.echo", "version": "1.0.0" }
  ],
  "subscribers": [],
  "policies": [{ "id": "manual-approval-required" }],
  "tags": ["demo", "example"],
  "provenance": {
    "source": "greenfield",
    "author": "your-handle",
    "created_at": "2026-04-18T00:00:00Z"
  },
  "evidence": []
}
```

## Field Notes

- `kind`: Must be `event_contract`.
- `schema_version`: Must be `1.0.0`.
- `id`: Globally unique string for the event type, typically `namespace.name`.
- `namespace` / `name`: Stable identity components.
- `version`: SemVer string. Increment when the event contract changes.
- `lifecycle`: Use `draft` while iterating; publishable flows require `active`.
- `payload.schema`: JSON Schema describing the event payload.
- `payload.compatibility`: Declares payload change compatibility; use `backward-compatible` unless you have a reason not to.
- `classification`: Metadata used for discovery and documentation.
- `publishers`: Capabilities allowed to emit this event (identity + version).
- `subscribers`: Capabilities that subscribe to this event (identity + version).
- `policies`: Governance policy identifiers (for example, manual approval).
- `tags`: Search / organization tags.
- `provenance`: Traceability metadata.
- `evidence`: Validation evidence records (often empty for a new draft).

## Lifecycle Values

| Value        | Meaning                                         |
|--------------|-------------------------------------------------|
| `draft`      | Not publishable for registry/runtime use        |
| `active`     | Eligible for registry and runtime use           |
| `deprecated` | Still valid but discouraged for new composition |
| `retired`    | No longer eligible for new selection            |
| `archived`   | Retained as historical record only              |

## Authoring Steps (Create → Validate → Register)

1. Choose `namespace`, `name`, and compute `id = namespace.name`.
2. Start with `lifecycle: draft`.
3. Define a strict `payload.schema`.
4. Add at least one `publisher` capability reference once the emitting capability exists.
5. Validate locally:

```bash
cargo test -p traverse-contracts
```

6. Inspect an event contract via the CLI (this should fail fast if malformed):

```bash
cargo run -p traverse-cli -- event inspect <path-to-contract.json>
```

7. Add the event contract to a bundle manifest and register it:

```bash
cargo run -p traverse-cli -- bundle register <path-to-manifest.json>
```

## Common Mistakes

- Using permissive payload schemas that undermine determinism.
- Forgetting to update `publishers` when the emitting capability version changes.
- Treating `tags` as a stability boundary. They are discoverability metadata, not identity.


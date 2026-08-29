# Feature Specification: Synced Public Registry Event Contract Preparation

**Feature Branch**: `claude/spec-126-event-preparation`
**Created**: 2026-08-29
**Status**: Approved
**Input**: Traverse #1215; Specs 055, 118, 124, 125, and 534.

## Purpose

Extend Spec 125's host-run public-bundle preparation to also carry the event
contracts a prepared public capability references through its `emits[]`. Without
this, `serve`'s registration-time contractual-enforcement gate fails closed for
any bundle containing an event-emitting capability, because the emitted-event
reference cannot resolve — the pipeline connects through `materialize` but
`serve` will not start. This spec closes that gap without adding request-time
network access to `serve`, `materialize`, or runtime execution, and without
changing the published registry index shape (which already carries `events[]`
per Traverse Spec 534 / registry FR-016).

## Capability Boundary

Event-contract preparation is part of the same host-owned
`registry prepare-public-bundle` capability defined by Spec 125. Its input is
still one synced-public-index state path; its output is still one complete local
registry-bundle directory — now including a local `events` tree and the
corresponding `bundle.json` `events` entries for events actually referenced by
prepared capabilities. Only referenced events are fetched. Artifact download and
verification remain owned by `registry materialize`; `serve` remains an offline
read-only consumer.

## Registry-Side Change (published `traverse-registry` crate)

The synced public-index surface must carry event pointers. In
`crates/traverse-registry/src/public_registry_state.rs`
(`traverse-framework/registry`, published to crates.io):

```rust
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct PublicRegistryEventRecord {
    pub namespace: String,
    pub id: String,
    pub version: String,
    pub product_digest: String,
    pub product_url: String,
    pub deprecated: bool,
}
```

- `PublicRegistryIndex` gains `#[serde(default)] pub events: Vec<PublicRegistryEventRecord>`.
- `SyncedPublicRegistryState` gains the same field; `write_synced_public_registry_state`
  sets `events: index.events`.
- `validate_public_registry_index` applies the same empty-field and
  duplicate-record checks to event records that it already applies to capability
  records.
- `#[serde(default)]` so an `index.json` generation or a synced state predating
  this change still deserializes (empty event list).
- Additive; a `traverse-registry` minor release, same pattern as `0.15.0`–
  `0.15.2` (registry `#312` / `#318` / `#319`). Workflows are the identical
  pattern and are deliberately left for a later additive change.

## Functional Requirements

- **FR-001**: `traverse-cli registry sync` MUST persist the fetched index's
  `events[]` into `SyncedPublicRegistryState.events`, each entry carrying
  `namespace`, `id`, `version`, `product_digest`, `product_url`, and
  `deprecated`. A fetched index with no `events` array MUST still sync
  successfully with an empty event list.
- **FR-002**: For every non-deprecated prepared capability, `prepare-public-bundle`
  MUST resolve each `emits[]` entry (`event_id` + `version`) of that capability's
  contract against `SyncedPublicRegistryState.events`. An `emits[]` reference
  with no matching non-deprecated event record MUST fail closed with a stable,
  capability-qualified, secret-free error.
- **FR-003**: For each referenced event, preparation MUST fetch its
  `product_url`, verify the downloaded bytes against `product_digest`, validate
  that the product's identity and version match the pointer, and write the
  product to a local `events/<id>/<version>/product.json` inside `--out`.
- **FR-004**: The generated `bundle.json` MUST list every prepared event under
  `events` with one local, relative product path, preserving exact event
  identity and version. Events not referenced by any prepared capability MUST
  NOT be fetched or included. `workflows` remains empty (its own spec).
- **FR-005**: A failed fetch, digest mismatch, identity mismatch, invalid path
  segment, or output-write failure MUST leave no partial bundle usable at
  `--out` and MUST preserve any prior complete output generation.
- **FR-006**: The command MUST make all event-contract network access during
  explicit host preparation only. `registry materialize`, `serve`, and runtime
  execution MUST make no network request for event contracts.
- **FR-007**: The command's JSON evidence MUST report a prepared-event count
  alongside Spec 125's prepared-capability count. Evidence MUST contain no
  credentials or raw product bytes.
- **FR-008**: Existing `registry sync`, `registry materialize`, and `serve`
  flags and behavior remain compatible. A synced state produced by a
  `traverse-registry` version predating the FR-001 field (no `events`) MUST make
  `prepare-public-bundle` behave exactly as under Spec 125 alone — no events
  prepared — so an event-emitting capability still fails closed at `serve` until
  the operator upgrades `traverse-registry`. That is an expected upgrade
  requirement, not a regression.

## Command Shape

Unchanged from Spec 125 — the same four commands. `prepare-public-bundle` now
additionally emits referenced event contracts and `bundle.json` `events`
entries.

```text
traverse-cli registry sync --workspace proxy --json
traverse-cli registry prepare-public-bundle \
  --synced-state .traverse/workspaces/proxy/registry/public/index.json \
  --out .traverse/workspaces/proxy/registry/prepared
traverse-cli registry materialize \
  --registry-state .traverse/workspaces/proxy/registry/prepared/bundle.json \
  --out .traverse/workspaces/proxy/artifacts
traverse-cli serve \
  --registry-state .traverse/workspaces/proxy/registry/prepared/bundle.json \
  --artifact-state .traverse/workspaces/proxy/artifacts/artifact-state.json
```

## Acceptance Scenarios

1. Given a synced state whose `events[]` contains `core.status-transitioned@1.0.0`
   and a prepared `core.transition-action-status@1.4.0` that emits it,
   preparation writes `events/core.status-transitioned/1.0.0/product.json` and a
   matching `bundle.json` `events` entry; `serve` registers the bundle with no
   unresolved-event error.
2. Given a prepared capability whose `emits[]` names an event absent from
   `events[]` (or matched only by a deprecated record), preparation fails closed
   with a capability-qualified error and preserves the prior output.
3. Given a referenced event whose fetched product bytes do not match
   `product_digest`, or whose identity/version do not match the pointer,
   preparation fails closed.
4. Given a prepared bundle in which no capability emits any event, `bundle.json`
   `events` is empty and behavior is identical to Spec 125.
5. Given a synced state produced by a `traverse-registry` version without the
   FR-001 `events` field, preparation succeeds with empty events (Spec 125
   behavior); an event-emitting capability then fails closed at `serve` with a
   message that identifies the missing event — the documented signal to upgrade
   `traverse-registry`.

## Compatibility and Out of Scope

Additive to Specs 125, 055, 118, 124, and 534. It does not change the published
`index.json` shape (which already carries `events[]`), does not mutate published
registry files, does not add a default output path, does not merge public and
private registrations, does not prepare workflows, and does not introduce
request-time refresh. The `traverse-registry` change is a single additive field
on the public sync types, guarded by `#[serde(default)]`, shipped as a minor
crate release; traverse then bumps its `traverse-registry` dependency.

## Approval Note

Approved on creation — this traces to an owner-participated decision on Traverse
#1215, where option 1 (extend the synced index, prepare referenced event
contracts) was selected explicitly by the maintainer, and the
`PublicRegistryEventRecord` shape was set in that thread. Registered in
`approved-specs.json` at `version 0.1.0`, governing `crates/traverse-cli/` and
this spec directory (the registry-side crate change is governed in
`traverse-framework/registry` under its own `055`/`019` lineage, not here).

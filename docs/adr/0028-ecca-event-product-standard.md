# ADR-0028: ECCA Event-Product Standard

- Status: Proposed
- Date: 2026-07-29
- Governing spec: `534-ecca-event-products`
- Related: Traverse #894, #895, #896, #897, #898, #899
- Extends: ADR-0016 and the existing event-contract/runtime boundary specs

## Context

Traverse already models event contracts and capability `emits`/`consumes`, but
published capabilities provide no real event-product adoption. A schema or a UI
badge does not establish ownership, semantic meaning, lifecycle, controlled
classification, policy enforcement, or operational proof. The result is an
event model that can exist without providing governed composition.

ECCA requires contracts to be first-class, discoverable, governed, and
observable products. Traverse needs one portable standard that works across
Rust, TypeScript, Swift, Kotlin, .NET, local hosts, and future hosted adapters.

## Decision

Adopt the ECCA event-product standard in Spec 534.

1. A meaningful asynchronous domain fact is a governed event product. Generic
   runtime lifecycle telemetry, commands, and requests are separate kinds.
2. The canonical contract combines a CloudEvents-compatible envelope with a
   strict native Traverse descriptor and a JSON Schema payload. AsyncAPI and
   language bindings are generated views.
3. Ownership is federated to the producing capability's domain/team under one
   shared, machine-enforced Traverse governance profile.
4. Descriptor metadata, semantic fact naming, controlled exposure/data classes,
   lifecycle, immutable semantic-version evolution, and compatibility are
   mandatory and validated at authoring and publication.
5. Traverse offers portable at-least-once delivery with idempotent consumers,
   explicit ordering, deduplication, correlation, and causation. It makes no
   exactly-once claim.
6. Catalog declarations and runtime-observed lineage remain separate records;
   OpenTelemetry-compatible evidence closes the drift and impact-analysis loop.
7. New publication is strictly blocked on invalid contracts. Runtime has an
   instrumented migration mode, then rejects invalid traffic with sanitized
   quarantine evidence after objective conformance criteria are met.
8. Capability and reference-app generation remain paused until the approved
   specification/ADR, passing validator fixtures, and end-to-end
   three-capability reference proof exist.

## Consequences

- Event contracts become enforceable platform interfaces rather than optional
  declaration fields.
- Registry, runtime, host adapters, generators, and reference applications have
  a single contract authority and a common conformance suite.
- Domain teams retain semantic autonomy but cannot choose incompatible metadata,
  schema, naming, delivery, or evidence conventions.
- Existing capabilities need an evidence-backed classification or migration
  before republication; no artificial event is required for non-asynchronous
  capabilities.
- The first implementation is deliberately incremental: strict new publication
  immediately, visible legacy migration, then objective runtime enforcement.

## Alternatives Considered

- **Simple Emits/Consumes badges**: rejected; they do not deliver governance,
  validation, discoverability, or runtime evidence.
- **Raw CloudEvents only**: rejected; portable transport metadata does not
  supply Traverse governance semantics.
- **AsyncAPI as the sole source**: rejected; useful derived documentation but
  too broad to be the runtime contract authority.
- **Producer-chosen schema/naming/classification**: rejected; makes the
  standard unvalidated and cross-capability composition unreliable.
- **Exactly-once delivery**: rejected; not a credible portable guarantee across
  Traverse hosts.
- **Immediate runtime rejection for all existing work**: rejected; it would
  block migration without evidence. Migration mode is observable and bounded.

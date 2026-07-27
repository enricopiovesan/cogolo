# ADR-0019: Make Durable DataStore Reachable Only Through Explicit Embedder Injection

- Status: Accepted
- Date: 2026-07-27
- Governing spec: `519-embedder-owned-datastore-integration`
- Extends: ADR-0018 / Spec 518

## Context

Spec 518 made `LocalFileDataStore` durable and fail-closed but deliberately left generic runtime and CLI execution without a storage root. The remaining reachability gap must not be closed by silently assigning persistence ownership to the runtime.

## Decision

Embedder hosts explicitly construct and inject one DataStore for one host-selected app or workspace scope. The root is opaque to Traverse. The initial host surface is stable, additive, and limited to host-only single-record read, write, and delete operations. Capabilities receive no direct DataStore access.

The store has one fixed public or private classification, one active owner, and stable typed failures. A second owner receives `store_locked`; no lease, concurrent writer, transaction, scan, migration, retention, backup, encryption, sync, or whole-store lifecycle behavior is introduced. Legacy and unknown persisted formats remain fail-closed. State telemetry contains only safe operation metadata.

The first conformance evidence is a Rust embedder example and restart/integrity integration test. Platform-specific lock lifecycle and crash-recovery evidence must be recorded before broadening the supported-platform claim.

## Consequences

- Durable state is reachable without weakening host ownership or tenant isolation.
- Existing users see no behavior change unless they opt in.
- Stateful capabilities, multi-process operation, and data lifecycle policy remain separate governed decisions.

## Alternatives Considered

- Runtime/CLI default storage: rejected because it silently chooses ownership and retention.
- Direct capability storage access: rejected because it bypasses host policy.
- Best-effort storage failures: rejected because durable state loss must be explicit.

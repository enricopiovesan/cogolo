# ADR-0027: Keep DataStore Synchronization Host-Owned and Deterministic

**Status:** Accepted
**Governing spec:** `088-datastore-synchronization`

## Decision

Traverse defines versioned, idempotent per-key mutation envelopes and deterministic last-writer-wins convergence ordered by Lamport counter then writer ID. Hosts own peer membership, credentials, transport selection, retention, and explicit reconciliation. The first conformance transport is an in-process deterministic test double.

## Consequences

Incremental replay is bounded by host retention; expired cursors fail with `resync_required`. Tombstones cannot be dropped in a way that permits stale replay to resurrect deleted data. A real local or hosted transport must pass the same conformance suite before it is supported.

## Alternatives considered

Full snapshots hide replay and deletion semantics. Type-specific CRDTs expand the contract beyond arbitrary DataStore values. Network transport and discovery are deferred until this protocol is proven.

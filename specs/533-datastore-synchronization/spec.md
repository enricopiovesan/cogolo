# Feature Specification: Provider-Neutral DataStore Synchronization

**Status**: Approved
**Canonical governing ID**: `086-datastore-synchronization`
**Extends**: `518-durable-local-datastore`, `519-embedder-owned-datastore-integration`

## Purpose

Define a transport-neutral synchronization protocol for host-owned DataStores. The first conformance transport is an in-process deterministic peer test double.

## Decisions

- A mutation envelope contains a protocol version, operation ID, synchronization-set ID, writer ID, Lamport counter, key, value or tombstone, and safe integrity metadata.
- Hosts provide an allowlist of opaque peer IDs for each synchronization set. Unauthorized peers are rejected.
- Same-key concurrent mutations converge by higher Lamport counter, then lexicographically higher writer ID.
- Per-peer opaque cursors replay retained envelopes. Expired cursors return `resync_required`; hosts explicitly request reconciliation.
- Tombstones follow host-declared synchronization retention. A peer outside that window must reconcile and must not replay stale writes.
- The initial transport is an in-process deterministic test double only. Network and hosted transports are out of scope.

## Requirements

- **FR-001**: Every envelope MUST have a unique operation ID; receivers MUST deduplicate retries.
- **FR-002**: A receiver MUST advance its Lamport clock before emitting a later mutation.
- **FR-003**: A receiver MUST expose deterministic conflict evidence without exposing a losing private value.
- **FR-004**: Membership, credentials, and transport authentication remain host-owned.
- **FR-005**: Cursor expiry and unauthorized membership MUST return stable machine-readable failures.
- **FR-006**: Reconciliation MUST be host-explicit and MUST not silently transfer a full snapshot.
- **FR-007**: Conformance MUST cover retry, replay, expiry, tombstone anti-resurrection, unauthorized peer, and conflict vectors.

## Definition of Done

- Deterministic local-peer fixtures prove every requirement.
- #883 implements only this approved protocol.

## Out of Scope

Network transports, hosted providers, peer discovery, credential storage, and distributed transactions.

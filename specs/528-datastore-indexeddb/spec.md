# Feature Specification: Browser IndexedDB DataStore Backend

**Feature Branch**: `528-datastore-indexeddb`  
**Created**: 2026-07-29  
**Status**: Approved  
**Canonical governing ID**: `085-datastore-indexeddb`  
**Extends**: `518-durable-local-datastore`, `519-embedder-owned-datastore-integration`  
**Related**: `084-datastore-encryption-at-rest` (private deferred on this backend)  
**Input**: Project 1 Specify IndexedDB ticket; planning locks recorded in Decision 41.

## Purpose

Define an IndexedDB storage backend that implements the same DataStore port and
integrity-envelope semantics as the local file adapter for browser embedders.
v1 is public CRUD with exclusive tab ownership. Private encryption and
maintenance (prune/backup/restore) are out of scope for this slice.

## Decisions

- IndexedDB is a backend for the same DataStore port and envelope rules, not a
  separate application API.
- v1 supports `public` records with integrity verification. `private`
  read/write fail closed until a browser `KeyProvider` follow-on is approved
  and implemented.
- Multi-tab concurrency uses exclusive ownership via Web Locks (or platform
  equivalent). Contenders receive `store_locked`. If locks are unavailable,
  open fails closed.
- `DataStoreMaintenance` operations return `unsupported` on this backend in
  v1. Native file-backed maintenance (Spec 083) lands first.
- Quota and persistence failures map to stable typed errors. No auto-prune and
  no silent dropped writes.

## Functional Requirements

- **FR-001**: Browser hosts MUST construct the DataStore with an explicit
  origin-scoped backend configuration; Traverse MUST NOT invent a global DB
  name outside host input.
- **FR-002**: Read/write/delete MUST preserve envelope integrity semantics from
  Spec 518 for supported classifications.
- **FR-003**: Private operations MUST fail with `key_provider_required` or
  `unsupported_classification` until a follow-on provider ships.
- **FR-004**: Open MUST acquire an exclusive Web Lock for the store identity;
  failure MUST return `store_locked` or `locking_unsupported`.
- **FR-005**: Quota exhaustion MUST return `quota_exceeded`. Persistence
  unavailable MUST return `persistence_unavailable`. Other backend failures
  MUST return `backend_failed` / `storage_io_failed` without DOM exception
  leakage as the public contract.
- **FR-006**: Maintenance APIs, if invoked, MUST return `unsupported` without
  mutating state.
- **FR-007**: Evidence/telemetry MUST be secret-free and MUST NOT include
  record payloads.

## Acceptance Scenarios

1. Given a browser host opens a store and holds the lock, when a second tab
   opens the same identity, then it receives `store_locked`.
2. Given public write/read across reload in the owner tab, when integrity
   verifies, then the value round-trips.
3. Given a private write, when no browser KeyProvider exists, then the call
   fails closed and no record is stored.
4. Given quota exceeded on write, when the API returns, then the error is
   `quota_exceeded` and no silent success occurs.
5. Given `backup` on the maintenance port, when called against IDB, then it
   returns `unsupported`.

## Out of Scope

- Private encryption via Web Crypto KeyProvider (Future follow-on).
- Maintenance parity (prune/backup/restore) on IDB.
- Multi-writer CRDT/merge semantics.
- Service worker–only ownership models beyond Web Locks rules above.

## Compatibility

Additive for web embedders. Native file adapter behavior is unchanged.

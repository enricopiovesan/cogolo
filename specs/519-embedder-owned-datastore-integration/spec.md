# Feature Specification: Embedder-Owned DataStore Integration

**Feature Branch**: `519-embedder-owned-datastore-integration`  
**Status**: Approved  
**Supersedes/extends**: `518-durable-local-datastore` for explicit host integration and ownership locking

## Purpose

This specification makes the approved durable local DataStore reachable through a stable, opt-in embedder integration. It does not create default runtime persistence, capability-owned storage, automatic migration, multi-record transactions, scans, replication, or multi-process coordination.

## Decisions

- An embedding host explicitly creates and injects a DataStore. Runtime and CLI paths never choose a root or create persistence implicitly.
- One injected store belongs to one host-defined app or workspace scope. The root is opaque to Traverse; Traverse does not derive subdirectories or add a second key namespace.
- Only the host performs explicit single-record read, write, and delete operations. Capabilities do not receive direct storage access.
- Classification is fixed when the host creates a store. Hosts use separate stores for public and private data.
- Storage failures are returned as stable typed DataStore errors. They do not alter unrelated capability execution unless the host makes a state operation a prerequisite.
- `local-datastore/1` is the only accepted persisted format. Unknown and legacy formats fail closed; migration is a separate approved slice.
- The host owns root creation, retention, backup, restore, and whole-store deletion. Traverse records safe structured metadata only and never records values, sensitive keys, or storage roots.

## Requirements

- **FR-001**: The public host API MUST expose additive, opt-in injection of a host-owned DataStore without changing behavior for hosts that do not configure one.
- **FR-002**: The injected store MUST be scoped to exactly one host-selected app or workspace boundary; runtime code MUST NOT derive filesystem paths or address another scope.
- **FR-003**: The first public operation set MUST be read, write, and delete of one validated record. Listing, scanning, batching, transactions, and capability-direct access are out of scope.
- **FR-004**: Typed DataStore errors, including `store_locked`, `integrity_check_failed`, `durability_commit_failed`, and `storage_io_failed`, MUST cross the host boundary with safe machine-readable detail.
- **FR-005**: State-operation telemetry MUST contain operation, outcome, classification, and stable error code only. It MUST NOT contain a state value, sensitive key, or storage root.
- **FR-006**: Local ownership remains exclusive. A second owner of the same root MUST receive `store_locked`; no lease or concurrent multi-process write policy is introduced.
- **FR-007**: Supported-platform lock lifecycle, owner release, owner-crash recovery, and deterministic unsupported-platform failure behavior MUST be recorded in ADR-0019 before platform-specific implementation is accepted.
- **FR-008**: A Rust embedder integration example and test MUST inject a private temporary store, persist a record, recreate the host, and verify durable reopen and integrity rejection.
- **FR-009**: Generic runtime and CLI execution without an injected store MUST create no persistent root and perform no implicit state write.

## Acceptance Criteria

- A host can inject a private or public store for one app/workspace and explicitly read, write, and delete a record.
- A fresh host instance can reopen the same injected root and read a committed record.
- A second owner deterministically receives `store_locked` and cannot change the committed record.
- Tampered, legacy, and unknown-format records return a typed failure without a state value.
- CI proves the Rust embedder example, restart behavior, safe telemetry, and no-default-persistence behavior.

## Out of Scope

- Capability-direct state permissions or implicit `state_schema` persistence.
- Retention, compaction, backup, restore, encryption, automatic migration, browser/remote adapters, synchronization, and multi-process coordination.
- Runtime-managed storage roots, namespaces, or whole-store deletion.

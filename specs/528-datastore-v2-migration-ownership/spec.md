# Feature Specification: DataStore v2 Migration and Ownership

**Feature Branch**: `codex/issue-848-datastore-v2-migration`
**Created**: 2026-07-28
**Status**: Approved
**Canonical governing ID**: `092-datastore-v2-migration-ownership`
**Input**: Issue #848, Decision 38, `524-production-app-readiness`, and Spec 082.

## Purpose

Define the first approved DataStore transition from `local-datastore/1` to a
host-owned file-backed `local-datastore/2`. A host can make an explicit,
recoverable migration with verified backup and restore evidence, while exactly
one local writer owns a root. Traverse never owns roots, keys, tenancy, or
authorization.

## Capability Boundary

The migration capability validates a host-provided owned store and an approved
format transition, writes a verified backup and target representation, then
commits atomically. The host supplies root handles, backup handles, encryption
and key access, tenancy mapping, and authorization. Runtime startup and normal
CLI commands never select a root or migrate implicitly.

## Functional Requirements

- **FR-001**: `local-datastore/2` MUST be a file-backed envelope with an
  explicit format id, format version, schema version, payload-integrity digest,
  and integrity metadata. It MUST reject unknown, malformed, and legacy
  envelopes with `datastore_source_invalid` without rewriting them.
- **FR-002**: Only a host supplying an owned root handle and authorization MAY
  request migration, restore, or ownership acquisition. Traverse APIs and
  evidence MUST not reveal roots, tenancy identifiers, stored values, or key
  material.
- **FR-003**: `local-datastore/1` to `local-datastore/2` is the sole approved
  transition. The migrator MUST validate the committed v1 representation before
  it writes a backup or candidate v2 file; unsupported transitions fail as
  `datastore_transition_unsupported`.
- **FR-004**: Before changing committed source data, migration MUST create and
  verify a host-directed backup bound to source/target format, source digest,
  backup digest, and creation result. Backup failure leaves the source readable
  and returns `datastore_backup_failed`.
- **FR-005**: Migration MUST write and verify a complete v2 candidate before an
  atomic commit. Any source-validation, backup, write, verification, or commit
  failure MUST preserve the last committed representation and return a stable
  error: `datastore_source_invalid`, `datastore_backup_failed`,
  `datastore_write_failed`, `datastore_verification_failed`, or
  `datastore_commit_failed`.
- **FR-006**: Restore MUST be explicit and host-authorized. It MUST verify the
  backup binding and integrity before atomic write; unknown, mismatched, or
  corrupt backup evidence fails as `datastore_restore_failed` without changing
  the current committed representation.
- **FR-007**: A root MUST have exactly one active local writer for v1. A second
  writer or migration attempt fails closed as `datastore_owner_locked`. Lock
  acquisition and release must be crash-safe and expose only safe lifecycle
  evidence.
- **FR-008**: Encryption is host-owned and opaque: the specification requires
  an encryption-disclosure field describing whether host protection is enabled
  and its algorithm/key-reference class, but never serializes a key, key id,
  plaintext, root, or credential in the envelope, backup, report, or error.
- **FR-009**: Safe reports MUST name source/target version, verified backup
  evidence, counts, ownership outcome, and stable error code. They MUST NOT
  contain data values, paths, keys, or tenancy information.

## Acceptance Scenarios

1. Given a host-owned valid v1 store, when it explicitly requests migration,
   then a verified backup and candidate v2 envelope are created before atomic
   commit, and the report names only safe evidence.
2. Given injected source, backup, candidate-write, verification, or commit
   failure, when migration runs, then the prior committed representation remains
   readable at every boundary and the matching stable error is returned.
3. Given a verified backup, when an authorized host explicitly restores it,
   then integrity is verified before commit. A corrupt or mismatched backup
   changes nothing and returns `datastore_restore_failed`.
4. Given one active owner for a root, when another process requests writer or
   migration access, then it receives `datastore_owner_locked` and cannot
   modify the store. After crash recovery, ownership can be safely reacquired.
5. Given an encrypted host store, when migration evidence is inspected, then it
   reports only the configured disclosure class and never keys, paths, values,
   or tenancy metadata.

## Governed Files and Conformance

Future implementation scope is limited to `crates/traverse-runtime/src/data_store.rs`,
`crates/traverse-runtime/tests/data_store_v2_migration.rs`, host adapter
surfaces, and normative fixtures in this directory. Independent conformance
must inject every failure boundary, exercise interruption and owner recovery,
verify source preservation and explicit restore, and test equivalent behavior
on every supported file-backed host platform. This ticket implements none.

## Compatibility and Out of Scope

This draft extends Spec 082. `local-datastore/1` remains readable until a host
explicitly migrates it; no runtime or CLI path may trigger migration. Remote or
browser adapters, synchronization, compaction, retention policy, automatic key
rotation, and multi-process coordination beyond the v1 single-writer guard are
out of scope. This draft must not be added to `approved-specs.json` without
explicit maintainer approval.

# Feature Specification: DataStore Retention and Verified Backup/Restore

**Feature Branch**: `526-datastore-retention-backup`  
**Created**: 2026-07-29  
**Status**: Draft  
**Canonical governing ID**: `083-datastore-retention-backup`  
**Extends**: `518-durable-local-datastore`, `519-embedder-owned-datastore-integration`  
**Input**: Project 1 Specify retention ticket; planning locks recorded in Decision 39.

## Purpose

Define host-explicit retention pruning and verified whole-store backup/restore
for embedder-owned durable local DataStores. Compaction, encryption, remote
adapters, browser backends, and multi-writer coordination are out of scope.

## Decisions

- Retention and backup/restore are invoked only through an explicit host
  maintenance API. Runtime capability execution never auto-prunes, backs up,
  or restores.
- Maintenance lives on a separate `DataStoreMaintenance` port constructed for
  the same host-owned root and exclusive ownership lock as the DataStore.
- Retention knobs in v1 are maximum record count and maximum age. Oldest-first
  ordering uses the store’s deterministic record order. Age compares each
  envelope’s canonical retained timestamp to a host-supplied `as_of` instant.
  Traverse MUST NOT read the OS wall clock for prune decisions.
- Prune is interruptible best-effort: it may remove a prefix of eligible
  records, then stop on failure, returning removed counts and a stable error.
  It MUST NOT leave corrupt envelopes.
- Backup produces a single zip archive with a mandatory `manifest.json`.
  Backup and restore require the exclusive ownership lock for the entire
  operation.
- Restore verifies the archive, writes to a temporary root, verifies again,
  then atomically replaces the store root. Merge restores are forbidden.
  Empty-store backup and restore are valid.
- Every maintenance call returns structured evidence. Appending that evidence
  to a durable journal is optional and host-opt-in only.

## Functional Requirements

- **FR-001**: Public maintenance operations MUST be exposed on
  `DataStoreMaintenance`, not mixed into ordinary read/write/delete.
- **FR-002**: `prune` MUST accept host policy `{ max_count?, max_age? }` and
  required `as_of`. At least one bound MUST be present. Records newer than
  `as_of - max_age` or within `max_count` newest MUST be retained.
- **FR-003**: `prune` MUST hold the exclusive ownership lock, emit evidence
  including attempted/removed/retained counts and stable failure reason, and
  remain crash-safe per envelope (no half-written records).
- **FR-004**: `backup` MUST hold the exclusive lock, write a zip whose members
  and `manifest.json` satisfy FR-006, and verify the archive content digest
  before success.
- **FR-005**: `restore` MUST hold the exclusive lock, verify the archive and
  manifest, materialize a temp root, verify envelopes, then atomically replace
  the destination root. Non-empty destinations are replaced only through this
  path (no merge).
- **FR-006**: `manifest.json` MUST include: manifest format version; created
  `as_of`/timestamp; record count; archive content digest; per-record index
  (key, classification, envelope digest, member path); store format id
  (`local-datastore/1` unless a later approved format applies); writer
  tool/semver identity. Host free-form notes are forbidden in v1.
- **FR-007**: Zip member paths MUST be UTF-8, pinned by the spec’s member
  layout, and use the Spec-pinned compression policy (store or deflate only as
  named in the ADR).
- **FR-008**: Empty stores MUST backup and restore successfully.
- **FR-009**: Evidence objects MUST be returned on success and failure for
  `prune_completed` / `prune_failed`, `backup_created` / `backup_failed`, and
  `restore_committed` / `restore_failed`. Evidence MUST be secret-free (no
  roots, keys, payloads).
- **FR-010**: Stable errors MUST include at least: `store_locked`,
  `invalid_retention_policy`, `backup_verify_failed`, `restore_verify_failed`,
  `unsupported_store_format`, `maintenance_io_failed`.

## Acceptance Scenarios

1. Given a locked store with 10 records and `max_count=3`, when the host prunes
   with `as_of`, then 7 oldest records are removed and evidence reports counts.
2. Given prune fails after deleting 2 of 7 victims, when the call returns, then
   those 2 are gone, remaining victims untouched, envelopes intact, and a
   stable error is returned with partial counts.
3. Given an exclusive owner, when backup runs, then a zip+manifest verifies
   and concurrent openers receive `store_locked`.
4. Given a verified backup, when restore runs onto a non-empty root, then the
   prior root is atomically replaced by the verified temp root with no merged
   keys.
5. Given an empty store, when backup and restore run, then both succeed.

## Out of Scope

- Compaction / defragmentation (Future).
- Encryption, key providers, classification changes.
- IndexedDB / remote KV backends (separate specs).
- Auto-prune on open/write/execute.
- OS wall-clock retention.
- In-archive merge restore or partial key restore.

## Compatibility

Additive to Specs 518/519. Existing read/write/delete behavior is unchanged
until hosts construct `DataStoreMaintenance`.

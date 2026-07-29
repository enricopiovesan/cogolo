# ADR-0021: Host-Explicit DataStore Retention and Verified Zip Backup/Restore

- Status: Accepted
- Date: 2026-07-29
- Governing spec: `526-datastore-retention-backup` / `083-datastore-retention-backup` (Draft)
- Extends: ADR-0018, ADR-0019

## Context

Specs 518/519 make durable local state embedder-owned with exclusive locking,
but provide no governed retention or disaster-recovery path. Folding auto-prune
into ordinary execution would silently assign lifecycle policy to Traverse.
Omitting backup/restore leaves migration and incident response ungoverned.

## Decision

Introduce a separate `DataStoreMaintenance` port for the same root and
exclusive lock. Hosts explicitly prune with count and/or age bounds and a
required `as_of` timestamp. Traverse never reads the OS clock for prune.

Backups are zip archives with a mandatory verification manifest. Backup and
restore take the exclusive lock for the whole operation. Restore verifies,
writes a temporary root, and atomically replaces the store root. Merge
restores are forbidden. Empty stores may be backed up and restored.

Prune is interruptible and may complete partially with stable evidence.
Maintenance always returns structured secret-free evidence; durable journal
append is host-opt-in only.

Compaction, encryption, remote/browser backends, and multi-writer access remain
separate decisions.

## Consequences

- Apps gain a deterministic ops surface without hidden retention.
- Implement work is blocked on approval of Spec 083.
- Compaction is explicitly Future.

## Alternatives Considered

- Auto-prune on open/write: rejected (hidden policy, non-determinism).
- Merge restore: rejected (ambiguous identity/order).
- Tar-only or host-opaque copies: rejected (weaker cross-platform ops story).

# Feature Specification: Explicit DataStore Format Migration and Recovery

**Status**: Approved
**Canonical governing ID**: `082-datastore-format-migration`
**Extends**: `518-durable-local-datastore`, `519-embedder-owned-datastore-integration`

## Purpose

Define the only supported path for an owning host to migrate a durable local
DataStore envelope when a future approved format supersedes
`local-datastore/1`. Generic runtime and CLI execution remain unable to select
a root, discover stores, or migrate data implicitly.

## Decisions

- Migration is explicitly requested by the embedder host that owns the store
  root; it is not a runtime startup action or a generic CLI action.
- A migration names an approved source-format to target-format transition.
  Unknown, legacy, malformed, and unapproved source versions remain
  fail-closed and are never guessed or rewritten.
- Before changing a committed record, the host-directed migrator creates a
  verified backup in a host-selected backup location. It writes and verifies
  the target representation before atomically committing it.
- A failed or interrupted migration leaves the source committed record
  readable. Restore is an explicit host operation from a verified backup; a
  target format is never silently downgraded.
- No retention, encryption, remote adapter, browser adapter, synchronization,
  transaction, scan, or automatic background migration is introduced.

## User Scenarios and Acceptance

### Host performs an approved migration

An embedder developer selects a known store root and an approved migration
transition. The migrator validates each source record, creates a verified
backup, commits the target representation, and returns a deterministic report
containing only counts, versions, and stable failures.

### Migration is interrupted or fails

If validation, backup, target write, verification, or atomic commit fails, the
previous committed source representation remains readable. The host receives a
stable error without a root, key, or stored value.

### Unsupported input is rejected

Legacy unverified files, unknown formats, malformed envelopes, and unapproved
format transitions return a typed failure and do not create a backup or target
record.

## Requirements

- **FR-001**: Only a host that explicitly supplies its owned root MAY invoke a
  migration or restore operation.
- **FR-002**: Each supported transition MUST be named in an approved successor
  specification before implementation.
- **FR-003**: Migration MUST validate source integrity before backup or write.
- **FR-004**: Backup metadata MUST bind source format, target format, record
  digest, and creation result without exposing values, sensitive keys, or roots.
- **FR-005**: Target data MUST be verified before atomic commit; a failed
  migration MUST preserve the prior committed representation.
- **FR-006**: Restore MUST be explicit, integrity-verified, and fail closed on
  an unknown, corrupted, or mismatched backup.
- **FR-007**: Runtime execution and ordinary CLI commands MUST continue to
  create no root and perform no implicit migration, backup, or restore.
- **FR-008**: Errors MUST be stable and machine-readable for unsupported
  transition, source validation, backup, commit, verification, and restore.
- **FR-009**: Tests MUST cover successful migration, every failure boundary,
  interruption recovery, explicit restore, and no-default-persistence behavior.

## Compatibility

`local-datastore/1` remains the only currently accepted format. This approved
policy does not authorize a new format or any implementation. Existing legacy and
unknown-format behavior remains fail-closed.

## Definition of Done

- An approved successor names at least one source-to-target transition.
- An implementation ticket contains the exact format, backup representation,
  stable errors, and host API contract.
- Cross-platform tests prove source preservation, verified backup, target
  verification, interruption recovery, and explicit restore.

## Out of Scope

Automatic migration, generic CLI migration, runtime-owned roots, retention,
compaction, encryption, synchronization, remote or browser storage, and
multi-process coordination.

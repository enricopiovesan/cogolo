# Feature Specification: Multi-Process DataStore Coordination Model

**Feature Branch**: `codex/issue-889-local-multiprocess-lock`
**Created**: 2026-07-29
**Status**: Draft — requires maintainer approval before implementation.
**Input**: Issue #878; extends Specs 518 and 519.

## Purpose

Define the coordination-model boundary for a host that deliberately permits
multiple processes to access one DataStore root. It preserves the current
single-owner fail-closed default and names the conditions a future coordinated
implementation must satisfy.

## Capability Boundary

The host supplies a local root on a filesystem that supports reliable advisory
locking. Traverse acquires one OS-backed exclusive lock before durable mutation
and releases it on process exit. Traverse never discovers roots, starts a
coordinator daemon, derives a tenant identity, or silently weakens the current
single-owner behavior.

## Functional Requirements

- **FR-001**: Single-process exclusive ownership remains the default. A host
  MUST explicitly opt into this coordination model; otherwise `store_locked`
  behavior from Specs 518/519 is unchanged.
- **FR-002**: One OS-backed exclusive lock MUST scope every local DataStore
  root. A contender receives `store_busy` and retries only when explicitly
  requested by its host; it MUST NOT spin or infer an owner deadline.
- **FR-003**: The lock MUST be held for mutation, migration, restore, backup,
  and maintenance. Readers observe only completed committed snapshots and MUST
  never read a partially committed record.
- **FR-004**: Crash recovery relies solely on operating-system lock release.
  No lease, heartbeat, fencing epoch, takeover timer, or coordinator daemon is
  permitted in this model.
- **FR-005**: Diagnostics MAY include a generated process-instance identifier,
  PID, and acquisition timestamp for local troubleshooting only. Public
  evidence MUST NOT disclose the root, command line, credentials, or tenant.
- **FR-006**: Coordination MUST preserve same-root atomic commit, integrity,
  backup, restore, and migration guarantees. It MUST NOT merge concurrent
  writes or redefine DataStore synchronization semantics.
- **FR-007**: Filesystems without reliable advisory locking MUST fail closed as
  `locking_unsupported`. Network filesystems and distributed coordination are
  out of scope.
- **FR-008**: Stable secret-free failures are `store_busy`,
  `locking_unsupported`, `coordination_unavailable`, and
  `coordination_protocol_incompatible`.
- **FR-009**: Safe coordination evidence MUST contain operation, outcome,
  lock state, retry outcome, and stable failure only. It MUST NOT contain
  roots, process command lines, credentials, tenant identity, or data.
- **FR-010**: This model does not select a distributed lock provider, define
  remote DataStore synchronization, or implement coordination. Implementation
  requires the separate approved ticket #879.

## Acceptance Scenarios

1. Given one process holds the OS lock, when another process requests the
   same root, then it returns `store_busy` and leaves committed records
   unchanged.
2. Given an owner crashes, when the OS releases its lock, then a later owner
   can acquire it and read only the last completed committed snapshot.
3. Given a reader overlaps a writer's atomic commit, when it reads, then it
   observes either the previous or next committed record, never a partial one.
4. Given an unsupported filesystem, when a host opens the root, then it
   returns `locking_unsupported` without a mutation.

## Compatibility and Governed Files

This draft is additive and preserves current single-owner locking. A future
implementation is limited to local advisory-lock integration and cross-process
conformance harnesses under `crates/traverse-runtime/` and
`crates/traverse-embedder/`. Root discovery and synchronization transport are
out of scope.

## Out of Scope

- Implementing distributed leases, a coordinator service, or background recovery.
- CRDTs, replication, merge semantics, or remote DataStore synchronization.
- Automatic root selection, process discovery, or silent fallback to shared
  access.
- Adding this draft to the approved registry without maintainer approval.

## Independent Conformance Evidence

Cross-process fixtures on supported local filesystems must cover contention,
owner crash and OS-release recovery, explicit retry, reader/writer snapshot
visibility, unsupported roots, and proof that rejected contenders leave
committed records unchanged.

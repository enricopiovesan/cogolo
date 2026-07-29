# Feature Specification: Multi-Process DataStore Coordination Model

**Feature Branch**: `codex/issue-878-multiprocess-datastore-model`
**Created**: 2026-07-29
**Status**: Draft — requires maintainer approval before implementation.
**Input**: Issue #878; extends Specs 518 and 519.

## Purpose

Define the coordination-model boundary for a host that deliberately permits
multiple processes to access one DataStore root. It preserves the current
single-owner fail-closed default and names the conditions a future coordinated
implementation must satisfy.

## Capability Boundary

The host supplies a root and explicitly configures a coordination authority.
Traverse acquires, renews, validates, and releases a scoped ownership lease
before durable mutation. Traverse never discovers roots, derives process
identity, selects a distributed lock service, or silently weakens the current
single-owner behavior.

## Functional Requirements

- **FR-001**: Single-process exclusive ownership remains the default. A host
  MUST explicitly opt into this coordination model; otherwise `store_locked`
  behavior from Specs 518/519 is unchanged.
- **FR-002**: Every coordination attempt MUST include a host-provided root
  scope, process identity, owner epoch, and bounded lease deadline. These are
  opaque identifiers and MUST NOT expose root paths, user identity, or
  credentials in public evidence.
- **FR-003**: A process MUST hold a valid exclusive lease before it begins a
  durable mutation, backup, restore, migration, or prune operation. Failed or
  expired acquisition returns `store_locked`; no write is attempted.
- **FR-004**: Lease renewal MUST be explicit and bounded. A process that cannot
  prove renewal before expiry MUST stop new mutations and return
  `store_lease_expired`; it MUST NOT assume ownership or extend a lease
  locally.
- **FR-005**: Recovery after owner crash requires an expired lease plus a
  host-provided fencing epoch greater than the prior epoch. A stale owner MUST
  fail closed as `store_fenced` before it can commit a record.
- **FR-006**: Coordination MUST preserve same-root atomic commit, integrity,
  backup, restore, and migration guarantees. It MUST NOT merge concurrent
  writes or redefine DataStore synchronization semantics.
- **FR-007**: Fairness policy is FIFO among observable contenders for one
  coordination authority. If the authority cannot provide deterministic
  ordering, it MUST declare `coordination_fairness_unsupported` and fail
  closed rather than claiming fairness.
- **FR-008**: Stable secret-free failures are `store_locked`,
  `store_lease_expired`, `store_fenced`, `coordination_unavailable`,
  `coordination_protocol_incompatible`, and
  `coordination_fairness_unsupported`.
- **FR-009**: Safe coordination evidence MUST contain operation, outcome,
  owner epoch, lease state, wait outcome, and stable failure only. It MUST NOT
  contain roots, process command lines, credentials, tenant identity, or data.
- **FR-010**: This model does not select a lock/lease provider, define remote
  DataStore synchronization, or implement coordination. Provider adapters and
  implementation require a separate approved ticket.

## Acceptance Scenarios

1. Given one process holds a valid lease, when another process requests the
   same root, then it returns `store_locked` and leaves committed records
   unchanged.
2. Given an owner crashes and its lease expires, when a host supplies a greater
   fencing epoch, then a recovery owner can acquire the lease; the stale owner
   receives `store_fenced` before a write.
3. Given a lease cannot renew before its deadline, when a mutation begins,
   then it returns `store_lease_expired` without attempting a commit.
4. Given observable FIFO contenders, when ownership is released, then the next
   acquisition produces deterministic wait evidence.

## Compatibility and Governed Files

This draft is additive and preserves current single-owner locking. A future
implementation is limited to a portable coordination port, host integration,
and cross-process conformance harnesses under `crates/traverse-runtime/` and
`crates/traverse-embedder/`. Provider SDKs, root discovery, and synchronization
transport are out of scope.

## Out of Scope

- Implementing leases, lock services, provider SDKs, or background recovery.
- CRDTs, replication, merge semantics, or remote DataStore synchronization.
- Automatic root selection, process discovery, or silent fallback to shared
  access.
- Adding this draft to the approved registry without maintainer approval.

## Independent Conformance Evidence

A portable test-double authority must cover contention, FIFO ordering, owner
crash recovery, fencing of stale owners, renewal expiry, unavailable authority,
and proof that all rejected contenders leave committed records unchanged.

# Feature Specification: DataStore Synchronization Protocol

**Feature Branch**: `codex/issue-877-datastore-sync-protocol`
**Created**: 2026-07-29
**Status**: Draft — requires maintainer approval before synchronization implementation.
**Input**: Issue #877; extends Specs 518 and 519.

## Purpose

Define a provider-neutral protocol for an explicit host-requested synchronization
attempt between two DataStore replicas. It formalizes deterministic merge and
evidence semantics without choosing a transport, cloud provider, background
replication model, or runtime-owned storage.

## Capability Boundary

An embedding host supplies two authenticated, authorized replicas or a
transport adapter and explicitly requests one synchronization attempt. Traverse
compares safe record metadata, selects deterministic winners, applies the
approved changes, and returns auditable merge evidence. The host owns endpoint,
identity, credentials, tenancy, scheduling, connectivity, and retry policy.

## Functional Requirements

- **FR-001**: Synchronization MUST be explicitly requested by a host. Runtime
  and CLI paths MUST NOT discover peers, start background sync, choose a
  transport, or retain an offline queue.
- **FR-002**: Every attempt MUST be scoped to one host-defined replica pair and
  tenant/workspace. A mismatched, missing, or unauthorized scope MUST fail
  closed as `sync_scope_denied` without exposing records or peer identity.
- **FR-003**: Protocol messages MUST have a version, attempt identifier,
  replica identity, scope assertion, opaque cursor, and idempotency key.
  Cursors and IDs are protocol metadata; they MUST NOT contain state values,
  credentials, endpoint paths, or tenant identifiers.
- **FR-004**: The v1 local-peer/test-double protocol is request/response only:
  handshake, page exchange, deterministic decision set, apply acknowledgement,
  and completed or interrupted outcome. Hosted transport is a successor.
- **FR-005**: For one key, a greater Lamport clock wins. On equal clocks, the
  lexicographically greater writer identifier wins. Local-only and remote-only
  records are selected as such. Every decision MUST state its rule.
- **FR-006**: A repeated attempt with the same idempotency key and unchanged
  replica inputs MUST yield the same decision set and MUST NOT create a second
  logical change. An interrupted attempt returns `sync_interrupted` with safe
  continuation evidence; it is never reported as completed.
- **FR-007**: Each selected record MUST retain the versioned envelope,
  classification, and integrity verification required by Spec 518. Invalid or
  tampered remote metadata returns `sync_integrity_failed` before application.
- **FR-008**: Stable secret-free failures are `sync_scope_denied`,
  `sync_protocol_incompatible`, `sync_cursor_invalid`, `sync_interrupted`,
  `sync_integrity_failed`, `sync_transport_unavailable`, and
  `sync_apply_failed`.
- **FR-009**: Evidence MUST include protocol version, attempt outcome, cursor
  progress, counts, decision rules, retry count, and stable failures only. It
  MUST NOT disclose state values, keys, credentials, peer endpoints, or tenant
  identity.
- **FR-010**: Synchronization is distinct from remote storage. This draft does
  not approve a provider adapter, hosted transport, CRDT, multi-process lease,
  automatic retry, or conflict rule other than FR-005.

## Acceptance Scenarios

1. Given disconnected local peers with overlapping records, when a host starts
   an attempt, then every conflict resolves by Lamport clock then writer ID and
   produces one safe decision record.
2. Given the same idempotency key and unchanged inputs, when the host repeats
   the attempt, then it returns the same decision set without a duplicate
   logical application.
3. Given an interrupted page exchange, when the attempt returns, then it
   reports `sync_interrupted` with cursor progress and no completed outcome.
4. Given a remote envelope with an invalid digest, when validation runs, then
   it returns `sync_integrity_failed` and applies no affected record.

## Compatibility and Governed Files

This draft formalizes and preserves the existing Lamport-clock then writer-ID
merge behavior. It is additive to the DataStore port and does not change local
CRUD. Future implementation is limited to a protocol surface under
`crates/traverse-runtime/`, a local-peer/test-double conformance harness, and
explicit host integration; hosted transports require a separate successor.

## Out of Scope

- Hosted transport, a provider SDK, cloud replication, or transport discovery.
- Background synchronization, offline queues, automatic retry, or scheduling.
- CRDT semantics, transactions, scanning beyond protocol paging, or multi-process
  coordination.
- Adding this draft to the approved registry without maintainer approval.

## Independent Conformance Evidence

An adapter-independent local-peer test double must prove deterministic
Lamport/writer decisions, idempotent replay, cursor rejection, interruption,
integrity failure, denied scope, and secret-free evidence.

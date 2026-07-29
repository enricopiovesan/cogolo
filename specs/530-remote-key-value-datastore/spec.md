# Feature Specification: Remote Key-Value DataStore Contract

**Feature Branch**: `codex/issue-874-remote-datastore-contract`
**Created**: 2026-07-29
**Status**: Draft — requires maintainer approval before any adapter implementation.
**Input**: Issue #874; extends Specs 518 and 519.

## Purpose

Define the portable contract boundary for a host-selected remote key-value
DataStore adapter. This is a contract and conformance specification only: it
does not select a provider, establish synchronization, or grant Traverse
ownership of credentials, tenancy, or remote storage.

## Capability Boundary

An embedding host explicitly supplies a remote DataStore adapter for one
host-defined app or workspace. Traverse validates the same state/envelope
semantics as the local DataStore port and returns deterministic, secret-free
outcomes. The host owns endpoint selection, authentication, tenant mapping,
credential rotation, network policy, costs, and availability commitments.

## Functional Requirements

- **FR-001**: The adapter MUST be explicitly injected through the existing
  host-owned DataStore boundary. Runtime and CLI paths MUST NOT choose a
  provider, endpoint, namespace, credential, or remote root.
- **FR-002**: The first remote operation set is single-record `read`, `write`,
  and `delete` for one host-selected scope. Scan, transactions, batch writes,
  provider discovery, and cross-scope access are out of scope.
- **FR-003**: A successful write MUST be durably acknowledged by the selected
  adapter before success is returned. Ambiguous transport failure MUST return
  `remote_outcome_unknown`; it MUST NOT be reported as success.
- **FR-004**: Each remote record MUST preserve the versioned envelope,
  classification, and integrity verification rules from Spec 518. The remote
  adapter MUST fail closed on malformed, unknown, or digest-mismatched data.
- **FR-005**: The adapter contract MUST name its consistency guarantee per
  record. v1 conformance requires read-your-write for the owning host after an
  acknowledged write; weaker or cross-client guarantees require a successor
  contract and MUST NOT be inferred.
- **FR-006**: Authentication, authorization, and tenancy decisions remain
  host-owned. Traverse receives only a host-provided adapter and MUST expose
  `remote_unauthorized` or `remote_scope_denied` without credentials, endpoint
  paths, tenant identities, or provider exception text.
- **FR-007**: Stable secret-free failures are `remote_unavailable`,
  `remote_timeout`, `remote_outcome_unknown`, `remote_unauthorized`,
  `remote_scope_denied`, `remote_integrity_failed`, and `remote_backend_failed`.
- **FR-008**: Retry policy is host-configured and bounded. The adapter MUST
  expose whether an operation may have reached the provider; it MUST NOT retry
  non-idempotent writes invisibly after an ambiguous outcome.
- **FR-009**: Every conforming adapter MUST produce safe evidence containing
  operation, outcome, classification, consistency mode, retry count, and
  stable failure code only. It MUST NOT include values, keys, credentials,
  endpoints, tenant IDs, or provider request identifiers.
- **FR-010**: Remote storage is not synchronization. This specification does
  not define replication, cursors, conflict resolution, offline queues, or
  merge behavior; those require a separate synchronization protocol.

## Acceptance Scenarios

1. Given a host-injected remote adapter acknowledges a write, when that host
   reads the same record, then it observes the integrity-verified record.
2. Given a write times out after submission, when the adapter returns, then it
   reports `remote_outcome_unknown` and safe evidence rather than success.
3. Given a provider returns an authorization failure, when Traverse projects
   it to the host, then it returns `remote_unauthorized` without provider or
   credential details.
4. Given remote bytes whose envelope digest is altered, when they are read,
   then the adapter returns `remote_integrity_failed` and no state value.

## Compatibility and Governed Files

This contract is additive to the existing DataStore port. It preserves local
adapter behavior and does not change the public DataStore trait. A follow-up
implementation is limited to a provider-neutral adapter contract under
`crates/traverse-runtime/`, its focused conformance tests, and an explicit
host integration. Provider SDKs, cloud endpoints, and credentials are not
governed by this draft.

## Out of Scope

- A provider-specific adapter, SDK dependency, hosted service, or credential
  store.
- Synchronization, offline queues, replication, conflict merging, or remote
  browsing.
- Remote retention, backup, encryption-key management, and cost optimization.
- Adding this draft to `approved-specs.json` without maintainer approval.

## Independent Conformance Evidence

An adapter-independent test double must cover acknowledged read-your-write,
timeout/ambiguous-write outcomes, denied scope, integrity mismatch, bounded
retry evidence, and zero disclosure of values, keys, credentials, endpoints,
or tenant identifiers.

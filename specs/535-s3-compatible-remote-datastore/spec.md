# Feature Specification: S3-Compatible Remote DataStore Adapter

**Status**: Approved
**Canonical governing ID**: `095-s3-compatible-remote-datastore`
**Extends**: `530-remote-key-value-datastore`, `518-durable-local-datastore`,
`519-embedder-owned-datastore-integration`
**Input**: Issue #892.

## Purpose

Define the provider-specific conformance profile for a host-injected,
S3-compatible remote key-value DataStore adapter. This profile makes object
naming, envelope integrity, conditional writes, and operational failures
portable across S3-compatible services without moving endpoint, credential,
tenant, or retry ownership into Traverse.

## Capability Boundary

The host provisions an S3-compatible client, a stable opaque tenant prefix,
and least-privilege credentials. Traverse uses that injected adapter for a
single key at a time. The adapter is neither a hosted synchronization service
nor a credential store, queue, provider-discovery mechanism, or multi-key
transaction coordinator.

## Requirements

- **FR-001**: The object name MUST be derived only from the host-provisioned
  opaque tenant prefix and an encoded DataStore key. Traverse MUST NOT infer,
  enumerate, log, persist, or expose bucket names, endpoint URLs, tenant
  identities, or provider request identifiers.
- **FR-002**: Every stored object MUST be a canonical serialized DataStore
  value envelope. Its integrity digest MUST be the Traverse SHA-256 digest of
  those canonical bytes. An S3 ETag is a provider concurrency token only and
  MUST NOT be treated as an integrity digest or persisted as content identity.
- **FR-003**: A write or delete with an expected opaque version token MUST use
  the S3-compatible conditional operation supplied by the host adapter. A
  precondition mismatch MUST return `remote_conflict`; it MUST NOT overwrite,
  retry, or silently re-read a conflicting value.
- **FR-004**: A successful write is acknowledged only after the provider
  confirms the conditional operation. A timeout or connection loss after a
  possible submission MUST return `remote_outcome_unknown`; Traverse MUST NOT
  issue a hidden retry.
- **FR-005**: The host owns retry policy. The adapter MUST return stable,
  secret-free evidence that identifies whether failure was before submission,
  an explicit provider outage, a timeout, a denied operation, a conflict, or
  an unknown outcome. It MUST NOT include credentials, values, keys, prefixes,
  endpoints, tenant IDs, or provider text.
- **FR-006**: Credentials MUST be host-provisioned, least-privilege, and
  excluded from DataStore persistence, traces, errors, and conformance
  fixtures. Authorization failure maps to `remote_unauthorized`; inaccessible
  host scope maps to `remote_scope_denied`.
- **FR-007**: Reads MUST re-compute the Traverse SHA-256 digest over canonical
  envelope bytes and fail closed as `remote_integrity_failed` for malformed or
  mismatched content before exposing a value.
- **FR-008**: The stable provider-profile failures are `remote_conflict`,
  `remote_unavailable`, `remote_timeout`, `remote_outcome_unknown`,
  `remote_unauthorized`, `remote_scope_denied`, `remote_integrity_failed`,
  and `remote_backend_failed`. Provider-specific errors MUST be mapped into
  this set.

## Acceptance Scenarios

1. Given two conditional writes using one opaque version token, when the first
   succeeds, then the second returns `remote_conflict` and changes no value.
2. Given an object whose bytes are altered while retaining any provider ETag,
   when it is read, then the adapter returns `remote_integrity_failed`.
3. Given a post-submission timeout, when the operation returns, then it emits
   `remote_outcome_unknown` and performs no retry.
4. Given denied credentials or an inaccessible prefix, when an operation is
   attempted, then it returns the appropriate stable secret-free failure.
5. Given the conformance suite is run against MinIO, when all scenarios pass,
   then it proves only S3-compatible protocol behavior, not provider
   availability or credential provisioning.

## Conformance and Compatibility

The adapter MUST pass a MinIO integration matrix covering conditional
conflict, digest mismatch, authorization denial, outage, timeout, and retry
evidence. The same matrix may be run against a provider, but provider results
are supplementary and do not alter this profile. This draft is additive to
the existing DataStore port and does not change local storage, synchronization,
or public trait semantics.

## Out of Scope

- Multi-key transactions, scans, local write queues, replication, or hosted
  synchronization.
- Credential storage, provider discovery, bucket provisioning, or endpoint
  selection.
- Adding this draft to `approved-specs.json` without explicit maintainer
  approval.

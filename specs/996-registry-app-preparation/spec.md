# Feature Specification: Application Registry Reference Preparation

**Feature Branch**: `codex/issue-1274-verified-registry-preparation`
**Created**: 2026-09-08
**Status**: Approved (2026-09-08)
**Canonical governing ID**: `996-registry-app-preparation`
**Version**: 0.1.0
**Input**: Explicit host-run preparation for application Registry references; validation, activation, and execution remain offline consumers of verified cache evidence.

## Purpose

Enable a host to prepare an application `registry_ref` from a validated synced
Registry index before application validation or activation. The preparation
boundary makes an exact, policy-authorized selection, verifies the published
contract, artifact, and trust evidence, and records immutable non-secret
evidence for offline consumers. It closes the gap between synced Registry
pointers and the offline activation lifecycle without granting network access
to validation, activation, or execution.

## User Scenarios & Testing

### User Story 1 - Prepare an Exact Registry Dependency (Priority: P1)

A deployment operator prepares an application whose manifest declares an exact
Registry reference, so the application can later be validated and activated
without reaching the network.

**Why this priority**: Exact, verified preparation is the prerequisite for a
trusted Registry dependency and all downstream offline behavior.

**Independent Test**: Given a valid signed active release and a policy that
permits its host, preparation creates one verified record for the exact
requested version.

**Acceptance Scenarios**:

1. **Given** a validated Registry index containing active `1.0.1`, **when** an
   operator prepares a reference with `=1.0.1`, **then** the record selects
   `1.0.1` and retains its identity, contract digest, artifact digest, trust
   lifecycle, ABI, target, placement, and redacted resolver evidence.
2. **Given** the index has only `1.0.0`, **when** the operator prepares
   `=1.0.1`, **then** preparation fails with
   `registry_version_range_unsatisfied` and selects no substitute version.

---

### User Story 2 - Enforce Trust and Host Policy (Priority: P1)

A host administrator prevents unapproved sources or invalid releases from
being prepared for an application.

**Why this priority**: Policy and trust checks must happen before bytes enter
the verified cache.

**Independent Test**: A test policy denial, invalid signature, inactive
lifecycle, digest mismatch, ABI mismatch, or target mismatch produces its
corresponding stable redacted result and commits no cache record.

**Acceptance Scenarios**:

1. **Given** a source that the host policy denies, **when** preparation starts,
   **then** it returns `registry_policy_denied` before attempting retrieval.
2. **Given** retrieved bytes or trust evidence that do not verify, **when**
   preparation runs, **then** it fails closed and leaves the prior verified
   cache generation usable and unchanged.

---

### User Story 3 - Validate Offline from Prepared Evidence (Priority: P2)

An application developer validates a prepared application without exposing
hosts, endpoints, credentials, cache paths, or artifact bytes.

**Why this priority**: Offline validation makes the preparation boundary
observable and preserves the activation/execution trust boundary.

**Independent Test**: After preparation, validation succeeds with network
access disabled; missing or altered evidence fails with a stable error and no
network request.

**Acceptance Scenarios**:

1. **Given** an immutable verified record matching the manifest reference,
   **when** application validation runs offline, **then** it accepts the
   Registry component using that record only.
2. **Given** a missing or altered record, **when** validation runs offline,
   **then** it fails closed with a secret-free cache or drift outcome and does
   not fetch or re-resolve a dependency.

### Edge Cases

- An exact range that has no identical active version must never resolve a
  nearby, newer, older, deprecated, inactive, draft, or yanked release.
- A cache key already holding different bytes must reject the write and retain
  the original verified bytes.
- A failure after any retrieval step must not publish a partial generation.
- Empty placement permissions are interpreted only according to the existing
  Registry compatibility rules; preparation must not invent a target fallback.

## Requirements

### Functional Requirements

- **FR-001**: The system MUST provide an explicit host-run preparation
  capability for application `registry_ref` dependencies. Validation,
  activation, and execution MUST NOT retrieve, refresh, or substitute Registry
  content.
- **FR-002**: Preparation MUST accept a validated synced Registry index,
  requested reference, host network/host/permission policy, requested target
  and placement, and a host-owned verified-cache writer.
- **FR-003**: Preparation MUST select only an active exact matching version
  for an exact range and MUST report `registry_version_range_unsatisfied`
  without fallback when no such version exists.
- **FR-004**: Before retrieving either contract or artifact bytes, preparation
  MUST enforce configured network, host, and permission policy and return
  `registry_policy_denied` on denial.
- **FR-005**: Preparation MUST verify contract identity and digest, artifact
  digest, signature evidence, lifecycle, supported ABI, permitted target,
  placement, and declared constraints before making an entry available for
  offline consumption.
- **FR-006**: Preparation MUST return the documented stable, secret-free code
  for each first failing boundary: `registry_index_selection_failed`,
  `registry_version_range_unsatisfied`, `registry_lifecycle_rejected`,
  `registry_policy_denied`, `registry_contract_unreachable`,
  `registry_contract_digest_mismatch`, `registry_artifact_unreachable`,
  `registry_artifact_digest_mismatch`, `registry_signature_unverified`,
  `registry_abi_incompatible`, `registry_target_incompatible`, or
  `registry_cache_commit_failed`.
- **FR-007**: A successful preparation MUST persist immutable digest-keyed
  evidence containing namespace, capability identifier, requested range,
  selected version, contract digest, artifact digest, trust lifecycle, ABI,
  target, placement, constraints, and non-secret resolver evidence.
- **FR-008**: A cache writer MUST reject bytes that differ from an existing
  record under the same digest key and MUST not overwrite that record.
- **FR-009**: Public outcomes and persisted evidence MUST NOT include URLs,
  endpoints, credentials, authorization headers, host-private cache paths, or
  contract/artifact bytes.
- **FR-010**: The Registry contract MUST expose the preparation result and
  failure taxonomy as a versioned public surface consumable by Traverse without
  CLI-specific reinterpretation.
- **FR-011**: Existing local-component validation and registration behavior
  MUST remain unchanged.

### Key Entities

- **Preparation request**: A host-authorized request naming the index snapshot,
  Registry reference, policy, target, placement, and cache owner.
- **Verified preparation record**: Immutable evidence tying one selected
  Registry release and verified bytes to a digest-keyed cache entry.
- **Preparation policy**: The host's non-secret authorization decision for
  retrieval and placement.
- **Preparation outcome**: A redacted success or first-failure result with a
  stable code and permitted evidence fields.

## Success Criteria

### Measurable Outcomes

- **SC-001**: All eleven diagnosed Registry resolution boundaries plus policy
  denial produce one distinct documented stable outcome in automated tests.
- **SC-002**: An exact `=1.0.1` reference succeeds only when `1.0.1` is active,
  signed, policy-authorized, and fully verified; tests show zero alternate
  version selections.
- **SC-003**: Offline validation, activation, and execution tests make zero
  network requests after successful preparation.
- **SC-004**: Tests prove an attempted conflicting write leaves the original
  digest-keyed record byte-identical and usable.
- **SC-005**: Success and failure evidence tests contain none of the prohibited
  private values: URLs, endpoints, credentials, headers, local paths, or raw
  bytes.

## Compatibility and Scope

This specification extends the explicit-host preparation direction of
`125-synced-public-registry-preparation` and the offline lifecycle of
`1258-offline-cache-activation`. It does not change Registry index identity,
allow manifest rewriting, grant activation/execution network access, or alter
local component behavior. It requires a compatible versioned Registry public
contract before Traverse implementation can merge.

## Assumptions

- Hosts own network policy, credential storage, endpoint configuration, cache
  storage, and user authorization; none are surfaced in public evidence.
- Existing signed Registry evidence remains the source of trust assertions.
- The diagnosed error taxonomy in the checked-in Registry-resolution fixture
  is the baseline; this specification adds the explicit policy-denial boundary.

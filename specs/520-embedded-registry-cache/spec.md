# Feature Specification: Embedded Registry Cache

**Feature Branch**: `520-embedded-registry-cache`  
**Created**: 2026-07-28  
**Status**: Draft  
**Input**: Define the embedded-host `registry_ref` dependency-resolution contract.

## User Scenarios & Testing

### User Story 1 - Run a prepared registry-only app offline (Priority: P1)

As a product-app host, I want to load an app whose components use `registry_ref`
after an explicit preparation step, so that execution works without a CLI
sidecar or live network access.

**Independent Test**: Prepare a bundle from a synced registry snapshot, disable
network access, then execute it through Rust and Web embedder conformance hosts.

**Acceptance Scenarios**:

1. **Given** a verified prepared dependency set, **When** an embedded host loads
   a registry-only bundle, **Then** it selects the declared dependency and runs
   without network, CLI, or manifest rewriting.
2. **Given** a bundle is not prepared, **When** the host loads it, **Then** it
   fails with an actionable stable missing-preparation error.

### User Story 2 - Diagnose invalid or unavailable dependencies (Priority: P2)

As an app developer, I want deterministic dependency failures so that I can
repair preparation rather than debug host-specific behavior.

**Acceptance Scenarios**:

1. **Given** no version matches a reference, **When** preparation runs, **Then**
   it reports a stable no-matching-version error.
2. **Given** a selected artifact fails digest verification or is yanked, **When**
   preparation runs, **Then** it fails before execution with a stable error.

### Edge Cases

- A stale snapshot remains usable only when every selected record and artifact
  is still present and verified; it must never trigger background network I/O.
- A prepared cache owned by one host/workspace must not be silently reused by
  another identity or classification boundary.

## Requirements

### Functional Requirements

- **FR-001**: A preparation phase MUST consume an explicitly synced registry
  snapshot and produce a traceable selected dependency set.
- **FR-002**: Selection MUST record namespace, id, version, index identity,
  artifact digest, and deprecation state for every dependency.
- **FR-003**: Preparation MUST verify each selected artifact before it becomes
  available to an embedded host.
- **FR-004**: Embedded execution MUST use only prepared local data and MUST NOT
  fetch from a network, invoke `traverse-cli serve`, or rewrite `registry_ref`.
- **FR-005**: Cache ownership, lifecycle, replacement, and invalidation MUST be
  explicit and scoped to the host/workspace that prepared it.
- **FR-006**: Stable errors MUST distinguish missing preparation, no matching
  version, yanked-only range, failed preparation, and digest mismatch.
- **FR-007**: Rust and Web conformance scenarios MUST prove the same offline
  success and failure behavior.
- **FR-008**: The contract MUST document ownership: Registry publishes records,
  Traverse prepares/resolves, and App-References consumes prepared bundles.

### Key Entities

- **Synced Registry Snapshot**: locally obtained index state with source identity.
- **Prepared Dependency Set**: host-owned immutable selection and verification
  evidence for one bundle.
- **Verified Artifact Cache**: host/workspace-scoped local artifact material.

## Success Criteria

- **SC-001**: Both reference embedders execute a prepared registry-only bundle
  with zero runtime network requests.
- **SC-002**: All five required dependency failures produce the same stable code
  across both reference embedders.
- **SC-003**: Every executed registry dependency is traceable to one snapshot,
  version, and verified digest.

## Assumptions

- The existing Registry sync remains the only source of public registry state.
- This specification succeeds Specs 054/055/068 where their current contracts
  do not define embedded preparation; it preserves Specs 057/068 no-sidecar
  production requirements.
- Hosted registry APIs, remote runtime hosts, and implementation of the resolver
  are out of scope.

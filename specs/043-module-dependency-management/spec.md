# Feature Specification: Module Dependency Management

**Feature Branch**: `043-module-dependency-management`
**Created**: 2026-04-19
**Status**: Draft
**Input**: Spec slice defining how capabilities declare, resolve, lock, and verify inter-capability dependencies within Traverse's registry and contracts model. Covers dependency declaration in contracts, registration-time resolution using semver range matching, dependency lock records, execution-time digest verification, transitive resolution up to depth 5, circular dependency detection, and workspace-scoped resolution. Unblocks GitHub issue #338.

## Purpose

This spec defines the first implementation-governing slice for inter-capability dependency management in Traverse.

It narrows the broad dependency concept into a concrete, testable model for:

- declaring dependencies in a capability contract using a `dependencies` field
- resolving declared dependencies against the workspace registry at registration time using semver range rules from spec 037
- locking resolved dependencies as immutable `(id, version, digest)` tuples tied to the registering capability's version
- verifying locked dependency digests at execution time to detect silent supply-chain drift
- detecting and rejecting circular dependency chains at registration time
- resolving transitive dependencies up to a fixed depth limit

This slice does not define shared-workspace dependency federation, dependency updates/upgrades, or dependency graph visualization. It is intentionally limited to workspace-scoped, static resolution so the lock and verification model can be built and tested cleanly before more complex scenarios are added.

## User Scenarios and Testing

### User Story 1 — Capability Resolves a Satisfiable Dependency at Registration (Priority: P1)

As a capability author, I want to declare `{"capability_id": "traverse.logging", "version_range": "^1.0.0"}` in my contract's `dependencies` field so that the registry resolves it to the installed version (e.g. `1.2.0`), locks the digest, and my capability can execute with full dependency availability.

**Why this priority**: Dependency resolution at registration time is the foundational operation; all other dependency behaviors build on it.

**Independent Test**: Register `traverse.logging` at version `1.2.0`. Then register a capability with a `dependencies` entry of `traverse.logging ^1.0.0`. Verify the registration succeeds, the returned dependency lock contains `{id: "traverse.logging", version: "1.2.0", digest: <expected>}`, and subsequent execution succeeds.

**Acceptance Scenarios**:

1. **Given** `traverse.logging` registered at `1.2.0` and a capability declaring `traverse.logging ^1.0.0`, **When** the capability is registered, **Then** the registry resolves `^1.0.0` to `1.2.0`, creates an immutable dependency lock record, and the registration succeeds.
2. **Given** a successfully registered capability with a locked dependency, **When** the capability is executed, **Then** the runtime verifies the locked dependency digest matches the currently registered digest of `traverse.logging 1.2.0` and proceeds with execution.
3. **Given** a capability contract where the `dependencies` field is absent, **When** the capability is registered, **Then** the registry treats it as having an empty dependency list and registers without error.

### User Story 2 — Registration Fails for an Unsatisfiable Dependency (Priority: P1)

As a capability author, I want a clear error when I declare `traverse.logging ^3.0.0` but the workspace only has `2.1.0` registered — so that I know immediately at registration time that my dependency cannot be satisfied, rather than discovering the problem at execution time.

**Why this priority**: Early failure at registration time is the only safe model; silent registration of unsatisfiable dependencies would produce non-deterministic execution failures.

**Independent Test**: Register `traverse.logging` at version `2.1.0`. Attempt to register a capability with `traverse.logging ^3.0.0`. Verify the registration fails with `dependency_unsatisfiable`, the error body identifies the unsatisfied dependency, and the capability is not stored.

**Acceptance Scenarios**:

1. **Given** `traverse.logging` registered only at `2.1.0`, **When** a capability declaring `traverse.logging ^3.0.0` is registered, **Then** the registry rejects it with `dependency_unsatisfiable` and identifies the unresolved dependency in the error body.
2. **Given** a `dependency_unsatisfiable` failure, **When** the error is inspected, **Then** it contains the `capability_id`, the declared `version_range`, and the highest available version that did not satisfy the range.
3. **Given** a capability with two dependencies where one is satisfiable and one is not, **When** the capability is registered, **Then** the registry fails with `dependency_unsatisfiable` and reports all unsatisfied dependencies, not just the first.

### User Story 3 — Execution Fails When a Locked Dependency Digest Changes (Priority: P2)

As a platform security engineer, I want execution of a capability to fail with `dependency_digest_changed` when the digest of a locked dependency has changed since registration — even if the version string is the same — so that Traverse detects silent supply-chain drift or tampering.

**Why this priority**: Digest verification at execution time is the tamper-detection layer; without it, a re-registered dependency with different content would silently replace the locked version.

**Independent Test**: Register `traverse.logging 1.2.0`, register a dependent capability (locking the digest), re-register `traverse.logging 1.2.0` with different content (new digest), then attempt to execute the dependent capability and verify it fails with `dependency_digest_changed` identifying the affected dependency.

**Acceptance Scenarios**:

1. **Given** a capability with a locked dependency on `traverse.logging 1.2.0` (digest D1), **When** `traverse.logging 1.2.0` is re-registered with a different content (digest D2), **Then** the next execution of the dependent capability fails with `dependency_digest_changed`.
2. **Given** a `dependency_digest_changed` failure, **When** the error is inspected, **Then** it contains the dependency `capability_id`, `version`, the locked digest, and the current digest.
3. **Given** a capability with a locked dependency whose digest has not changed, **When** the capability is executed, **Then** the digest verification step passes silently and execution proceeds normally.

### User Story 4 — Circular Dependency Chain Rejected at Registration (Priority: P2)

As a registry operator, I want the registry to detect and reject circular dependency chains (`A → B → A`) at registration time so that dependency resolution can never enter an infinite loop and lock records remain acyclic.

**Why this priority**: Cyclic dependency graphs make resolution non-terminating and lock records meaningless; they must be rejected before entering the registry.

**Independent Test**: Register capability A with a dependency on B, register B with a dependency on A. Verify B's registration fails with `circular_dependency_detected` identifying the cycle path `B → A → B`.

**Acceptance Scenarios**:

1. **Given** capability A registered with a dependency on B, **When** B is registered with a dependency on A, **Then** the registry detects the cycle and rejects B's registration with `circular_dependency_detected` and the cycle path in the error body.
2. **Given** a three-node cycle `A → B → C → A`, **When** C is registered, **Then** the registry rejects it with `circular_dependency_detected` and the full cycle path.
3. **Given** a valid DAG dependency chain `A → B → C` with no cycles, **When** all three capabilities are registered, **Then** all registrations succeed and all three lock records are created.

## Edge Cases

- Transitive dependency depth exceeds 5 levels — fail at registration with `max_transitive_depth_exceeded`; the error MUST identify the depth at which the limit was reached and the chain up to that point.
- `dependencies` field absent from contract — treat as an empty dependency list; registration proceeds without dependency resolution.
- Dependency lock record corrupted or missing at execution time — fail execution with `dependency_lock_invalid` rather than re-resolving silently; silent re-resolution could mask supply-chain drift.
- Same capability version registered in two workspaces at different digests — workspace isolation means each workspace's lock record is independent; no cross-workspace lock comparison is performed.
- A dependency satisfies the version range but the resolved capability has itself been deprecated — `deprecated_dependency` warning recorded in the registration result; registration is not blocked by deprecation in this slice.
- Two capabilities in the same workspace declare the same transitive dependency at different version ranges — resolve each independently; if the resolved versions differ, each capability's lock record captures its own resolved version.
- A dependency's `version_range` field is an invalid semver expression — fail at registration with `invalid_version_range` and identify the malformed expression.
- A capability is registered with a `dependencies` entry that references itself — `circular_dependency_detected` at registration time; self-referential dependencies are a degenerate cycle.

## Functional Requirements

- **FR-001**: The capability contract schema MUST be extended with an optional `dependencies` field containing an array of `{capability_id: string, version_range: string}` objects.
- **FR-002**: The registry MUST validate each entry in the `dependencies` array at registration time; `capability_id` MUST be a non-empty string and `version_range` MUST be a valid semver range expression.
- **FR-003**: If the `dependencies` field is absent or an empty array, the registry MUST treat the capability as having no dependencies and complete registration without dependency resolution.
- **FR-004**: For each declared dependency, the registry MUST perform semver range resolution against the workspace's registered capabilities using the rules defined in spec 037.
- **FR-005**: Resolution MUST select the highest matching version within the declared range; if no registered version satisfies the range, registration MUST fail with `dependency_unsatisfiable`.
- **FR-006**: `dependency_unsatisfiable` errors MUST identify the `capability_id`, declared `version_range`, and the highest available version that did not satisfy the range; all unsatisfied dependencies MUST be reported in one error, not one at a time.
- **FR-007**: The registry MUST resolve transitive dependencies recursively up to a maximum depth of 5 levels; exceeding depth 5 MUST fail registration with `max_transitive_depth_exceeded` identifying the chain depth and path.
- **FR-008**: The registry MUST detect directed cycles in the dependency graph during transitive resolution; a cycle MUST fail registration with `circular_dependency_detected` and the full cycle path in the error body.
- **FR-009**: On successful resolution of all declared and transitive dependencies, the registry MUST write an immutable `dependency_lock` record for the registering capability containing the resolved `(capability_id, version, digest)` tuple for each dependency.
- **FR-010**: The `dependency_lock` record MUST be tied to the registering capability's version and digest; a new capability version registration creates a new lock record independently.
- **FR-011**: The `dependency_lock` record MUST be immutable after creation; it MUST NOT be modified when a dependency is re-registered at a different digest.
- **FR-012**: At execution time, the runtime MUST read the `dependency_lock` record for the executing capability and verify that the current registered digest of each locked dependency matches the digest captured in the lock.
- **FR-013**: If any locked dependency's current digest differs from the lock record's digest, execution MUST fail with `dependency_digest_changed` identifying the `capability_id`, `version`, locked digest, and current digest.
- **FR-014**: If the `dependency_lock` record is missing or unreadable at execution time, execution MUST fail with `dependency_lock_invalid`; silent re-resolution is not permitted.
- **FR-015**: Dependency resolution MUST be workspace-scoped; the registry MUST NOT resolve dependencies against capabilities registered in other workspaces unless those workspaces are explicitly shared (shared workspace resolution is out of scope for this slice).
- **FR-016**: A `version_range` that is not a valid semver range expression MUST cause registration failure with `invalid_version_range` identifying the malformed expression.
- **FR-017**: A self-referential dependency (a capability declaring itself as a dependency) MUST be detected and rejected as `circular_dependency_detected`.
- **FR-018**: The `dependency_lock` record MUST be included in the registration response so that the caller can inspect the resolved versions and digests.

## Non-Functional Requirements

- **NFR-001 Determinism**: Dependency resolution, version range matching, transitive graph traversal, and lock record creation MUST be deterministic for the same workspace state and contract input.
- **NFR-002 Atomicity**: Dependency resolution and lock record creation MUST be atomic with the capability registration; partial registrations where the capability is stored but the lock is missing MUST not occur.
- **NFR-003 Explainability**: All dependency-related registration failures (`dependency_unsatisfiable`, `circular_dependency_detected`, `max_transitive_depth_exceeded`, `invalid_version_range`) MUST include structured error bodies with enough detail to identify and fix the problem without inspecting internal registry state.
- **NFR-004 Testability**: Semver range resolution, cycle detection, depth limiting, and digest verification MUST be independently testable at unit level without a running registry server.
- **NFR-005 Immutability**: Lock records MUST be immutable after creation; no mutation path may exist even under concurrent re-registration of dependencies.
- **NFR-006 Workspace Isolation**: Dependency resolution MUST not leak across workspace boundaries; a capability in workspace A MUST NOT resolve dependencies from workspace B without explicit shared-workspace configuration (out of scope for this slice).
- **NFR-007 Performance**: Transitive resolution up to depth 5 MUST complete within the same registration latency budget as a non-dependency registration; resolution MUST NOT perform unbounded registry scans.

## Non-Negotiable Quality Standards

- **QG-001**: Dependency resolution MUST fail at registration time for all invalid inputs (unsatisfiable range, cycle, depth exceeded, invalid semver); deferred detection at execution time is not acceptable.
- **QG-002**: Digest verification MUST run on every execution of a capability with a non-empty lock record; it MUST NOT be skippable via configuration or request parameters.
- **QG-003**: A missing or unreadable lock record at execution time MUST cause execution failure with `dependency_lock_invalid`; silent re-resolution is never acceptable.
- **QG-004**: Core dependency resolution logic (range matching, cycle detection, depth limiting, lock creation) MUST reach 100% automated line coverage under the protected coverage gate.
- **QG-005**: Lock records MUST be immutable after creation; any code path that mutates an existing lock record is a blocking defect.

## Key Entities

- **Dependency Declaration**: A `{capability_id, version_range}` object in a capability contract's `dependencies` field. Declares that the capability requires another capability within the given semver range.
- **Dependency Lock Record**: The immutable registry artifact created at registration time containing the resolved `(capability_id, version, digest)` tuple for each declared and transitively resolved dependency. Tied to the registering capability's version and digest.
- **Resolved Dependency**: A specific `(capability_id, version, digest)` triple produced by matching a dependency declaration's version range against the workspace registry.
- **Transitive Dependency**: A dependency of a dependency, resolved recursively up to the configured maximum depth.
- **Dependency Digest Verification**: The execution-time check that compares each locked dependency's digest against the currently registered digest of that dependency version. Fails execution on mismatch.
- **Semver Range Resolution**: The process of selecting the highest registered version of a `capability_id` that satisfies the declared `version_range`, using the semver rules from spec 037.

## Success Criteria

- **SC-001**: A capability declaring a satisfiable dependency is registered with a populated lock record containing the resolved version and digest; execution succeeds.
- **SC-002**: A capability declaring an unsatisfiable dependency fails registration with `dependency_unsatisfiable` identifying all unresolved dependencies; the capability is not stored.
- **SC-003**: A capability whose locked dependency has been re-registered at a different digest fails execution with `dependency_digest_changed`; execution does not proceed to capability code.
- **SC-004**: A circular dependency chain is detected at registration time and rejected with `circular_dependency_detected` and the cycle path; no partial lock records are created.
- **SC-005**: A transitive dependency chain exceeding 5 levels fails with `max_transitive_depth_exceeded`; registration is rejected.
- **SC-006**: Core dependency resolution logic reaches 100% automated line coverage under the protected coverage gate.

## Out of Scope

- Cross-workspace dependency resolution or federation
- Dependency upgrade or update commands
- Dependency graph visualization or reporting tools
- Optional dependencies or conditional dependency activation
- Dependency conflict resolution when two transitive chains require incompatible versions of the same capability
- Dependency removal or capability deregistration cascading
- Shared-workspace dependency resolution configuration
- Dependency pinning or lockfile export for external tooling
- Runtime dependency injection or lazy loading of dependency capabilities

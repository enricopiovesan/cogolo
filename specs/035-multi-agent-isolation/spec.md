# Feature Specification: Multi-Agent Workspace Isolation

**Feature Branch**: `035-multi-agent-isolation`
**Created**: 2026-04-19
**Status**: Draft
**Input**: Isolation model for multi-agent environments in Traverse, covering workspace-keyed registry scoping, subject-based authorization, shared workspace opt-in, and admin operations. Unblocks GitHub issue #303.

## Purpose

This spec defines the workspace isolation model for Traverse runtime and registry operations.

Without isolation, any agent registering capabilities or events into the global registry can observe and interfere with the registrations of every other agent. This is unacceptable when Traverse hosts concurrent agents for distinct principals (CI pipelines, separate teams, different environments).

The isolation model introduced here:

- scopes every registry operation to a `workspace_id`
- enforces that `subject_id` (derived from the JWT per spec 030) may only access workspaces it owns or has been explicitly granted access to
- allows teams to opt in to a shared workspace where multiple subjects collaborate
- reserves a `system` workspace for privileged admin operations
- preserves the existing registry and runtime API shapes while adding workspace scope as a required dimension

This spec does **not** address distributed cross-process isolation, federated workspace directories, or quota enforcement. Those are future concerns.

## User Scenarios and Testing

### User Story 1 — Two Agents Cannot Access Each Other's Capabilities (Priority: P1)

As a platform operator, I want two agents running under different subject identities to have completely separate views of the registry so that one agent cannot discover, read, or execute the other's registered capabilities.

**Why this priority**: Workspace isolation is the foundational safety property of multi-agent Traverse. All other isolation features depend on it being correct.

**Independent Test**: Register two agents under distinct subjects. Each registers a capability with the same capability id. Verify that a runtime request from agent A resolves only agent A's registration and that agent B's registration is invisible to agent A's resolution path.

**Acceptance Scenarios**:

1. **Given** two agents registered under distinct `subject_id` values, each with their own workspace, **When** agent A submits a runtime request for a capability id that both have registered, **Then** the runtime resolves only the registration in agent A's workspace and never evaluates agent B's registration.
2. **Given** agent A attempts to list, read, or execute capabilities in a workspace owned by agent B, **When** the authorization check runs, **Then** the runtime rejects the operation with an `unauthorized_workspace` error before any registry lookup occurs.
3. **Given** agent B's workspace is deleted, **When** agent A submits a request, **Then** agent A's runtime behavior is entirely unaffected.

### User Story 2 — CI Pipeline Isolation (Priority: P1)

As a CI engineer, I want pipeline runs to register capabilities under ephemeral workspace identifiers so that test registrations cannot pollute production workspaces or interfere with each other across builds.

**Why this priority**: CI is the primary driver of ephemeral, high-volume capability registration. Without workspace isolation, CI registrations are a correctness hazard for production agents.

**Independent Test**: Register capabilities under workspace `ci.build-001` and workspace `production`. Verify that a runtime request scoped to `production` never resolves anything registered under `ci.build-001`.

**Acceptance Scenarios**:

1. **Given** a CI pipeline that registers capabilities under workspace `ci.build-NNN`, **When** a runtime request is scoped to workspace `production`, **Then** the registry returns no results from `ci.build-NNN` registrations.
2. **Given** two concurrent CI builds using `ci.build-001` and `ci.build-002`, **When** both register a capability with the same id, **Then** each build's runtime resolves its own registration without conflict.
3. **Given** the CI workspace `ci.build-001` expires or is deleted, **When** the production workspace is queried, **Then** no references to `ci.build-001` registrations are visible.

### User Story 3 — Shared Workspace for Team Collaboration (Priority: P2)

As a team lead, I want to declare a workspace as shared so that multiple authenticated agents within my team can register and execute capabilities in the same namespace.

**Why this priority**: Some legitimate multi-agent workflows require collaboration within one registry namespace. Shared workspaces must be an explicit opt-in, not a default.

**Independent Test**: Create a shared workspace. Register two different subjects as members. Verify each subject can register and execute capabilities in the workspace and that a third non-member subject is rejected.

**Acceptance Scenarios**:

1. **Given** a workspace declared `shared: true` with two member subjects, **When** either subject registers a capability in that workspace, **Then** both subjects can discover and execute it via runtime requests scoped to that workspace.
2. **Given** a shared workspace with two members, **When** a third subject not in the member list submits a request scoped to the shared workspace, **Then** the runtime rejects the operation with `unauthorized_workspace`.
3. **Given** a shared workspace, **When** one member deregisters a capability, **Then** the other member immediately loses the ability to resolve that capability from the workspace.

### User Story 4 — Admin Workspace for System Operations (Priority: P2)

As a platform administrator, I want to use a privileged token to list all workspaces and inspect their metadata via the reserved `system` workspace so that I can audit multi-agent usage without needing individual workspace access.

**Why this priority**: Administrative visibility is required for operations, debugging, and compliance without compromising per-workspace isolation.

**Independent Test**: Issue a request scoped to `system` with a JWT carrying a privileged role claim. Verify the response lists all active workspaces. Issue the same request with a non-privileged token and verify rejection.

**Acceptance Scenarios**:

1. **Given** a JWT with a privileged role claim, **When** the caller submits a request scoped to workspace `system`, **Then** the runtime returns a list of all active workspaces with their metadata.
2. **Given** a JWT without a privileged role claim, **When** the caller attempts any operation scoped to workspace `system`, **Then** the runtime rejects the request with `insufficient_privileges`.
3. **Given** a new workspace is created by an agent, **When** an admin lists workspaces via the `system` scope, **Then** the new workspace appears in the listing with accurate metadata.

## Edge Cases

- `subject_id` derived from an expired JWT — the runtime MUST reject the request without consulting cached identity state; no stale identity may be used for authorization.
- `workspace_id` that is an empty string or contains only whitespace — reject at the validation boundary with a `workspace_id_invalid` error before any registry operation.
- Concurrent creation of the same workspace by two agents — idempotent; workspace creation is keyed on `workspace_id` and the second creation attempt returns the existing workspace without error.
- A legacy-style request that omits `workspace_id` entirely — reject with a `workspace_id_required` migration error containing a pointer to the migration guide; do not silently default to a global scope.
- A subject attempting to create a workspace it already owns — idempotent; return the existing workspace.
- A shared workspace with zero members — treat as inaccessible; no subject can access it until at least one member is added.
- The `system` workspace cannot be deleted, renamed, or declared `shared: true` — any such attempt MUST be rejected with `reserved_workspace`.
- A request that references a workspace id that does not exist — reject with `workspace_not_found` before any registry lookup.

## Functional Requirements

- **FR-001**: Every registry API operation (capability registration, deregistration, lookup, event registration, workflow registration) MUST accept `workspace_id` as a required field.
- **FR-002**: The runtime MUST enforce that `workspace_id` is present and non-empty on every incoming request before executing any business logic; absence MUST produce a `workspace_id_required` error.
- **FR-003**: The runtime MUST validate `workspace_id` is a non-empty, non-whitespace string of at most 128 characters consisting of alphanumeric characters, hyphens, dots, and underscores only.
- **FR-004**: Every registry lookup MUST be scoped to the provided `workspace_id` and MUST NOT return results from any other workspace unless the requesting subject has explicit shared-workspace access.
- **FR-005**: The runtime MUST derive `subject_id` from the validated JWT on every request and MUST NOT accept a caller-supplied `subject_id`.
- **FR-006**: The runtime MUST verify that the `subject_id` derived from the JWT is the owner of the referenced `workspace_id` or is a declared member of a shared workspace before executing any registry operation.
- **FR-007**: Authorization failure MUST produce an `unauthorized_workspace` error and MUST NOT leak any workspace metadata, capability names, or event names from the rejected workspace.
- **FR-008**: Workspace creation MUST be idempotent — submitting a create request for an existing `workspace_id` owned by the same subject MUST return the existing workspace without modifying it.
- **FR-009**: A workspace MUST support a `shared` boolean flag; default value is `false`; a workspace owner MAY set `shared: true` to enable multi-subject access.
- **FR-010**: When `shared: true`, the workspace owner MUST provide an explicit member list of `subject_id` values; the runtime MUST enforce that only listed subjects can access the workspace.
- **FR-011**: Two workspaces MAY register capabilities with the same capability id without conflict; the registry MUST distinguish registrations by `(workspace_id, capability_id)` composite key.
- **FR-012**: The `system` workspace_id MUST be reserved; the runtime MUST reject any attempt to create, delete, rename, or modify the `system` workspace by a non-privileged caller.
- **FR-013**: Operations scoped to the `system` workspace MUST require a JWT carrying a privileged role claim; requests without the privileged claim MUST be rejected with `insufficient_privileges`.
- **FR-014**: The `system` workspace MUST support a `list_workspaces` operation that returns all active workspace metadata (ids, owners, shared flag, member count, creation timestamp).
- **FR-015**: JWT expiry MUST be re-validated on every request; the runtime MUST NOT use a cached `subject_id` from a previous request once the JWT has expired.
- **FR-016**: Registry state mutations (registration, deregistration, membership changes) within one workspace MUST be immediately visible to all subjects with access to that workspace on subsequent requests.
- **FR-017**: Concurrent creation of the same workspace by two agents MUST be handled atomically; exactly one creation wins and the other receives the existing workspace; neither request MUST see an error.
- **FR-018**: The runtime MUST emit a structured workspace audit event for each workspace creation, deletion, membership change, and authorization failure.
- **FR-019**: Workspace deletion MUST atomically deregister all capabilities, events, and workflows associated with that workspace.
- **FR-020**: The runtime MUST propagate `workspace_id` through all internal execution contexts so that trace artifacts and state events are labelled with the workspace scope.

## Non-Functional Requirements

- **NFR-001 Correctness**: Cross-workspace data leakage MUST be treated as a critical defect; no registry query path may return results from a workspace not matching the resolved scope.
- **NFR-002 Determinism**: Authorization decisions and workspace lookup results MUST be deterministic for the same JWT, workspace state, and request input.
- **NFR-003 Performance**: Workspace authorization and registry scoping MUST add no more than one additional indexed lookup per request compared to the pre-isolation baseline.
- **NFR-004 Testability**: Workspace isolation, authorization enforcement, shared workspace access, and system workspace privilege checks MUST each be independently testable without requiring a running JWT issuer.
- **NFR-005 Auditability**: Every authorization failure and workspace mutation MUST produce a structured, machine-readable audit event suitable for compliance review.
- **NFR-006 Compatibility**: The addition of `workspace_id` MUST be a versioned, semver-disciplined breaking change; legacy clients without `workspace_id` MUST receive a `workspace_id_required` error rather than silent degradation.
- **NFR-007 Maintainability**: Workspace resolution, authorization enforcement, and scoped registry lookup MUST be implemented as clearly separated concerns within `traverse-runtime` and `traverse-registry`.

## Non-Negotiable Quality Standards

- **QG-001**: No registry read or write operation MAY return or modify data from a workspace that the caller does not own or have explicit shared access to; any such cross-workspace access is a blocker defect.
- **QG-002**: JWT expiry MUST be checked on every inbound request; stale identity MUST never be used to authorize a workspace operation.
- **QG-003**: Authorization failures MUST NOT leak workspace metadata, capability names, or event names from the rejected workspace in any error response or audit trail visible to the unauthorized caller.
- **QG-004**: 100% automated line coverage is required for workspace authorization, scoped registry lookup, shared workspace access enforcement, and system workspace privilege checks.
- **QG-005**: Workspace isolation behavior MUST align with this governing spec and fail the spec-alignment CI gate when drift occurs.

## Key Entities

- **Workspace**: A named, subject-owned scope within the Traverse registry. All registry objects (capabilities, events, workflows) belong to exactly one workspace.
- **workspace_id**: The stable string identifier for a workspace. Required on all API calls. Validated for format at the request boundary.
- **subject_id**: The identity of the caller, derived from the validated JWT. The runtime MUST derive this value; callers MUST NOT supply it directly.
- **Workspace Member**: A `subject_id` explicitly granted access to a shared workspace by the workspace owner.
- **Shared Workspace**: A workspace declared `shared: true` with an explicit member list. Multiple subjects may register and execute capabilities within a shared workspace.
- **System Workspace**: The reserved `system` workspace_id. Supports privileged administrative operations. Cannot be created, deleted, or modified by unprivileged callers.
- **Workspace Audit Event**: A structured event emitted on workspace creation, deletion, membership change, or authorization failure.

## Success Criteria

- **SC-001**: Two agents under distinct subject identities cannot observe each other's registry registrations when each operates within its own workspace.
- **SC-002**: A runtime request that omits `workspace_id` is rejected with `workspace_id_required` before any registry lookup occurs.
- **SC-003**: A shared workspace correctly grants access to all declared members and rejects all non-members.
- **SC-004**: The `system` workspace rejects unprivileged callers and returns a correct workspace listing to privileged callers.
- **SC-005**: Concurrent workspace creation with the same `workspace_id` is idempotent and produces no error for either caller.
- **SC-006**: 100% automated line coverage is achieved for all workspace isolation and authorization paths.

## Out of Scope

- Distributed cross-process workspace federation
- Workspace quota enforcement (capability count limits, event volume limits)
- Workspace billing or usage metering
- JWT issuance or key management (handled by the identity layer per spec 030)
- Workspace templates or cloning
- Role-based access control within a workspace (fine-grained per-capability permissions)
- Cross-workspace capability sharing without shared workspace declaration

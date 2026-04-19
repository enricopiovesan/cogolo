# Feature Specification: Programmatic Capability Registration

**Feature Branch**: `034-programmatic-registration`
**Created**: 2026-04-19
**Status**: Draft
**Input**: Runtime capability registration via the HTTP API, covering workspace-scoped persistence, idempotency by contract digest, full contract validation at registration time, and session-ephemeral scope for one-off capabilities.

## Purpose

This spec defines programmatic capability registration for Traverse.

It narrows the broad registry extensibility intent into a concrete, testable model for:

- accepting capability contract submissions via `POST /v1/capabilities/register`
- validating contracts fully at registration time before any persistence
- keying the registry by (workspace_id, capability_id, version) for workspace-scoped isolation
- making registration idempotent for the same contract digest within a workspace
- supporting two registration scopes: `workspace_persisted` (default) and `session_ephemeral`
- rejecting conflicting re-registrations with a deterministic `ImmutableVersionConflict` error

This slice does **not** define capability artifact distribution, remote artifact fetching, or multi-workspace federation. It is intentionally limited to the registration lifecycle so the registry control plane can be built and verified before artifact management is added.

## User Scenarios and Testing

### User Story 1 - Register a Capability at Runtime Without a CLI Command (Priority: P1)

As an agent or automation system, I want to register a capability via `POST /v1/capabilities/register` so that capabilities can be provisioned dynamically without requiring a manual CLI step.

**Why this priority**: Programmatic registration is the primary mechanism for agent-driven capability provisioning; without it, Traverse cannot be used in headless or automated contexts.

**Independent Test**: Send `POST /v1/capabilities/register` with a valid capability contract JSON body including a `workspace_id`; verify the capability is immediately executable via `POST /v1/capabilities/execute` without a CLI command.

**Acceptance Scenarios**:

1. **Given** a valid capability contract JSON body with a `workspace_id`, **When** the client sends `POST /v1/capabilities/register`, **Then** the server validates the contract, persists it under `workspace_persisted` scope, and returns HTTP 201 with the registered capability record.
2. **Given** a successful registration, **When** the client sends `POST /v1/capabilities/execute` with a matching request, **Then** the runtime discovers and executes the newly registered capability.
3. **Given** a capability registered under `workspace_persisted` scope, **When** the server restarts, **Then** the capability is still present in the registry and executable.

### User Story 2 - Idempotent Re-registration With the Same Digest (Priority: P1)

As an agent, I want re-registering the same capability with the same contract digest to succeed silently so that registration logic does not need to track whether a capability was previously registered.

**Why this priority**: Idempotency is required for safe automation; without it, agents must implement their own deduplication, which leaks registry concerns into capability authors.

**Independent Test**: Register a capability; register the same capability a second time with an identical contract digest; verify the server returns HTTP 200 with `already_registered: true` and does not write to persistent storage again.

**Acceptance Scenarios**:

1. **Given** a capability already registered under a given workspace, **When** the same contract digest is submitted for the same capability id and version, **Then** the server returns HTTP 200 with `{"already_registered": true}` and does not modify the existing registry entry.
2. **Given** two concurrent registration requests for the same capability from two agents, **When** both requests complete, **Then** exactly one write occurs and both agents receive a success response (`already_registered: true` or HTTP 201).

### User Story 3 - Session-Ephemeral Registration for One-Off Tasks (Priority: P2)

As an agent, I want to register a capability with `scope: session_ephemeral` so that the capability is cleaned up automatically on server restart without requiring an explicit deregistration call.

**Why this priority**: Ephemeral registrations are important for short-lived automation tasks; persistent storage of one-off capabilities wastes registry space and leaks between sessions.

**Independent Test**: Register a capability with `scope: session_ephemeral`; verify it is executable in the current session; simulate server restart; verify the capability is no longer present in the registry.

**Acceptance Scenarios**:

1. **Given** a capability registered with `scope: session_ephemeral`, **When** the server restarts, **Then** the capability is absent from the registry and cannot be executed.
2. **Given** a session-ephemeral capability registered in the current session, **When** the client executes it, **Then** it runs successfully within the same session.

### User Story 4 - Reject Invalid Contracts Before Touching the Registry (Priority: P2)

As a platform developer, I want invalid capability contracts to be rejected at registration time so that malformed contracts never reach the registry or persistent storage.

**Why this priority**: Pre-persistence validation is a non-negotiable correctness guarantee; a registry containing invalid contracts can cause silent runtime failures.

**Independent Test**: Submit `POST /v1/capabilities/register` with a contract body missing a required field (e.g., `service_type`); verify the server returns HTTP 422 with a structured validation error and no registry write occurs.

**Acceptance Scenarios**:

1. **Given** a contract body missing required fields, **When** the client sends `POST /v1/capabilities/register`, **Then** the server returns HTTP 422 with a structured error listing each violated requirement and does not write to the registry.
2. **Given** a contract with an invalid `artifact_type` value, **When** the client sends the registration, **Then** the server returns HTTP 422 with a machine-readable error code and the offending field path.
3. **Given** a contract where the declared state schema is malformed JSON Schema, **When** the client sends the registration, **Then** the server returns HTTP 422 with a `malformed_state_schema` error before any persistence.

## Edge Cases

- Concurrent registration of the same capability from two agents simultaneously: idempotency MUST hold regardless of request interleaving; no duplicate writes and no conflicting error responses.
- `workspace_id` missing from the request body MUST be rejected with HTTP 400 and a clear `missing_workspace_id` error before any validation proceeds.
- Registration of a capability whose artifact is not yet present locally MUST return a structured `artifact_not_found` error; partial registration MUST NOT be persisted.
- Version conflict: same `capability_id`, same `workspace_id`, different `version`, different digest — this is a valid new registration; same `capability_id`, same `workspace_id`, same `version`, different digest MUST return `ImmutableVersionConflict`.
- Registration with a `workspace_id` that contains invalid characters (e.g., path separators) MUST be rejected before any file system or storage operation.
- Re-registration attempt for a `session_ephemeral` capability after server restart MUST succeed as a fresh registration (the old entry no longer exists).
- Contract body exceeding the HTTP max payload limit (governed by spec 033) MUST be rejected by the HTTP layer before the registry is consulted.

## Functional Requirements

- **FR-001**: The server MUST expose `POST /v1/capabilities/register` accepting a capability contract JSON body and a required `workspace_id` field.
- **FR-002**: Every registration request MUST include a `workspace_id`; requests without `workspace_id` MUST be rejected with HTTP 400 and a `missing_workspace_id` error before any further processing.
- **FR-003**: The registry MUST key each entry by the compound key `(workspace_id, capability_id, version)`; registrations in different workspaces MUST be isolated from each other.
- **FR-004**: The server MUST fully validate the submitted contract before any write to the registry, including schema validation, required field presence, `service_type` validity, and `artifact_type` validity.
- **FR-005**: Contracts that fail validation MUST be rejected with HTTP 422 and a structured error body listing each violated requirement; no partial registry write MUST occur.
- **FR-006**: A malformed `state_schema` field in the contract MUST cause rejection with a `malformed_state_schema` error code before any registry write.
- **FR-007**: When the submitted contract digest matches an existing entry for the same `(workspace_id, capability_id, version)`, the server MUST return HTTP 200 with `{"already_registered": true}` and MUST NOT perform a new write.
- **FR-008**: When a different contract digest is submitted for an existing `(workspace_id, capability_id, version)`, the server MUST return HTTP 409 with an `ImmutableVersionConflict` error and MUST NOT overwrite the existing entry.
- **FR-009**: Concurrent registration requests for the same key MUST be handled with correct mutual exclusion; exactly one write MUST occur and both callers MUST receive a non-error response.
- **FR-010**: The registration request MUST include a `scope` field with valid values `workspace_persisted` (default) or `session_ephemeral`; unknown scope values MUST be rejected with HTTP 422.
- **FR-011**: `workspace_persisted` registrations MUST be written to persistent storage and MUST survive server restarts.
- **FR-012**: `session_ephemeral` registrations MUST be held in-memory only and MUST NOT be written to persistent storage; they MUST be absent from the registry after a server restart.
- **FR-013**: A capability registered via this endpoint MUST be immediately discoverable and executable via `POST /v1/capabilities/execute` without requiring a separate activation step.
- **FR-014**: `GET /v1/capabilities?workspace_id=<id>` MUST return only capabilities registered in the specified workspace; capabilities from other workspaces MUST NOT appear in the response.
- **FR-015**: A `workspace_id` containing path separators, null bytes, or other invalid characters MUST be rejected with HTTP 400 before any storage or file system operation.
- **FR-016**: The registration response for a new HTTP 201 registration MUST include the full registered capability record with its computed digest, scope, and workspace_id.
- **FR-017**: Registration of a capability whose referenced artifact is not present locally MUST return HTTP 422 with an `artifact_not_found` error and MUST NOT persist a registry entry.
- **FR-018**: The contract digest MUST be computed deterministically by the server; callers MUST NOT provide the digest as an input field.
- **FR-019**: The registry MUST support listing capabilities filtered by `workspace_id`; the listing MUST include both `workspace_persisted` and `session_ephemeral` entries for the current session.
- **FR-020**: All registry mutations MUST be atomic from the caller's perspective; a failed registration MUST leave the registry in its pre-request state.

## Non-Functional Requirements

- **NFR-001 Idempotency**: Registration MUST be idempotent for the same `(workspace_id, capability_id, version, digest)` tuple regardless of how many times or how concurrently the request is issued.
- **NFR-002 Atomicity**: Registry writes MUST be atomic; partial writes that leave the registry in an inconsistent state are blocking defects.
- **NFR-003 Isolation**: Registrations in one workspace MUST NOT be visible to registry lookups in another workspace.
- **NFR-004 Testability**: Contract validation, idempotency logic, and scope handling MUST be testable independently of the HTTP transport layer.
- **NFR-005 Determinism**: Contract digest computation MUST be deterministic; the same contract bytes MUST always produce the same digest.
- **NFR-006 Compatibility**: The registration request and response wire format MUST be semver-versioned and stable; breaking changes require a new governing spec version.
- **NFR-007 Fail-Fast Validation**: Contract validation MUST occur fully before any storage operation is initiated; the validation and persistence phases MUST be clearly separated.

## Non-Negotiable Quality Standards

- **QG-001**: An invalid contract MUST NEVER reach the persistent registry; any code path that writes an unvalidated contract is a spec violation.
- **QG-002**: Idempotency MUST hold under concurrent load; a race condition that produces duplicate registry entries or conflicting error responses is a blocking defect.
- **QG-003**: `workspace_id` MUST be validated for illegal characters before any file system or storage operation; an injection via `workspace_id` is a blocking security defect.
- **QG-004**: Core registration logic (validation, idempotency, scope handling, digest computation) MUST reach 100% automated line coverage.
- **QG-005**: The registration wire format MUST be verified against the governing spec before merge; drift from the declared interface is a blocking CI failure.

## Key Entities

- **Registration Request**: The HTTP request body submitted to `POST /v1/capabilities/register`, containing the capability contract, `workspace_id`, and optional `scope`.
- **Contract Digest**: A deterministic hash of the submitted capability contract, computed by the server and used as the idempotency key.
- **Registration Key**: The compound key `(workspace_id, capability_id, version)` used to look up and store registry entries.
- **Registration Scope**: The lifecycle policy for a registry entry — `workspace_persisted` survives restarts; `session_ephemeral` does not.
- **ImmutableVersionConflict**: The structured error returned when a different contract digest is submitted for an already-registered `(workspace_id, capability_id, version)`.
- **Workspace**: A named isolation boundary for capability registrations; each registration belongs to exactly one workspace.
- **Validation Phase**: The pre-persistence step that checks schema, required fields, `service_type`, `artifact_type`, and state schema correctness before any write.
- **Persistent Registry Storage**: The durable backend (filesystem or embedded DB) used for `workspace_persisted` entries.

## Success Criteria

- **SC-001**: An agent registers a capability via `POST /v1/capabilities/register` and immediately executes it via `POST /v1/capabilities/execute` without any CLI command.
- **SC-002**: Re-registering the same capability with the same contract digest returns HTTP 200 with `already_registered: true` regardless of how many times the request is issued.
- **SC-003**: A `session_ephemeral` registration is absent from the registry after a server restart.
- **SC-004**: A registration request with an invalid contract is rejected with HTTP 422 before any registry write occurs.
- **SC-005**: Core registration logic reaches 100% automated line coverage under the protected coverage gate.

## Out of Scope

- Capability deregistration or explicit deletion from the registry
- Remote artifact fetching or distribution at registration time
- Cross-workspace capability sharing or federation
- Capability versioning migration tooling
- Bulk registration from a manifest file
- Webhook or event notification on registration
- Registration of capabilities without a `workspace_id` (anonymous workspace)

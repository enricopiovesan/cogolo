# Feature Specification: Workflow Composition API

**Feature Branch**: `041-workflow-composition-api`
**Created**: 2026-04-19
**Status**: Draft
**Input**: Spec slice defining the runtime and registry surface for registering, inspecting, and executing DAG-based workflows composed of capability nodes — covering JSON document format, HTTP API endpoints, CLI commands, cycle detection, and edge schema validation. Unblocks GitHub issue #309.

## Purpose

This spec defines the first implementation-governing slice for programmatic workflow composition in Traverse.

It narrows the broad workflow concept into a concrete, testable model for:

- accepting a canonical JSON workflow document at an HTTP endpoint and CLI command
- validating the workflow graph for cycles and unresolved capability references at registration time
- locking the workflow as an immutable versioned artifact with a content-addressable digest
- exposing listing and inspection endpoints for registered workflows
- executing a registered workflow as an ordered sequence of capability invocations within the existing runtime execution model

This slice is intentionally limited to workspace-scoped, DAG workflows over already-registered capabilities. Event-driven orchestration, cross-workspace workflow sharing, and dynamic capability discovery during traversal are out of scope.

## User Scenarios and Testing

### User Story 1 — Register and Execute a Two-Step Workflow via HTTP API (Priority: P1)

As an agent developer, I want to build a two-step workflow JSON document that wires capability A's output into capability B's input, register it via `POST /v1/workflows/register`, and execute it in a single request — without writing any files to disk — so that Traverse proves its first dynamic, programmatic workflow path.

**Why this priority**: The canonical use case for workflow composition is agent-side programmatic authoring. Without this path there is no value in the composition API.

**Independent Test**: Construct a valid two-node workflow JSON document in memory, POST it to `/v1/workflows/register`, verify a `201` response with a stable `workflow_id` and digest, then execute the workflow via the runtime request model and verify both capabilities are called in order with correct input/output mapping.

**Acceptance Scenarios**:

1. **Given** two registered capabilities where capability A's output schema is compatible with capability B's input schema, **When** an agent submits a valid workflow JSON document to `POST /v1/workflows/register`, **Then** the server validates the graph, resolves all capability references, and returns `201` with a stable `workflow_id` and content-addressable digest.
2. **Given** a registered workflow, **When** a runtime request targets that workflow by `workflow_id`, **Then** the runtime executes capability A with the supplied input, maps A's output to B's input according to the declared edge mapping, executes capability B, and returns a structured execution result and trace covering both nodes.
3. **Given** a workflow registration that would complete successfully, **When** the same workflow document is submitted a second time with the same `workflow_id` and identical content, **Then** the server returns `200` (idempotent re-registration) with the same digest.

### User Story 2 — Register a File-Based Workflow via CLI (Priority: P1)

As a developer, I want to run `traverse-cli workflow register my-workflow.json` to register a workflow stored in a local file — using the same HTTP API internally — so that file-based and programmatic workflows share one canonical code path.

**Why this priority**: CLI-based registration is the primary developer workflow; it must be consistent with the API surface to avoid divergent behavior.

**Independent Test**: Write a valid two-node workflow JSON document to a local file, invoke `traverse-cli workflow register`, verify it prints the assigned `workflow_id` and digest, then call `traverse-cli workflow inspect <id>` and verify the returned graph matches the submitted document.

**Acceptance Scenarios**:

1. **Given** a valid workflow JSON file on disk, **When** the developer runs `traverse-cli workflow register <path>`, **Then** the CLI reads the file, POSTs to `/v1/workflows/register`, and prints the assigned `workflow_id` and digest.
2. **Given** a workflow file with a validation error (e.g. cycle or unresolved capability), **When** the developer runs the register command, **Then** the CLI prints the server error code and human-readable description and exits non-zero.
3. **Given** a registered workflow, **When** the developer runs `traverse-cli workflow list`, **Then** the CLI prints a table of all workspace-scoped workflows with their ids, versions, and digests.

### User Story 3 — Inspect a Registered Workflow via HTTP API (Priority: P2)

As an agent or developer, I want to call `GET /v1/workflows/{id}` and receive the full graph definition including all nodes, edges, and edge mappings so that I can verify the composition before executing it.

**Why this priority**: Inspection is necessary for debugging and for agents that need to confirm a workflow's structure before trusting its execution.

**Independent Test**: Register a three-node workflow, then GET the workflow by id and verify the response contains all three nodes, all declared edges, all edge input/output mappings, and the registration digest.

**Acceptance Scenarios**:

1. **Given** a registered workflow, **When** an agent calls `GET /v1/workflows/{id}`, **Then** the server returns the full workflow document including nodes, edges, edge mappings, registration metadata, and digest.
2. **Given** a `workflow_id` that does not exist in the workspace, **When** an agent calls `GET /v1/workflows/{id}`, **Then** the server returns a structured `workflow_not_found` error with the requested id.
3. **Given** multiple registered workflows, **When** an agent calls `GET /v1/workflows`, **Then** the server returns a paginated list of all workspace-scoped workflows with summary fields.

### User Story 4 — Reject Cyclic and Invalid Workflow Graphs at Registration (Priority: P2)

As a platform developer, I want the registry to detect and reject workflow graphs containing cycles, unresolved capability references, or edge schema mismatches at registration time — not at execution time — so that invalid workflows never enter the registry.

**Why this priority**: Silent acceptance of invalid graphs would produce non-deterministic runtime failures; early rejection is the only safe model.

**Independent Test**: Submit three invalid workflow documents — one with a direct cycle, one with an unresolved capability reference, and one with an edge connecting an incompatible output schema to an input schema — and verify each returns the correct structured error code.

**Acceptance Scenarios**:

1. **Given** a workflow document where node A declares a directed edge to node B and node B declares a directed edge back to node A, **When** the document is submitted to `POST /v1/workflows/register`, **Then** the server rejects it with `workflow_cycle_detected` and includes the cycle path in the error body.
2. **Given** a workflow document referencing a `capability_id` not present in the workspace registry, **When** the document is submitted to `POST /v1/workflows/register`, **Then** the server rejects it with `unresolved_capability_reference` and identifies the missing capability.
3. **Given** a workflow document where an edge maps node A's `result.count` (integer) to node B's `input.name` (string), **When** the document is submitted, **Then** the server rejects it with `edge_schema_mismatch` and identifies the incompatible field pair.

## Edge Cases

- Workflow references a capability not registered in the workspace — `unresolved_capability_reference` at registration time; the error body MUST identify each unresolved reference.
- Workflow with a single node and no edges — valid; executes as a single-capability invocation with no edge mapping step.
- Input/output type mismatch on an edge (schema incompatibility between connected nodes) — `edge_schema_mismatch` at registration; the error body MUST identify the edge and the incompatible field types.
- Workflow `id` collision where the same `id` is submitted with a different document digest — `ImmutableVersionConflict`; same idempotency rules as capability registration.
- Workflow document with zero nodes — `empty_workflow` validation error at registration; empty graphs are not executable and MUST be rejected before graph analysis.
- Edge mapping references a field not present in the source node's output schema — `edge_field_not_found` at registration.
- Workflow where all declared capabilities are present but one has been deprecated since registration — `deprecated_capability_reference` warning at execution time; behavior governed by workspace deprecation policy.
- Concurrent registration of the same workflow `id` with different digests by two agents — the registry MUST serialize and apply the idempotency rules; one request wins, the other receives `ImmutableVersionConflict`.

## Functional Requirements

- **FR-001**: The registry MUST accept a workflow registration request at `POST /v1/workflows/register` whose body is a canonical JSON workflow document.
- **FR-002**: A workflow document MUST contain a stable `workflow_id`, a `version` string, a `nodes` array, and an `edges` array; all fields are required.
- **FR-003**: Each node in a workflow document MUST reference a `capability_id` and `capability_version` that are registered in the workspace at the time of workflow registration.
- **FR-004**: Each edge in a workflow document MUST declare a `source_node`, `source_output_field`, `target_node`, and `target_input_field`.
- **FR-005**: The registry MUST perform a topological sort of the workflow graph at registration time to detect directed cycles; any cycle MUST cause registration failure with `workflow_cycle_detected` and a cycle path in the error body.
- **FR-006**: The registry MUST validate that every `capability_id` referenced in the workflow `nodes` array is present in the workspace registry at the time of registration; any unresolved reference MUST cause registration failure with `unresolved_capability_reference`.
- **FR-007**: The registry MUST validate that the output schema field referenced by each edge's `source_output_field` is compatible with the input schema field referenced by each edge's `target_input_field`; schema incompatibility MUST cause registration failure with `edge_schema_mismatch`.
- **FR-008**: On successful registration the registry MUST assign a content-addressable digest to the workflow document and store it immutably.
- **FR-009**: Re-registration of a workflow with the same `workflow_id` and identical digest MUST be accepted idempotently and return the existing registration record.
- **FR-010**: Re-registration of a workflow with the same `workflow_id` and a different digest MUST be rejected with `ImmutableVersionConflict`.
- **FR-011**: The registry MUST expose `GET /v1/workflows` returning a paginated list of all workspace-scoped registered workflows with summary fields including `workflow_id`, `version`, `digest`, and `registered_at`.
- **FR-012**: The registry MUST expose `GET /v1/workflows/{id}` returning the full workflow document, all node records, all edge records, edge mappings, and registration metadata for the identified workflow.
- **FR-013**: `GET /v1/workflows/{id}` MUST return a structured `workflow_not_found` error when the `id` does not exist in the workspace.
- **FR-014**: The CLI MUST expose `traverse-cli workflow register <path>` which reads a JSON workflow file and POSTs to `/v1/workflows/register` using the same HTTP client as other CLI commands.
- **FR-015**: The CLI MUST expose `traverse-cli workflow list` which calls `GET /v1/workflows` and renders results in a tabular format.
- **FR-016**: The CLI MUST expose `traverse-cli workflow inspect <id>` which calls `GET /v1/workflows/{id}` and renders the full graph definition.
- **FR-017**: The runtime MUST support targeting a registered workflow by `workflow_id` in a runtime request.
- **FR-018**: When a runtime request targets a workflow, the runtime MUST execute capability nodes in topological order derived from the registered graph.
- **FR-019**: The runtime MUST apply the declared edge field mappings to route each node's output into the subsequent node's input before execution.
- **FR-020**: The runtime MUST validate each node's input against its capability's contract input schema before executing that node.
- **FR-021**: The runtime MUST validate each node's output against its capability's contract output schema after executing that node and before applying edge mappings.
- **FR-022**: The runtime MUST emit state events and produce a structured trace that covers all nodes in the workflow execution, not just the first.
- **FR-023**: Workflow documents MUST be workspace-scoped; a workflow registered in workspace A MUST NOT be visible or executable in workspace B.
- **FR-024**: A workflow document with zero nodes MUST be rejected at registration with `empty_workflow`.

## Non-Functional Requirements

- **NFR-001 Determinism**: Graph traversal order, topological sort, edge mapping application, state event ordering, and trace generation MUST be deterministic for the same workflow graph and input.
- **NFR-002 Explainability**: Registration failures MUST include structured error bodies identifying the exact validation failure, affected node or edge, and — for cycles — the full cycle path.
- **NFR-003 Immutability**: Once a workflow is registered with a given `workflow_id` and digest, its document MUST be immutable; modification requires a new version.
- **NFR-004 Testability**: Cycle detection, edge schema validation, topological sort, and node execution sequencing MUST be independently testable at unit level without a running HTTP server.
- **NFR-005 Compatibility**: The canonical workflow JSON document format MUST be versionable and suitable for semver discipline; breaking changes to the schema require a new spec.
- **NFR-006 Portability**: Workflow execution MUST use the existing runtime placement abstraction so that future remote and browser placements work without changes to the workflow model.
- **NFR-007 Workspace Isolation**: Workflow registration, listing, inspection, and execution MUST be strictly workspace-scoped with no cross-workspace leakage.

## Non-Negotiable Quality Standards

- **QG-001**: Cycle detection MUST run at registration time and MUST NOT be deferred to execution time; cyclic graphs MUST never enter the registry.
- **QG-002**: Every workflow registration validation failure MUST return a structured error code and human-readable message; generic HTTP 500 responses for validation errors are not acceptable.
- **QG-003**: The CLI MUST delegate all business logic to the HTTP API; no standalone validation logic may live only in the CLI.
- **QG-004**: Core workflow validation logic (cycle detection, edge schema validation, capability reference resolution) MUST reach 100% automated line coverage.
- **QG-005**: Workflow execution MUST use the runtime's existing contract validation path for each node; bypassing input/output schema validation for workflow-executed capabilities is not acceptable.

## Key Entities

- **Workflow Document**: The canonical JSON artifact defining a workflow's `workflow_id`, `version`, nodes, and edges. This is a governed artifact type with the same immutability rules as capability contracts.
- **Workflow Node**: A member of a workflow's `nodes` array that references a registered capability by `capability_id` and `capability_version`.
- **Workflow Edge**: A member of a workflow's `edges` array that declares a directed data-flow connection between two nodes, including source and target field mappings.
- **Workflow Registration Record**: The immutable registry entry created on successful registration, containing the workflow document, digest, and registration metadata.
- **Workflow Execution Trace**: The structured trace artifact produced by the runtime for a workflow execution; covers all nodes rather than a single capability.
- **Edge Field Mapping**: The runtime operation that reads a named field from a node's output and writes it to a named field in the subsequent node's input before that node executes.

## Success Criteria

- **SC-001**: A two-node workflow can be registered via `POST /v1/workflows/register` and executed end-to-end with correct input/output field routing and a trace covering both nodes.
- **SC-002**: Registration of a cyclic workflow graph fails at registration time with `workflow_cycle_detected` and a cycle path; the workflow is not stored.
- **SC-003**: Registration of a workflow with an unresolved capability reference fails with `unresolved_capability_reference`; the workflow is not stored.
- **SC-004**: `GET /v1/workflows/{id}` returns the full graph document for a registered workflow; unknown ids return a structured `workflow_not_found` error.
- **SC-005**: `traverse-cli workflow register`, `list`, and `inspect` work correctly against a live server and produce consistent output with the HTTP API.
- **SC-006**: Core workflow validation logic reaches 100% automated line coverage under the protected coverage gate.

## Out of Scope

- Event-driven or reactive workflow triggers
- Cross-workspace workflow sharing or federation
- Workflow versioning beyond the immutable `workflow_id`+digest model
- Conditional branching, loops, or fan-out/fan-in within a workflow graph
- Dynamic capability discovery during workflow traversal
- Workflow rollback or compensating transactions
- Remote or distributed placement for workflow node execution
- Workflow scheduling or deferred execution

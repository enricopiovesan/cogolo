# Feature Specification: Contractual Enforcement Gate

**Feature Branch**: `040-contractual-enforcement-gate`
**Created**: 2026-04-19
**Status**: Draft
**Input**: Three-layer contractual validation enforced at authoring/CI-time, registration-time, and execution-time across all governed artifact directories. Introduces the draft quarantine convention, a unified violation taxonomy, and aggregate (non-fail-fast) violation reporting. Governs `scripts/ci/`, `crates/traverse-runtime/`, `crates/traverse-contracts/`, and `specs/governance/`. Unblocks GitHub issue #332.

## Purpose

This spec defines the Contractual Enforcement Gate — a three-layer validation architecture that prevents invalid, incomplete, or quarantine-violating artifacts from entering any governed execution path in Traverse.

The three enforcement layers are:

1. **Authoring/CI-time gate**: Runs in CI against all governed artifact directories (`contracts/`, `workflows/`, `specs/`, `crates/`). Fails for any validation error anywhere in those directories. No partial-pass mode.
2. **Registration-time gate**: Runs when a capability, event, workflow, or connector is submitted to the registry. Validates the same rules as CI plus referential integrity checks (connector contracts, event type registry).
3. **Execution-time gate**: Runs during capability execution. Validates input and output schemas against the capability contract (as established by spec 006) and additionally verifies that emitted events match the declared event catalog in the capability contract.

The draft quarantine convention establishes that experimental artifacts MUST live under `drafts/` (top-level). Artifacts in `drafts/` are never executable, never referenced by production workflows or governed specs, and explicitly rejected by the runtime if an execution path attempts to reference them.

All validation failures carry a unified violation record: `violation_code` (stable string), `path` (artifact path), and `message` (human-readable). All violations are collected and reported together; no fail-fast-on-first behavior is permitted.

## User Scenarios and Testing

### User Story 1 — CI Fails on Malformed Contract (Priority: P1)

As a developer, I want CI to fail with a specific `violation_code` pointing to the exact field and file when I submit a PR with a malformed contract JSON so that I can fix the problem without guessing which artifact is invalid.

**Why this priority**: Contract validation in CI is the outermost and earliest enforcement point; every developer encounters it before any other gate.

**Independent Test**: Introduce a contract JSON missing a required field (`service_type`). Submit through the CI gate script. Verify the script exits non-zero and the output includes a violation record with `violation_code`, `path`, and `message` pointing to the missing field.

**Acceptance Scenarios**:

1. **Given** a PR that adds a contract JSON missing the required `service_type` field, **When** the CI gate runs, **Then** it exits non-zero and emits a violation record with `violation_code: "missing_required_field"`, the artifact `path`, and a `message` identifying the missing field.
2. **Given** a PR with multiple invalid contracts in different files, **When** the CI gate runs, **Then** it collects all violations across all files and reports them in a single structured output before exiting non-zero.
3. **Given** a PR where all contracts are valid, **When** the CI gate runs, **Then** it exits 0 with no violation records emitted.

### User Story 2 — Registration Rejects Missing Required Field (Priority: P1)

As an agent or developer, I want the Traverse registry to reject a capability registration that is missing `service_type` so that invalid capabilities never enter the registry, even if CI was bypassed.

**Why this priority**: Registration-time validation is the second line of defense; it must be independent of CI so that programmatic registrations are also validated.

**Independent Test**: Submit a capability contract missing `service_type` directly to the registry API. Verify the response carries a validation error with `violation_code`, `path`, and `message` before any registry state is modified.

**Acceptance Scenarios**:

1. **Given** a capability contract submitted for registration that is missing `service_type`, **When** the registry processes the submission, **Then** it returns a validation error with `violation_code: "missing_required_field"` and does not write any registry state.
2. **Given** a capability contract with all required fields present but referencing a connector not in the registry, **When** registration is attempted, **Then** the registry returns `missing_required_connector` and does not write the capability.
3. **Given** a capability contract that fails multiple field validations simultaneously, **When** registration is attempted, **Then** the registry returns all violations in a single error response (not just the first).

### User Story 3 — Undeclared Event Emission Fails Execution (Priority: P1)

As a platform operator, I want the runtime to record a `undeclared_event_emission` violation in the trace and fail execution when a capability emits an event not declared in its contract's event catalog so that event catalog drift is caught at runtime.

**Why this priority**: Event catalog integrity is a contractual guarantee; silent undeclared emissions would corrupt downstream event consumers and audit trails.

**Independent Test**: Deploy a capability whose implementation emits an event type not listed in its contract's `event_catalog`. Execute it. Verify the execution trace contains `undeclared_event_emission` and the execution fails.

**Acceptance Scenarios**:

1. **Given** a capability that emits an event type not declared in its contract's event catalog, **When** the capability executes and the event is emitted, **Then** the runtime records `undeclared_event_emission` in the trace and fails the execution.
2. **Given** a capability that emits only events declared in its contract's event catalog, **When** execution completes, **Then** no `undeclared_event_emission` violation appears in the trace.
3. **Given** a capability that emits two events — one declared and one undeclared — **When** execution runs, **Then** the trace records the undeclared emission specifically and still fails execution.

### User Story 4 — Draft Quarantine Passes CI Without Errors (Priority: P2)

As a developer, I want to place a draft contract under `drafts/` so that CI passes without validation errors and no example or governed spec references it.

**Why this priority**: The draft quarantine enables safe experimentation without polluting the governed artifact space or triggering CI failures for intentionally incomplete artifacts.

**Independent Test**: Place a draft contract under `drafts/contracts/`. Run the CI gate. Verify it exits 0 and the draft artifact is not subject to production-schema validation. Verify that placing a reference to the draft inside `workflows/` causes CI to fail with `draft_reference_in_production_artifact`.

**Acceptance Scenarios**:

1. **Given** a draft contract placed under `drafts/contracts/`, **When** the CI gate runs, **Then** it exits 0 and does not validate the draft artifact against production schema rules.
2. **Given** a workflow in `workflows/` that references an artifact path starting with `drafts/`, **When** the CI gate runs, **Then** it fails with `draft_reference_in_production_artifact` identifying the workflow path and the draft artifact path.
3. **Given** the runtime is invoked with an execution request that resolves to an artifact path starting with `drafts/`, **When** the runtime processes the request, **Then** it rejects execution with `draft_artifact_not_executable` before any module code runs.

## Edge Cases

- Contract references an event type not in the event registry — `unresolved_event_reference` at registration time; not a CI-time error (event registry is not available at CI time)
- Workflow references a capability that exists in the registry but belongs to a different workspace scope — `capability_not_in_scope` at registration time
- Artifact in `drafts/` is referenced by a workflow in `workflows/` — CI fails with `draft_reference_in_production_artifact`; both the workflow path and the draft artifact path are included in the violation record
- All violations reported in one response — both CI and registration-time gates MUST aggregate all violations and report them together, never fail-fast on the first
- Contract JSON is syntactically valid JSON but fails schema validation on multiple fields simultaneously — all field-level violations reported in one response
- Workflow graph contains a dangling edge reference (references a node that does not exist) — `dangling_workflow_edge` at CI time and registration time
- Event catalog in a contract references an event type that exists in the event registry but with a different schema version — `event_schema_version_mismatch` at registration time
- Execution-time event catalog check runs for a capability that declares an empty event catalog — any emitted event triggers `undeclared_event_emission`; no events emitted is valid
- CI gate is run against a directory that contains no governed artifacts — gate exits 0 with no violations
- Two contracts in the same PR both have the same `id` field — `duplicate_artifact_id` reported for both; neither is valid

## Functional Requirements

- **FR-001**: The Contractual Enforcement Gate MUST operate at three enforcement layers: authoring/CI-time, registration-time, and execution-time; all three layers are required and none may be omitted.
- **FR-002**: The CI-time gate MUST scan all governed artifact directories (`contracts/`, `workflows/`, `specs/`, `crates/`) and fail for ANY validation error found anywhere in those directories; no partial-pass mode is permitted.
- **FR-003**: At CI time, the gate MUST validate: contract JSON schema validity, required fields (`kind`, `id`, `version`, `service_type`, `artifact_type` for capability contracts), event catalog completeness (all referenced event types are defined in the local event schema files), and workflow graph validity (no dangling edge references).
- **FR-004**: At registration time, the gate MUST perform all CI-time validations plus: referenced connector contracts must be registered, referenced event types must be present in the event registry.
- **FR-005**: At execution time, the gate MUST validate input and output schemas against the capability contract and MUST verify that every event emitted during execution is declared in the capability contract's event catalog.
- **FR-006**: If a capability emits an event not declared in its contract's event catalog, the runtime MUST record a `undeclared_event_emission` violation in the execution trace and MUST fail the execution.
- **FR-007**: Every validation failure produced by any enforcement layer MUST carry a violation record containing: `violation_code` (stable string), `path` (artifact path), and `message` (human-readable description).
- **FR-008**: All enforcement layers MUST aggregate all violations before reporting; fail-fast-on-first behavior is explicitly prohibited.
- **FR-009**: Artifacts placed under `drafts/` (top-level directory) MUST be excluded from production schema validation by the CI-time gate.
- **FR-010**: A workflow, governed spec, or example that references an artifact path starting with `drafts/` MUST cause the CI-time gate to fail with `draft_reference_in_production_artifact` identifying both the referencing artifact and the draft artifact path.
- **FR-011**: The runtime MUST reject execution of any artifact whose resolved path starts with `drafts/` with error code `draft_artifact_not_executable`, before any module code is executed.
- **FR-012**: The `violation_code` values used by the enforcement gate MUST be defined as a stable enumeration in the governing spec and MUST NOT be changed without a spec revision.
- **FR-013**: The CI-time gate script MUST exit with non-zero status when any violation is found and exit 0 only when no violations are found.
- **FR-014**: The CI-time gate MUST produce its violation output in a machine-readable format (JSON array of violation records) in addition to any human-readable summary.
- **FR-015**: Registration-time validation MUST reject the entire registration request if any violation is found; no partial registration is permitted.
- **FR-016**: The CI-time gate MUST be runnable as a standalone script (`scripts/ci/`) without requiring the Traverse registry or runtime to be running.
- **FR-017**: Workflow graph validation at CI time MUST detect and report dangling edge references — edges that point to workflow nodes not defined in the same workflow artifact.
- **FR-018**: Event catalog completeness validation at CI time MUST verify that every event type referenced in a capability contract's `event_catalog` field is defined in a local event schema file within the governed artifacts; unresolvable event type references at CI time MUST produce `unresolved_local_event_reference`.
- **FR-019**: The `unresolved_event_reference` violation at registration time MUST identify the event type that is missing from the event registry.
- **FR-020**: Duplicate `id` values across capability contracts submitted in the same registration batch MUST produce a `duplicate_artifact_id` violation for each conflicting entry.

## Non-Functional Requirements

- **NFR-001 Completeness**: All enforcement layers MUST report all violations before exiting or returning; partial reports that omit some violations are treated as gate failures.
- **NFR-002 Determinism**: Given the same set of artifacts and the same registry state, each enforcement layer MUST produce the same set of violations in the same order on every run.
- **NFR-003 Independence**: The CI-time gate MUST operate without a running registry or runtime; it MUST NOT require network access or external service availability.
- **NFR-004 Testability**: CI-time validation logic, registration-time validation logic, execution-time event catalog checking, and draft quarantine enforcement MUST each be independently testable with 100% automated line coverage.
- **NFR-005 Stability**: `violation_code` values MUST be stable across minor version increments; additions are permitted, changes or removals MUST require a spec revision.
- **NFR-006 Performance**: The CI-time gate MUST complete validation of the entire governed artifact tree within a time budget suitable for standard CI pipelines (no external blocking I/O).
- **NFR-007 Traceability**: Every violation record emitted at execution time MUST appear in the runtime execution trace with the same `violation_code`, `path`, and `message` fields used at CI and registration time.

## Non-Negotiable Quality Standards

- **QG-001**: All three enforcement layers MUST be active and non-bypassable; no code path MAY execute a governed artifact that has not passed all applicable layers.
- **QG-002**: Violation reporting MUST be aggregate; any enforcement layer that emits only the first violation and stops is non-compliant with this spec.
- **QG-003**: The CI-time gate script MUST exit non-zero on any violation and MUST exit 0 only when all governed artifact directories are valid.
- **QG-004**: A draft artifact MUST never be executable; the runtime MUST reject it with `draft_artifact_not_executable` before any module code runs.
- **QG-005**: CI-time validation, registration-time validation, execution-time event catalog checking, and draft quarantine enforcement MUST each reach 100% automated line coverage under the quality gate.

## Key Entities

- **Enforcement Layer**: One of the three validation stages — authoring/CI-time, registration-time, or execution-time — each with a defined scope and set of validation rules.
- **Violation Record**: The structured unit of a validation failure, carrying `violation_code` (stable string), `path` (artifact path), and `message` (human-readable description).
- **Violation Code**: A stable, enumerated string identifier for a specific type of validation failure; defined in this spec and versioned with it.
- **CI-Time Gate**: The standalone script in `scripts/ci/` that validates all governed artifact directories without requiring a running registry or runtime.
- **Registration-Time Gate**: The validation layer that runs when an artifact is submitted to the Traverse registry; includes all CI-time rules plus referential integrity checks.
- **Execution-Time Gate**: The validation layer that runs during capability execution; enforces input/output schema compliance and event catalog integrity.
- **Draft Quarantine**: The convention that experimental artifacts MUST reside under `drafts/` (top-level); draft artifacts are excluded from production validation, are never executable, and must not be referenced by production artifacts.
- **Event Catalog**: The list of event types declared in a capability contract that the capability is permitted to emit; any emission outside this list triggers `undeclared_event_emission`.

## Success Criteria

- **SC-001**: A PR with a malformed contract JSON fails the CI gate with a machine-readable violation record identifying the exact field and file, and the PR is blocked from merge.
- **SC-002**: A capability registration with a missing required field is rejected by the registry with a structured violation record before any registry state is modified.
- **SC-003**: A capability that emits an undeclared event during execution produces `undeclared_event_emission` in the execution trace and execution fails.
- **SC-004**: A draft artifact placed under `drafts/` does not trigger CI validation errors; a workflow referencing it triggers `draft_reference_in_production_artifact`.
- **SC-005**: All three enforcement layers aggregate all violations before reporting; no enforcement layer exits after the first violation.
- **SC-006**: CI-time validation, registration-time validation, execution-time event catalog checking, and draft quarantine enforcement each reach 100% automated line coverage.

## Out of Scope

- Automated remediation or auto-fix of validation violations
- UI or dashboard surfaces for violation reporting
- Validation of non-governed directories outside `contracts/`, `workflows/`, `specs/`, `crates/`, and `drafts/`
- Schema migration tooling for evolving existing contracts to new required fields
- Enforcement of semantic correctness beyond the structural and referential rules defined in this spec
- Cross-workspace validation (validating capability references across multiple Traverse workspaces)
- Real-time file-watch validation during local development (CI and registration-time gates only)

# Feature Specification: Runtime Request and Local Execution Model

**Feature Branch**: `006-runtime-request-execution`  
**Created**: 2026-03-27  
**Status**: Draft  
**Input**: Foundation runtime slice for `traverse-runtime`, covering request schema, deterministic local execution, ambiguity behavior, runtime state transitions, and trace output.

## Purpose

This spec defines the first implementation-governing runtime slice for Traverse.

It narrows the broad `Foundation v0.1` runtime intent into a concrete, testable model for:

- accepting a runtime request
- discovering eligible capabilities
- rejecting ambiguity deterministically
- executing one capability locally
- producing runtime state events
- producing a structured trace for success and failure paths

This slice does **not** define workflow traversal yet. It is intentionally limited to single-capability execution so the runtime control plane can be built and verified cleanly before graph traversal is added.

## User Scenarios and Testing

### User Story 1 - Execute One Registered Capability Locally (Priority: P1)

As a platform developer, I want to submit a runtime request that resolves to one registered capability and executes it locally so that Traverse proves its first real control-plane execution path.

**Why this priority**: Without deterministic request handling and one successful local execution path, there is no usable runtime foundation.

**Independent Test**: Register one valid capability, submit a valid runtime request, and verify the runtime returns a result, runtime state events, and a structured execution trace.

**Acceptance Scenarios**:

1. **Given** a registered executable capability and a valid runtime request, **When** the runtime resolves exactly one eligible candidate, **Then** it executes the capability locally and returns a successful execution result.
2. **Given** a successful execution, **When** the runtime completes the request, **Then** it emits ordered runtime state transitions and a structured trace artifact describing request handling, discovery, selection, execution, and completion.
3. **Given** a registered capability whose contract is not runnable in the current runtime context, **When** the request is evaluated, **Then** the runtime rejects it before execution with explicit validation evidence and a failure trace.

### User Story 2 - Reject Ambiguous Runtime Requests Safely (Priority: P1)

As a platform developer, I want ambiguous runtime requests to fail explicitly so that Traverse does not hide unsafe runtime decisions behind undocumented heuristics.

**Why this priority**: Explicit ambiguity failure is one of the non-negotiable runtime behaviors for `v0.1`.

**Independent Test**: Register two capabilities that both match the same runtime intent and verify the runtime refuses execution while still producing a trace and state transitions.

**Acceptance Scenarios**:

1. **Given** two or more eligible registered capabilities match the same runtime request, **When** the runtime cannot deterministically narrow them to one candidate, **Then** the request fails with an `ambiguous_match` style error result.
2. **Given** an ambiguity failure, **When** the runtime completes the request, **Then** it emits runtime state transitions through discovery and selection before ending in `error`.
3. **Given** an ambiguity failure, **When** the trace is produced, **Then** it records all matching candidates and the reason no candidate was selected.

### User Story 3 - Capture Explainable Runtime Evidence (Priority: P2)

As a platform developer or reviewer, I want each runtime execution attempt to produce machine-readable evidence so that CI, debugging, and future UI or MCP consumers can inspect what happened.

**Why this priority**: Explainability is part of the runtime contract, not an optional logging add-on.

**Independent Test**: Execute one successful request and one failure request, then verify both produce valid trace artifacts and runtime state event streams with stable identifiers.

**Acceptance Scenarios**:

1. **Given** a successful runtime execution, **When** the trace is inspected, **Then** it contains request identity, candidate evaluation, selected capability, execution metadata, and final result.
2. **Given** a request rejected before execution, **When** the trace is inspected, **Then** it still contains request identity, candidate evaluation details, failure classification, and terminal state.
3. **Given** a runtime consumer subscribed to state events, **When** a request runs, **Then** the consumer receives state changes in deterministic order with matching execution identifiers.

## Edge Cases

- What happens when a runtime request names an exact capability identity and version that are not present in the selected lookup scope?
- What happens when a request intent matches only capabilities in the public registry but a private overlay exists for a different version?
- What happens when a candidate contract is active but its execution metadata is incompatible with `v0.1` local execution rules?
- What happens when the runtime finds one candidate but the referenced artifact metadata is missing or incomplete?
- What happens when the runtime begins execution and the capability returns output that does not satisfy the declared contract?
- What happens when a request omits optional context fields such as preferred scope, exact version, or execution metadata?

## Functional Requirements

- **FR-001**: The runtime MUST accept a machine-readable runtime request artifact as the input boundary for capability execution.
- **FR-002**: A runtime request MUST support intent-based lookup and optional exact identity targeting.
- **FR-003**: A runtime request MUST carry a stable `request_id`.
- **FR-004**: The runtime MUST derive a stable `execution_id` for each execution attempt.
- **FR-005**: The runtime MUST support lookup scopes that distinguish at least `public_only` and `prefer_private`.
- **FR-006**: When the request specifies an exact capability identity and version, the runtime MUST resolve that exact registration or fail explicitly.
- **FR-007**: When the request specifies an intent rather than an exact capability identity, the runtime MUST collect eligible candidates from the registry using deterministic ordering rules.
- **FR-008**: The runtime MUST reject a request when no eligible capability matches the request.
- **FR-009**: The runtime MUST reject a request when more than one eligible capability remains after deterministic filtering and no safe tie-break rule is defined.
- **FR-010**: The runtime MUST execute only one selected capability for this slice and MUST use the `local` placement implementation only.
- **FR-011**: The runtime MUST validate that the selected capability is locally runnable according to its registered execution metadata before attempting execution.
- **FR-012**: The runtime MUST validate request input against the selected capability contract input schema before execution.
- **FR-013**: The runtime MUST validate execution output against the selected capability contract output schema before returning success.
- **FR-014**: The runtime MUST surface execution failure explicitly when contract validation, artifact availability, or capability execution fails.
- **FR-015**: The runtime MUST expose a state machine with at least the states `loading_registry`, `ready`, `discovering`, `evaluating_constraints`, `selecting`, `executing`, `completed`, and `error`.
- **FR-016**: The runtime MUST emit state events in deterministic order for each execution attempt.
- **FR-017**: Every execution attempt MUST produce a structured runtime trace, including successful execution, no-match failure, ambiguity failure, validation failure, and execution failure.
- **FR-018**: The trace MUST include candidate collection and candidate rejection information when discovery occurs.
- **FR-019**: The trace MUST include the selected capability record and artifact reference when execution occurs.
- **FR-020**: The trace MUST include terminal status and normalized failure classification when the runtime does not complete successfully.
- **FR-021**: The runtime MUST preserve the placement abstraction in the execution model, but only the `local` placement target is permitted in this slice.
- **FR-022**: The runtime MUST keep request, state, and trace artifacts machine-readable and stable enough for future MCP and UI consumption.
- **FR-023**: The runtime MUST NOT bypass registry lookup, contract validation, state emission, or trace generation through ad hoc execution paths.
- **FR-024**: The runtime MUST support deterministic replay-style testing by keeping state ordering and trace field semantics stable for identical inputs and registry state.

## Non-Functional Requirements

- **NFR-001 Determinism**: Candidate collection, candidate ordering, ambiguity detection, state ordering, and trace generation MUST be deterministic for the same registry state and request input.
- **NFR-002 Explainability**: Failure and success paths MUST preserve enough structured detail to explain runtime behavior without relying on unstructured logs alone.
- **NFR-003 Portability**: This slice MUST model execution in a way that preserves future browser, edge, and cloud placement without changing the request boundary.
- **NFR-004 Testability**: Core runtime decision and execution logic MUST be separable enough to achieve 100% automated line coverage.
- **NFR-005 Compatibility**: Runtime request and trace shapes MUST be versionable and suitable for semver discipline under the broader foundation contract.
- **NFR-006 Maintainability**: Request parsing, candidate resolution, execution validation, state transitions, and trace assembly MUST remain clearly separated inside `traverse-runtime`.

## Non-Negotiable Quality Standards

- **QG-001**: Ambiguity MUST fail explicitly and MUST NOT be silently resolved by hidden heuristics.
- **QG-002**: Every runtime terminal path MUST emit a terminal state and a terminal trace result.
- **QG-003**: Input and output contract validation MUST remain on the normal execution path and MUST NOT be bypassed.
- **QG-004**: Core runtime logic for this slice MUST reach 100% automated line coverage.
- **QG-005**: Runtime request and trace behavior MUST align with the governing spec and fail merge validation when drift occurs.

## Key Entities

- **Runtime Request**: The machine-readable invocation artifact that expresses intent, optional exact capability targeting, input payload, lookup preferences, and request context.
- **Runtime Execution Context**: The request-scoped metadata used for deterministic filtering and local execution evaluation.
- **Runtime State Event**: A deterministic emitted event representing one state-machine transition for an execution attempt.
- **Runtime Trace**: The structured explainability artifact produced for one execution attempt.
- **Runtime Candidate**: A registry-derived candidate capability considered during discovery and selection.
- **Runtime Execution Result**: The terminal success or failure output returned by the runtime for a single request.

## Success Criteria

- **SC-001**: A registered executable capability can be resolved and executed locally from one runtime request without bypassing registry, validation, state, or trace logic.
- **SC-002**: A request with no eligible candidates fails predictably with a structured failure result and trace.
- **SC-003**: A request with multiple eligible candidates fails predictably with an ambiguity result and trace containing all remaining candidates.
- **SC-004**: Runtime state events are emitted in deterministic order for success and failure cases.
- **SC-005**: Core runtime logic for this slice reaches 100% automated line coverage under the protected coverage gate.

## Out of Scope

- Workflow traversal
- Event-driven orchestration across multiple capabilities
- distributed or remote placement execution
- browser runtime adapters
- MCP transport details
- retries, backoff, and long-running execution management

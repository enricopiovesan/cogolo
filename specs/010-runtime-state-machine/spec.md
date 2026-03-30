# Feature Specification: Runtime State Machine

**Feature Branch**: `010-runtime-state-machine`  
**Created**: 2026-03-30  
**Status**: Draft  
**Input**: Dedicated runtime-governance slice for the Traverse runtime state machine, including state values, transition rules, emitted state-event semantics, and terminal behavior.

## Purpose

This spec defines the dedicated runtime state-machine slice for Traverse.

It narrows the broader runtime requirements from the foundation and request/workflow slices into a concrete, testable model for:

- the canonical runtime state set
- allowed transitions between states
- per-request execution state semantics
- stable runtime state event payload expectations
- terminal success and failure behavior
- explainable state evidence that future UI, MCP, and trace consumers can rely on

This slice does **not** define transport protocols, browser subscription APIs, retry orchestration, or distributed placement behavior. It is intentionally focused on the runtime state model itself so later runtime features share one governed lifecycle contract.

## User Scenarios and Testing

### User Story 1 - Observe Deterministic Runtime Lifecycle (Priority: P1)

As a platform developer, I want the runtime to expose a deterministic state machine so that request execution is explainable, testable, and stable for downstream consumers.

**Why this priority**: Traverse already emits runtime states, but without a dedicated governing slice the exact allowed states and transitions can drift across runtime features.

**Independent Test**: Execute one successful runtime request and verify the emitted runtime state sequence matches the governed transition order from request start to terminal completion.

**Acceptance Scenarios**:

1. **Given** a successful exact or intent-based runtime request, **When** the runtime processes it, **Then** it emits only governed state values in an allowed order.
2. **Given** a successful execution that emits domain events, **When** the runtime completes execution, **Then** it enters `emitting_events` before entering `completed`.
3. **Given** a successful workflow-backed execution, **When** traversal finishes, **Then** the runtime reaches the same governed terminal `completed` state as single-capability execution.

### User Story 2 - Fail Through Explicit Terminal State (Priority: P1)

As a reviewer or operator, I want runtime failures to end in explicit governed terminal states so that no execution path disappears into undocumented intermediate behavior.

**Why this priority**: Silent or ad hoc failure behavior would weaken Traverse’s non-negotiable explainability and quality guarantees.

**Independent Test**: Trigger one no-match failure, one ambiguity failure, and one execution failure, then verify each path emits allowed transitions and terminates in `error` with structured failure metadata.

**Acceptance Scenarios**:

1. **Given** a request with no eligible candidates, **When** discovery completes, **Then** the runtime transitions from `discovering` to `error` without entering `executing`.
2. **Given** a request with multiple eligible candidates after deterministic filtering, **When** selection fails, **Then** the runtime transitions through `selecting` before ending in `error`.
3. **Given** a selected capability that fails during execution, **When** the runtime handles the failure, **Then** it enters `executing`, emits a terminal `error` state, and records the failure classification.

### User Story 3 - Provide Stable State Evidence for Consumers (Priority: P2)

As a UI, MCP, or automation consumer, I want runtime state artifacts and transition rules to stay stable so that I can react to runtime progress without reverse-engineering internal implementation details.

**Why this priority**: Runtime states are part of the platform contract, not just internal implementation detail.

**Independent Test**: Validate that state events and transition records are machine-readable, stable, and sufficient to reconstruct one execution attempt from first transition to terminal result.

**Acceptance Scenarios**:

1. **Given** a runtime state event stream, **When** a consumer inspects it, **Then** each event includes execution identity, state value, timestamp, and structured details.
2. **Given** a runtime trace referencing state transitions, **When** it is inspected, **Then** the state sequence can be reconstructed without unstructured logs.
3. **Given** a future consumer that only understands governed state values, **When** the runtime implementation evolves internally, **Then** the governed state contract remains stable unless versioned intentionally.

## Edge Cases

- What happens when the runtime is initialized but no request has started yet?
- What happens when registry loading fails before the runtime ever reaches `ready`?
- What happens when a request fails during constraint evaluation before selection occurs?
- What happens when a request produces no domain events after successful execution?
- What happens when a workflow-backed capability traverses multiple nodes but still returns one terminal runtime result?
- What happens when a request is cancelled in a future slice, even though cancellation is out of scope here?

## Functional Requirements

- **FR-001**: The runtime MUST define a governed canonical state set containing at least `idle`, `loading_registry`, `ready`, `discovering`, `evaluating_constraints`, `selecting`, `executing`, `emitting_events`, `completed`, and `error`.
- **FR-002**: `idle`, `ready`, `completed`, and `error` MUST be treated as named lifecycle states even when internal implementation uses other helpers.
- **FR-003**: The runtime MUST expose allowed state transitions explicitly rather than relying on implicit implementation ordering.
- **FR-004**: The runtime MUST reject or treat as implementation defects any attempted transition that is not allowed by the governed state-transition table.
- **FR-005**: Runtime request execution MUST begin from `ready` and enter `discovering` as the first request-scoped active state.
- **FR-006**: Successful runtime execution MUST pass through `executing` and then `completed`; when domain or workflow events are emitted after execution, it MUST pass through `emitting_events` before `completed`.
- **FR-007**: Failed runtime execution MUST terminate in `error`, and `error` MUST be terminal for one execution attempt.
- **FR-008**: The runtime MUST permit terminal failure from `loading_registry`, `discovering`, `evaluating_constraints`, `selecting`, `executing`, and `emitting_events`.
- **FR-009**: The runtime MUST NOT enter `executing` before capability selection succeeds.
- **FR-010**: The runtime MUST NOT enter `emitting_events` before execution succeeds.
- **FR-011**: `completed` and `error` MUST be terminal states for one execution attempt; any subsequent request MUST begin from a fresh `ready` state context.
- **FR-012**: The runtime MUST emit a machine-readable runtime state event for every externally visible state transition.
- **FR-013**: Each runtime state event MUST include stable execution identity, request identity, governed state value, event timestamp, and structured transition details.
- **FR-014**: The runtime MUST preserve deterministic event ordering for identical inputs and identical registry state.
- **FR-015**: The runtime MUST provide a machine-readable transition record shape suitable for embedding in traces and later UI or MCP subscriptions.
- **FR-016**: The runtime MUST keep the state-machine contract shared across single-capability execution and workflow-backed capability execution.
- **FR-017**: Workflow-backed execution MAY emit additional traversal evidence, but it MUST still use the same governed runtime terminal states.
- **FR-018**: This slice MUST remain compatible with future placement, browser subscription, and MCP work without renaming existing governed state values unnecessarily.
- **FR-019**: The runtime MUST make the relationship between state values and normalized terminal result status explicit.
- **FR-020**: State-machine artifacts and validation behavior MUST remain machine-readable and suitable for protected CI validation in later implementation slices.

## Non-Functional Requirements

- **NFR-001 Determinism**: Runtime state values, transition ordering, terminal behavior, and state event emission MUST be deterministic for the same execution path.
- **NFR-002 Explainability**: The governed state model MUST be sufficient to explain where execution is in its lifecycle without relying on implementation-specific logs.
- **NFR-003 Compatibility**: Existing governed state values already referenced by runtime and workflow specs MUST remain stable unless changed under explicit versioned governance.
- **NFR-004 Testability**: State-transition logic MUST be separable enough to achieve 100% automated line coverage when implemented.
- **NFR-005 Maintainability**: The state machine MUST be defined once as the authoritative model rather than duplicated inconsistently across runtime features.
- **NFR-006 Extensibility**: Future cancellation, retry, placement, and UI-subscription slices MUST be able to extend state details without breaking the governed base states.

## Non-Negotiable Quality Standards

- **QG-001**: No runtime path may skip directly to success or failure without a governed terminal state.
- **QG-002**: No undocumented runtime state value may be exposed to external consumers in this slice.
- **QG-003**: State transitions MUST align with the approved state-transition table and fail implementation validation when drift occurs.
- **QG-004**: Core state-transition logic MUST reach 100% automated line coverage when implemented.
- **QG-005**: Workflow-backed execution MUST reuse the same governed runtime state contract rather than inventing a parallel workflow-only lifecycle for terminal runtime behavior.

## Key Entities

- **Runtime State**: One governed lifecycle value representing runtime progress for initialization or one execution attempt.
- **Runtime Transition Rule**: One allowed source-to-target state transition defined by this spec.
- **Runtime State Event**: A machine-readable event emitted when the runtime enters a governed externally visible state.
- **Runtime Transition Record**: A structured artifact capturing one transition, including from-state, to-state, timestamp, and reason metadata.
- **Terminal Runtime Result Mapping**: The explicit mapping between terminal runtime states and normalized execution result status.

## Success Criteria

- **SC-001**: Successful runtime execution emits only governed states in a valid order ending in `completed`.
- **SC-002**: Failure paths emit only governed states in a valid order ending in `error`.
- **SC-003**: State events contain stable machine-readable information sufficient for trace reconstruction and consumer use.
- **SC-004**: Workflow-backed execution uses the same governed runtime state contract as single-capability execution.
- **SC-005**: The dedicated state-machine slice becomes the authoritative reference for runtime lifecycle semantics across the repo.

## Out of Scope

- browser or MCP transport subscription protocols
- retry and cancellation behavior
- distributed placement state modeling
- UI presentation states
- trace schema redesign beyond the state-machine artifacts defined here

# Feature Specification: Workflow Registry and Deterministic Traversal

**Feature Branch**: `007-workflow-registry-traversal`  
**Created**: 2026-03-27  
**Status**: Draft  
**Input**: Foundation workflow slice for deterministic workflow registration, workflow metadata, and runtime traversal over registered capabilities and event edges.

## Purpose

This spec defines the first implementation-governing workflow slice for Cogolo.

It narrows the broader `Foundation v0.1` workflow intent into a concrete, testable model for:

- registering deterministic workflow definitions
- storing workflow metadata alongside governed workflow artifacts
- validating workflow references against capability and event contracts
- traversing one deterministic workflow path without planner heuristics
- representing a composed capability as a workflow-backed capability boundary
- producing structured traversal evidence suitable for later runtime traces and UI consumers

This slice does **not** define AI planning, dynamic path optimization, retries, or distributed workflow execution. It is intentionally limited to deterministic traversal over an approved workflow definition.

## User Scenarios and Testing

### User Story 1 - Register a Deterministic Workflow Definition (Priority: P1)

As a platform developer, I want to register a workflow definition with metadata and governed references so that workflows are first-class artifacts rather than hidden implementation glue.

**Why this priority**: Cogolo’s composition model depends on workflows being governed artifacts in the same way capabilities and events are governed.

**Independent Test**: Register a valid workflow definition referencing existing capability and event contracts, then verify the workflow registry stores the workflow artifact, metadata, and deterministic traversal order.

**Acceptance Scenarios**:

1. **Given** a valid workflow definition and valid referenced artifacts, **When** the workflow is registered, **Then** the workflow registry stores the workflow record, derived discovery metadata, and validation evidence.
2. **Given** a workflow definition that references a missing capability or event version, **When** registration is attempted, **Then** the workflow is rejected with explicit validation feedback.
3. **Given** a workflow definition whose published version already exists in the same scope, **When** the governed content differs, **Then** the registry rejects the republishing attempt as an immutable version conflict.

### User Story 2 - Traverse One Workflow Deterministically (Priority: P1)

As a platform developer, I want the runtime to traverse a registered workflow in deterministic order so that composed behavior remains explainable and stable.

**Why this priority**: Cogolo’s workflow value depends on composition being explicit and replayable, not hidden behind heuristics.

**Independent Test**: Execute a registered workflow with a valid workflow input and verify the runtime traverses the expected nodes and edges in deterministic order.

**Acceptance Scenarios**:

1. **Given** a workflow with direct sequential edges, **When** traversal begins, **Then** the runtime visits nodes in the declared deterministic order and records the visited node sequence.
2. **Given** a workflow with event-triggered edges, **When** a node emits the declared event, **Then** the runtime advances only to the nodes allowed by that event edge.
3. **Given** a workflow with no valid next step from the current node, **When** traversal cannot continue, **Then** the workflow run fails explicitly with structured traversal evidence.

### User Story 3 - Represent a Composed Capability Cleanly (Priority: P2)

As a developer or future agent, I want composed capabilities to be backed by workflow definitions so that higher-level business actions remain discoverable and reusable.

**Why this priority**: A composed capability is one of the main ways Cogolo turns graph traversal into stable business boundaries.

**Independent Test**: Register a workflow-backed composed capability and verify its implementation reference points to the workflow definition while preserving capability identity and versioning.

**Acceptance Scenarios**:

1. **Given** a workflow-backed composed capability, **When** it is registered, **Then** the registry records it as a capability with `implementation_kind = workflow` and a workflow reference.
2. **Given** a workflow-backed composed capability, **When** it is discovered, **Then** downstream consumers can inspect both capability metadata and the linked workflow identity.
3. **Given** a breaking workflow contract change for a composed capability, **When** a too-small semver bump is attempted, **Then** the compatibility check fails.

## Edge Cases

- What happens when a workflow references a capability that is present but not runtime-eligible?
- What happens when two nodes declare the same identifier?
- What happens when a workflow has more than one declared start node?
- What happens when an event edge references an event contract that the source node cannot emit?
- What happens when a direct edge would create a cycle in a deterministic `v0.1` traversal?
- What happens when a workflow-backed composed capability points to a workflow version that is missing from the workflow registry?

## Functional Requirements

- **FR-001**: The system MUST accept a machine-readable workflow definition artifact as the registration boundary for workflows.
- **FR-002**: A workflow definition MUST include stable identity, semver version, lifecycle, owner metadata, summary, tags, and governing spec metadata.
- **FR-003**: A workflow definition MUST declare nodes, edges, start nodes, and terminal nodes explicitly.
- **FR-004**: Each workflow node MUST reference one registered capability id and version.
- **FR-005**: Each workflow edge MUST declare its traversal trigger as either `direct` or `event`.
- **FR-006**: Event-triggered edges MUST reference one registered event contract id and version.
- **FR-007**: Workflow registration MUST fail when any referenced capability or event contract version is missing.
- **FR-008**: Workflow registration MUST fail when node identifiers are not unique.
- **FR-009**: Workflow registration MUST fail when there is not exactly one declared start node for this slice.
- **FR-010**: Workflow registration MUST fail when traversal edges create an invalid deterministic cycle for `v0.1`.
- **FR-011**: The workflow registry MUST preserve immutable publication semantics per `(scope, id, version)`.
- **FR-012**: The workflow registry MUST store both the authoritative workflow artifact and a derived discovery/index record.
- **FR-013**: The workflow registry MUST expose participating capability ids, event ids, workflow tags, lifecycle, and owner metadata for discovery.
- **FR-014**: The runtime MUST traverse a registered workflow using deterministic ordering and declared start/edge semantics only.
- **FR-015**: Deterministic traversal MUST NOT use planner heuristics or implicit best-path selection in this slice.
- **FR-016**: For direct edges, the runtime MUST advance only along the explicitly declared next node relation.
- **FR-017**: For event edges, the runtime MUST advance only when the required event reference is emitted by the source node.
- **FR-018**: The runtime MUST record visited nodes, traversed edges, and emitted events as structured traversal evidence.
- **FR-019**: The runtime MUST fail explicitly when a workflow cannot reach a valid next node or terminal node under the declared rules.
- **FR-020**: A composed capability MUST be representable as a first-class capability whose implementation reference points to a workflow definition.
- **FR-021**: Workflow-backed composed capabilities MUST remain subject to capability semver, immutability, and compatibility rules.
- **FR-022**: The workflow slice MUST keep workflow artifacts, derived registry entries, and traversal evidence machine-readable for future UI, MCP, and agent use.

## Non-Functional Requirements

- **NFR-001 Determinism**: Registration validation, start-node resolution, edge ordering, and runtime traversal order MUST be deterministic for the same registry state and workflow definition.
- **NFR-002 Explainability**: Workflow traversal MUST produce structured evidence that explains visited nodes, traversed edges, emitted events, and terminal outcome.
- **NFR-003 Portability**: Workflow definitions MUST describe orchestration semantics without assuming a specific UI, cloud, or host environment.
- **NFR-004 Testability**: Core workflow registry and traversal logic MUST remain separable enough to achieve 100% automated line coverage once implemented.
- **NFR-005 Compatibility**: Workflow identity and versioning MUST support semver discipline and immutable publication semantics.
- **NFR-006 Maintainability**: Workflow validation, registry storage, traversal planning, and traversal execution evidence MUST remain clearly separated in the implementation.

## Non-Negotiable Quality Standards

- **QG-001**: Workflow traversal MUST remain deterministic and MUST NOT silently choose among multiple valid paths without an approved rule.
- **QG-002**: Workflow registration MUST reject missing, incompatible, or undeclared references instead of repairing them implicitly.
- **QG-003**: Workflow-backed composed capabilities MUST remain discoverable as capabilities and MUST NOT be hidden as undocumented workflow-only artifacts.
- **QG-004**: Core workflow registry and traversal logic MUST reach 100% automated line coverage when implemented.
- **QG-005**: Workflow registry and traversal behavior MUST align with the governing spec and fail merge validation when drift occurs.

## Key Entities

- **Workflow Definition**: The authoritative machine-readable artifact describing nodes, edges, start node, terminal nodes, tags, and workflow metadata.
- **Workflow Registry Record**: The stored workflow artifact plus derived metadata, evidence, and immutable publication identity.
- **Workflow Node**: A workflow step that references one capability contract version and declares input/output mapping behavior.
- **Workflow Edge**: A deterministic transition between nodes, triggered either directly or by an event contract.
- **Workflow Traversal Evidence**: The structured artifact describing node visitation order, traversed edges, emitted events, and terminal traversal status.
- **Workflow-backed Capability**: A composed capability whose implementation reference points to a workflow definition rather than a direct executable artifact.

## Success Criteria

- **SC-001**: A valid workflow definition can be registered, indexed, and discovered with metadata derived from its governed artifact.
- **SC-002**: A workflow definition referencing missing or incompatible capability/event versions is rejected predictably with structured validation evidence.
- **SC-003**: One deterministic workflow can be traversed in declared node order without planner heuristics.
- **SC-004**: Workflow traversal produces structured evidence capturing visited nodes, traversed edges, and terminal status.
- **SC-005**: Workflow-backed composed capabilities remain discoverable as first-class capabilities linked to workflow definitions.

## Out of Scope

- AI planning or dynamic workflow generation
- multi-path optimization
- distributed workflow execution
- retries, backoff, compensation, or saga management
- browser runtime subscription behavior
- full metadata graph query model beyond workflow-local traversal semantics

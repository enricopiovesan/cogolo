# Feature Specification: Browser-Hosted MCP Consumer Model

**Feature Branch**: `codex/issue-174-browser-hosted-mcp-consumer-model`  
**Created**: 2026-04-07  
**Status**: Draft  
**Input**: Issue `#174`, the approved downstream consumer contract, the local browser adapter transport slice, the dedicated Traverse MCP server model, and the need for one governed browser-hosted consumption model for apps like `youaskm3`.

## Purpose

This specification defines the first governed browser-hosted MCP consumer model for Traverse.

It narrows the broader downstream-consumer contract into one explicit browser-hosted boundary for apps such as `youaskm3` so that:

- a browser-hosted app can know which Traverse surfaces are intentionally public
- the app can know which transport and packaging assumptions are allowed for the first supported browser-hosted path
- the app can know what remains out of scope for this first browser-hosted consumption model
- release readiness can be evaluated without guessing which browser-hosted integration shape Traverse intends to support

This slice governs the browser-hosted consumer boundary only. It does **not** redefine runtime execution semantics, browser adapter internals, MCP server internals, or `youaskm3` product UX.

## User Scenarios and Testing

### User Story 1 - Define the Browser-Hosted Consumer Boundary (Priority: P1)

As a downstream app steward, I want one governed browser-hosted consumer boundary so that I can tell whether a browser app is consuming Traverse in the supported way.

**Why this priority**: The browser-hosted path is the first real downstream shape that needs a clear contract before implementation can proceed safely.

**Independent Test**: A reviewer can read this spec alone and explain the supported browser-hosted Traverse boundary without referring to internal crate layout or repo archaeology.

**Acceptance Scenarios**:

1. **Given** a browser-hosted app wants to integrate Traverse, **When** it reads this spec, **Then** it can identify the approved public Traverse surfaces and what remains internal-only.
2. **Given** a reviewer compares browser-hosted consumption to the dedicated MCP server model, **When** this spec is reviewed, **Then** the relationship between the two is explicit.
3. **Given** the downstream app is `youaskm3` or a similar browser-hosted client, **When** the consumer boundary is reviewed, **Then** the browser-hosted path is generic enough to support future downstream apps without inheriting app-specific UX.

### User Story 2 - Define the Allowed Browser-Hosted Transport and Packaging Assumptions (Priority: P1)

As a release steward, I want the first browser-hosted Traverse path to have explicit transport and packaging assumptions so that app teams know what they can and cannot rely on.

**Why this priority**: A browser-hosted consumer model is only useful if it tells implementers which packaging and transport assumptions are actually supported.

**Independent Test**: A reviewer can identify the supported browser-hosted transport and bundle assumptions from this spec alone and see that unsupported assumptions are called out explicitly.

**Acceptance Scenarios**:

1. **Given** a browser-hosted app is using Traverse, **When** it follows the governed path, **Then** it depends on a released consumer bundle rather than internal-only modules.
2. **Given** a browser-hosted app needs runtime updates and terminal results, **When** it consumes Traverse, **Then** it does so through the approved browser-hosted transport assumptions rather than a local stdio-only assumption.
3. **Given** a browser-hosted app tries to assume unsupported deployment or auth behavior, **When** this spec is reviewed, **Then** those assumptions are explicitly out of scope.

### User Story 3 - Make the Model Concrete Enough for Release Readiness and Validation (Priority: P2)

As a release steward, I want this browser-hosted consumer model to be concrete enough for implementation, validation, and release planning so that the first browser-hosted app path can be reviewed without interpretation.

**Why this priority**: The model must support future tickets for implementation, compatibility validation, and downstream starter-kit documentation.

**Independent Test**: A reviewer can turn this spec into follow-on implementation, validation, and documentation tickets without inventing missing policy.

**Acceptance Scenarios**:

1. **Given** a team wants to implement the browser-hosted consumer path, **When** it reads this spec, **Then** it can identify the minimum supported browser-hosted surfaces and the release-readiness boundary.
2. **Given** a team wants to validate a browser-hosted app like `youaskm3`, **When** it reads this spec, **Then** it can determine what the consumer artifact and compatibility expectations must prove.
3. **Given** a team wants to extend Traverse to another browser-hosted app later, **When** it uses this spec, **Then** the browser-hosted contract remains reusable and versionable.

## Edge Cases

- A browser-hosted app tries to consume Traverse through raw internal crate APIs instead of a published consumer boundary.
- A browser-hosted app assumes the dedicated MCP server model alone is enough without the browser-hosted consumer path.
- A browser-hosted app expects unsupported remote deployment, multi-tenant, or auth behavior in the first browser-hosted slice.
- A browser-hosted app attempts to use an unversioned or unpublished consumer bundle.
- A browser-hosted app needs compatibility guarantees that go beyond the first governed browser-hosted contract.

## Requirements

### Functional Requirements

- **FR-001**: Traverse MUST define one governed browser-hosted MCP consumer model.
- **FR-002**: The model MUST identify the public Traverse surfaces a browser-hosted app may depend on.
- **FR-003**: The model MUST distinguish browser-hosted consumer surfaces from internal Traverse implementation details.
- **FR-004**: The model MUST describe the relationship between the browser-hosted consumer path, the local browser adapter transport, and the dedicated Traverse MCP server model.
- **FR-005**: The model MUST define the first supported browser-hosted packaging assumption as a published, versioned consumer bundle rather than a direct dependency on internal repo structure.
- **FR-006**: The model MUST define the first supported browser-hosted transport assumption in a way that does not require stdio-only local execution.
- **FR-007**: The model MUST identify what browser-hosted apps like `youaskm3` may rely on for runtime updates, trace visibility, and terminal outcomes.
- **FR-008**: The model MUST explicitly state unsupported assumptions that remain out of scope for the first browser-hosted slice.
- **FR-009**: The model MUST define browser-hosted compatibility expectations at the level of documented, versioned public surfaces.
- **FR-010**: The model MUST be specific enough to support one implementation ticket and one validation ticket without further interpretation.
- **FR-011**: The model MUST remain compatible with the downstream consumer contract and the existing browser adapter and MCP server slices.
- **FR-012**: Approved implementation and validation work for browser-hosted consumption MUST be checked against this governing spec before merge.

### Key Entities

- **Browser-Hosted Consumer Model**: The governed browser-facing Traverse consumption boundary for downstream apps such as `youaskm3`.
- **Published Consumer Bundle**: The versioned Traverse artifact set that a browser-hosted app is allowed to depend on.
- **Browser-Hosted Downstream App**: An external app that consumes Traverse from within a browser-hosted integration path.
- **Supported Public Surface Set**: The Traverse surfaces intentionally exposed for browser-hosted consumption.
- **Unsupported Assumption**: A deployment, transport, or security expectation that the first browser-hosted slice does not guarantee.
- **Compatibility Rule**: The documented stability expectation for public browser-hosted consumer surfaces.

## Success Criteria

### Measurable Outcomes

- **SC-001**: A reviewer can explain the browser-hosted Traverse consumer boundary without reading implementation code.
- **SC-002**: A reviewer can explain how the browser-hosted consumer model relates to the local browser adapter transport and the dedicated MCP server model.
- **SC-003**: A browser-hosted app can identify the supported public Traverse surfaces from governed docs/specs alone.
- **SC-004**: The browser-hosted consumer model is concrete enough to support follow-on implementation and validation tickets without guesswork.
- **SC-005**: The first browser-hosted path is bounded tightly enough that unsupported transport, auth, or deployment assumptions are explicitly out of scope.

## Assumptions

- `youaskm3` is the first browser-hosted downstream app the contract must support.
- The local browser adapter transport and dedicated MCP server model already exist as the base public surfaces this model narrows.
- Browser-hosted consumers should rely on a published, versioned consumer bundle rather than undocumented repository internals.
- Unsupported auth, multi-tenant, and remote deployment guarantees remain out of scope unless a future governed slice explicitly adds them.
- Implementation detail tickets for this model will be created separately after this governing spec is approved.

## Governing Relationship

This specification is governed by:

- `001-foundation-v0-1`
- `006-runtime-request-execution`
- `010-runtime-state-machine`
- `013-browser-runtime-subscription`
- `019-downstream-consumer-contract`
- `019-local-browser-adapter-transport`
- `022-mcp-wasm-server`
- constitution version `1.2.0`

This specification is intended to govern future implementation and documentation in:

- browser-hosted consumer bundle and compatibility validation slices
- downstream browser-hosted integration guidance for apps such as `youaskm3`
- future release-readiness and compatibility documentation tied to browser-hosted consumer use

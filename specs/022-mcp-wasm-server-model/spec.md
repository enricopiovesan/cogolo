# Feature Specification: MCP WASM Server Model

**Feature Branch**: `022-mcp-wasm-server-model`  
**Created**: 2026-04-06  
**Status**: Draft  
**Input**: The agreed direction that Traverse needs a dedicated MCP server product surface for downstream consumers such as `youaskm3`, using the useful patterns from UMA Chapter 13 while keeping Traverse runtime authority, governed artifacts, and product-shaped APIs.

## Purpose

This specification defines the first dedicated MCP WASM server model for Traverse.

It narrows the broader MCP and downstream-consumer goals into one explicit product model for:

- a dedicated Traverse MCP server package
- a first supported `stdio` host mode
- Traverse-native MCP operations plus a generic workflow-oriented convenience layer
- runtime-authoritative capability and workflow execution behind the MCP façade
- governed WASM-hosted capability and agent participation under approved contracts and manifests

This slice exists so Traverse can expose MCP as a first-class product surface rather than only as a foundation concept, chapter reference, or app-specific integration trick.

This slice does **not** define browser transport, HTTP/SSE MCP hosting, UI behavior, or federation behavior. It is intentionally focused on the first dedicated, local-first, stdio-hosted MCP server shape.

## User Scenarios and Testing

### User Story 1 - Discover and Invoke Governed Traverse Surfaces Through MCP (Priority: P1)

As a downstream developer, I want one dedicated Traverse MCP server so that I can discover governed capabilities and workflows and invoke them through MCP without reimplementing Traverse runtime behavior.

**Why this priority**: Your chosen architecture makes Traverse the runtime and MCP substrate under apps like `youaskm3`, so MCP must become a usable product surface rather than a theoretical future.

**Independent Test**: Start the server in the first supported host mode, list the governed entrypoints it exposes, invoke at least one governed execution path, and verify the result is produced through runtime-authoritative behavior.

**Acceptance Scenarios**:

1. **Given** a local downstream client connects through the supported MCP host mode, **When** it lists available Traverse entrypoints, **Then** it receives a deterministic machine-readable description of the governed MCP surface.
2. **Given** a client invokes one governed capability or workflow-backed capability, **When** execution begins, **Then** the server delegates to Traverse runtime authority instead of bypassing runtime validation.
3. **Given** a requested execution fails validation or runtime checks, **When** the client inspects the MCP result, **Then** the failure remains structured and explainable.

### User Story 2 - Expose a Rich but Product-Shaped MCP Surface (Priority: P1)

As a downstream app or agent-tool developer, I want a richer MCP surface than bare discovery so that Traverse can support real product consumption rather than only low-level metadata inspection.

**Why this priority**: The UMA Chapter 13 reference app proved that a richer MCP surface is more useful in practice, but Traverse needs that richness without inheriting chapter-specific semantics.

**Independent Test**: Inspect the governed MCP toolset and verify that it includes Traverse-native discovery and execution plus a generic workflow-oriented convenience layer, without hard-coding expedition or chapter-specific scenario vocabulary into the core.

**Acceptance Scenarios**:

1. **Given** a client needs basic product operations, **When** it inspects the core MCP surface, **Then** it can list and describe capabilities and workflows, run them, and render execution artifacts.
2. **Given** a client wants a friendlier workflow-oriented surface, **When** it inspects the convenience layer, **Then** it sees generic entrypoint-oriented operations rather than chapter-specific scenario language.
3. **Given** the expedition example remains the first proving domain, **When** the client uses the convenience layer, **Then** the surface stays product-generic rather than domain-locked.

### User Story 3 - Keep Traverse Runtime Authoritative Behind MCP (Priority: P1)

As a platform steward, I want the MCP server to remain a thin façade over Traverse runtime authority so that validation, state transitions, trace semantics, and execution policy do not drift behind an MCP-specific execution path.

**Why this priority**: The strongest architectural lesson from the UMA runtime model is that the runtime, not the transport surface, stays authoritative.

**Independent Test**: Review one governed MCP execution path and verify that request validation, candidate selection, execution, runtime-state behavior, and terminal results still flow through the Traverse runtime model.

**Acceptance Scenarios**:

1. **Given** a client invokes MCP execution, **When** the server handles the request, **Then** it translates the request into governed Traverse runtime execution rather than directly running business logic.
2. **Given** the runtime emits terminal results and trace outputs, **When** the MCP server returns or renders those artifacts, **Then** it does not redefine their semantics.
3. **Given** future host modes are added, **When** they are proposed, **Then** this slice still preserves runtime-authoritative execution as a non-negotiable boundary.

## Scope

In scope:

- first dedicated Traverse MCP server model
- first supported `stdio` host mode
- Traverse-native MCP discovery and execution operations
- generic workflow-oriented convenience operations
- MCP exposure of governed capability and workflow-backed execution
- WASM-hosted capability and agent participation under Traverse governance
- structured report and trace rendering through MCP-facing operations

Out of scope:

- browser transport or browser-native MCP hosting
- HTTP/SSE or remote network hosting in v0.1
- domain-specific scenario APIs modeled after UMA chapters
- federation across multiple MCP servers
- auth, identity, or multi-tenant deployment policy

## Functional Requirements

- **FR-001**: Traverse MUST define one dedicated MCP server model as a first-class product surface.
- **FR-002**: The first supported MCP host mode MUST be `stdio`.
- **FR-003**: This slice MUST explicitly allow future host modes later without making them part of the first implementation requirement.
- **FR-004**: The MCP server MUST remain a thin façade over Traverse runtime execution.
- **FR-005**: MCP requests that trigger execution MUST be translated into governed Traverse runtime requests rather than directly executing business logic inside the server.
- **FR-006**: The MCP server MUST expose Traverse-native core operations for discovery and description of governed capabilities and workflows.
- **FR-007**: The MCP server MUST expose Traverse-native execution operations for both direct capability execution and workflow-backed capability execution.
- **FR-008**: The MCP server MUST expose a generic workflow-oriented convenience layer for entrypoint-style discovery, description, execution, and execution-artifact rendering.
- **FR-009**: The convenience layer MUST remain product-generic and MUST NOT hard-code chapter- or domain-specific scenario semantics into the core server contract.
- **FR-010**: The first dedicated MCP server MUST support exposure of governed WASM-hosted capabilities or agents through approved manifests and contracts.
- **FR-011**: The MCP server MUST preserve governed validation before execution begins.
- **FR-012**: The MCP server MUST preserve machine-readable runtime failure behavior when validation or execution fails.
- **FR-013**: The MCP server MUST support rendering one structured execution artifact, such as a trace-derived or terminal-result-derived report, through the MCP surface.
- **FR-014**: The MCP server MUST expose enough machine-readable metadata for a downstream client to identify what is invocable and what input shape it expects.
- **FR-015**: The MCP server MUST remain aligned with the downstream-consumer contract and MCP validation slices rather than redefining them separately.
- **FR-016**: The first server implementation MUST be broad enough to feel product-usable and MUST NOT stop at discovery-only metadata inspection.
- **FR-017**: Approved implementation under this slice MUST be decomposable into multiple production-grade tickets with independent validation, not one unreviewable monolith.
- **FR-018**: Approved implementation and validation under this slice MUST be checked against this governing spec before merge.

## Non-Functional Requirements

- **NFR-001 Runtime Authority**: Validation, execution, state transitions, and terminal semantics MUST remain runtime-authoritative behind the MCP façade.
- **NFR-002 Determinism**: Discovery ordering, entrypoint description, execution behavior, and failure reporting MUST remain deterministic for equivalent inputs and registry state.
- **NFR-003 Product Boundary**: The first dedicated MCP server MUST stay product-shaped and reusable, not chapter-shaped or demo-specific.
- **NFR-004 Explainability**: MCP-visible outputs for execution, failure, and rendered artifacts MUST remain explainable without private internal logs.
- **NFR-005 Testability**: The dedicated server model MUST be specifiable and implementable in slices that support deterministic local and CI validation.
- **NFR-006 Portability**: The server model MUST preserve portability of governed WASM-hosted capabilities and agents across supported Traverse hosts, even though the first server host mode is `stdio`.
- **NFR-007 Maintainability**: Core Traverse-native operations and convenience-layer operations MUST remain separable so future surface growth does not pollute the core server boundary.

## Non-Negotiable Quality Gates

- **QG-001**: Traverse MUST NOT present the first dedicated MCP server as authoritative for execution logic; Traverse runtime remains authoritative.
- **QG-002**: The first dedicated MCP server MUST NOT depend on undocumented app-specific glue or private repo semantics.
- **QG-003**: The first dedicated MCP server MUST expose more than discovery-only metadata before it is considered release-usable.
- **QG-004**: The first convenience layer MUST remain generic and MUST NOT encode expedition-only or chapter-only scenario semantics into the core server contract.
- **QG-005**: The first server implementation MUST be decomposed into multiple production-grade slices with explicit validation paths.

## Key Entities

- **Traverse MCP Server**: The dedicated server package that exposes governed Traverse surfaces through MCP.
- **Core MCP Operation**: One Traverse-native discovery, description, or execution operation directly aligned with capabilities, workflows, traces, or terminal results.
- **Convenience MCP Operation**: One generic workflow-oriented MCP operation that presents a friendlier entrypoint-shaped façade over core Traverse semantics.
- **MCP Entrypoint Record**: One machine-readable description of an invocable governed capability or workflow-backed capability exposed through the server.
- **MCP Rendered Artifact**: One structured report or trace-derived representation returned by an MCP operation without redefining the underlying runtime semantics.

## Success Criteria

- **SC-001**: Traverse has one explicit governed model for a dedicated MCP server package rather than only MCP foundation pieces.
- **SC-002**: A downstream client can understand the first server host mode, core operations, convenience operations, and runtime-authoritative boundary from this spec alone.
- **SC-003**: The first implementation can be split into multiple production-grade tickets without guessing the product surface.
- **SC-004**: The Traverse MCP server reuses the useful lessons of UMA Chapter 13 while remaining a product surface for Traverse itself rather than a transplanted chapter artifact.

## Governing Relationship

This specification is governed by:

- `001-foundation-v0-1`
- `006-runtime-request-execution`
- `010-runtime-state-machine`
- `019-downstream-consumer-contract`
- `020-downstream-integration-validation`
- constitution version `1.2.0`

This specification is intended to govern future implementation and validation in:

- dedicated Traverse MCP server packaging
- dedicated MCP discovery and execution operations
- convenience entrypoint-oriented MCP operations
- downstream validation of the dedicated Traverse MCP server

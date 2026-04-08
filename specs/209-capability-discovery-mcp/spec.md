# Feature Specification: Capability Discovery via MCP Surface

**Feature Branch**: `209-capability-discovery-mcp`
**Spec ID**: 015
**Created**: 2026-04-08
**Status**: Draft
**Input**: GitHub issue #209

> **Governance note**: Governed by spec 001-foundation-v0-1 (MCP surface). Depends on #206 (tiered trace), #207 (event broker + catalog), #208 (service_type fields). traverse-mcp transitions from stub to functional implementation.

## Context

MCP (Model Context Protocol) is an open protocol for AI-to-tool communication that provides a standardized interface through which agents and tools can exchange structured requests and responses. Traverse adopts MCP as its primary developer-facing surface because it aligns with the project's goal of making capabilities, event types, and traces introspectable by both AI agents and human developers using the same protocol. By speaking MCP natively, Traverse avoids bespoke query APIs and gains compatibility with the growing ecosystem of MCP-aware agents and tooling.

The traverse-mcp crate is the ECCA developer portal layer equivalent — the single surface through which the internals of Traverse (the capability registry, event catalog, and trace store) are made discoverable. In v0.2.0 this surface is activated in-process, allowing agents to call MCP tools directly without requiring a network transport. This positions Traverse to expose the same six tools over stdio or HTTP in v0.3.0 with minimal additional work, while ensuring the discovery logic is fully tested and spec-compliant before transport concerns are introduced.

## User Scenarios & Testing

### User Story 1 — AI agent discovers capabilities (Priority: P1)

**Acceptance Scenarios**:
1. **Given** capabilities registered in the registry, **When** an agent calls list_capabilities, **Then** receives JSON array with id, name, service_type, permitted_targets, description for each.
2. **Given** a filter service_type: subscribable, **When** list_capabilities is called, **Then** only subscribable capabilities are returned.

### User Story 2 — AI agent discovers event types (Priority: P1)

**Acceptance Scenarios**:
1. **Given** event types registered in EventCatalog, **When** agent calls list_event_types, **Then** receives JSON with owner, version, lifecycle_status, consumer_count per type.

### User Story 3 — AI agent queries execution traces (Priority: P2)

**Acceptance Scenarios**:
1. **Given** traces in TraceStore, **When** agent calls list_traces with capability_id filter, **Then** receives matching public trace entries.
2. **Given** a trace_id, **When** agent calls get_trace with include_private: true, **Then** receives both public and private trace tiers.

### User Story 4 — AI agent gets full capability contract (Priority: P2)

**Acceptance Scenarios**:
1. **Given** a capability_id, **When** agent calls get_capability, **Then** receives full contract JSON.

## Requirements

- **FR-001**: list_capabilities MCP tool accepts optional filter { service_type, permitted_targets } and returns JSON array.
- **FR-002**: get_capability MCP tool accepts capability_id and returns full contract JSON or McpError::NotFound.
- **FR-003**: list_event_types MCP tool returns all EventCatalogEntry records as JSON.
- **FR-004**: get_event_type MCP tool accepts event_type string and returns full EventCatalogEntry or McpError::NotFound.
- **FR-005**: list_traces MCP tool accepts optional { capability_id, since, until } filter and returns public trace entries as JSON.
- **FR-006**: get_trace MCP tool accepts trace_id and include_private: bool; returns public entry always; private tier only if include_private: true.
- **FR-007**: McpContext struct holds Arc<CapabilityRegistry>, Arc<EventCatalog>, Arc<TraceStore> — injected at startup.
- **FR-008**: All MCP tool responses are valid JSON; errors return McpError with code and message.
- **FR-009**: No authentication or authorization in v0.2.0 — local process only.

## Success Criteria

- **SC-001**: All 6 MCP tools exercised in integration tests with in-process MCP calls (no network).
- **SC-002**: list_capabilities filter by service_type works correctly in tests.
- **SC-003**: get_trace with include_private: true returns private tier; without flag returns public only.
- **SC-004**: traverse-mcp crate compiles and all tests pass (not a stub after this spec lands).
- **SC-005**: cargo test passes for all crates.

## Assumptions

- MCP tool dispatch is in-process for v0.2.0 — no network transport required for tests.
- #206, #207, #208 land before #209 — McpContext depends on their types.
- MCP server transport (stdio/HTTP) is deferred to v0.3.0; v0.2.0 uses in-process invocation only.

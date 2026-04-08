# Implementation Plan: Capability Discovery via MCP Surface

**Branch**: `209-capability-discovery-mcp` | **Date**: 2026-04-08 | **Spec**: [spec.md](spec.md)

## Summary

Make traverse-mcp functional by implementing 6 MCP tools (capability discovery, event catalog, trace query) backed by dependency-injected McpContext. All tool invocations are in-process for v0.2.0.

## Technical Context

**Language/Version**: Rust 1.94+
**Primary Dependencies**: traverse-registry, traverse-runtime (EventCatalog, TraceStore), serde_json
**Storage**: All backing stores are in-memory (Arc-wrapped from #206, #207)
**Testing**: Integration tests using in-process MCP tool dispatch
**Target Platform**: All (local only in v0.2.0)
**Project Type**: MCP surface implementation
**Constraints**: No network transport in v0.2.0; no authentication; depends on #206 #207 #208 landing first

## Constitution Check

Implements the MCP surface committed in spec 001-foundation-v0-1. All gates pass.

## Files Touched

```text
crates/traverse-mcp/src/context.rs                  # CREATED — McpContext struct with Arc dependencies
crates/traverse-mcp/src/tools/mod.rs                # CREATED — tool registry
crates/traverse-mcp/src/tools/capabilities.rs       # CREATED — list_capabilities, get_capability
crates/traverse-mcp/src/tools/events.rs             # CREATED (or extended from #207)
crates/traverse-mcp/src/tools/traces.rs             # CREATED (or extended from #206)
crates/traverse-mcp/src/error.rs                    # CREATED — McpError enum
crates/traverse-mcp/src/lib.rs                      # MODIFIED — wire context + tools, remove stub
crates/traverse-mcp/tests/mcp_integration_tests.rs  # CREATED — 6 tool integration tests
```

## Phase 0: Research

- Inspect current traverse-mcp/src/lib.rs stub to understand what exists
- Confirm CapabilityRegistry API (list, get_by_id) from traverse-registry
- Confirm EventCatalog API from #207 spec
- Confirm TraceStore API from #206 spec
- Confirm service_type/permitted_targets field names from #208 spec

## Phase 1: McpContext + McpError

Define:
- `McpContext { registry: Arc<CapabilityRegistry>, event_catalog: Arc<EventCatalog>, trace_store: Arc<TraceStore> }`
- `McpError { NotFound(String), InvalidInput(String) }` with JSON serialization

## Phase 2: Capability tools

In `tools/capabilities.rs`:
- `list_capabilities(ctx, filter: Option<CapabilityFilter>) -> Result<Vec<CapabilitySummary>, McpError>`
  - CapabilityFilter: optional service_type, optional permitted_targets vec
  - CapabilitySummary: id, name, service_type, permitted_targets, description
- `get_capability(ctx, capability_id: &str) -> Result<serde_json::Value, McpError>`
  - Returns full contract serialized as JSON

## Phase 3: Event tools

Coordinate with #207 or implement:
- `list_event_types(ctx) -> Result<Vec<EventCatalogEntry>, McpError>`
- `get_event_type(ctx, event_type: &str) -> Result<EventCatalogEntry, McpError>`

## Phase 4: Trace tools

Coordinate with #206 or implement:
- `list_traces(ctx, filter: Option<TraceFilter>) -> Result<Vec<PublicTraceEntry>, McpError>`
  - TraceFilter: optional capability_id, since, until
- `get_trace(ctx, trace_id: Uuid, include_private: bool) -> Result<TraceResponse, McpError>`
  - TraceResponse: public entry always + optional private entry

## Phase 5: Integration tests

- test_list_capabilities_returns_all
- test_list_capabilities_filter_by_service_type
- test_get_capability_returns_full_contract
- test_list_event_types_returns_catalog
- test_list_traces_filter_by_capability_id
- test_get_trace_with_private_flag

## Implementation Sequence

1. Inspect current traverse-mcp stub
2. Implement McpContext + McpError (Phase 1)
3. Implement capability tools (Phase 2)
4. Wire event tools from #207 (Phase 3)
5. Wire trace tools from #206 (Phase 4)
6. Update lib.rs to expose all tools
7. Write integration tests (Phase 5)
8. cargo test all crates
9. bash scripts/ci/spec_alignment_check.sh
10. Commit + push + open PR declaring spec 001 MCP surface

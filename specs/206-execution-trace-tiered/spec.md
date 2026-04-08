# Feature Specification: Tiered Execution Trace

**Feature Branch**: `206-execution-trace-tiered`
**Spec ID**: 012
**Created**: 2026-04-08
**Status**: Draft
**Input**: GitHub issue #206

> **Governance note**: Extends spec `006-runtime-request-execution` (execution path) and is governed by `001-foundation-v0-1` for the MCP surface. PlacementDecision from spec 010 is part of the public trace tier. No raw inputs or outputs are stored in any tier.

## Context

Traverse needs a two-tier execution trace model that balances observability with data safety:

- **Public trace tier**: placement decision, outcome, capability identity, and timestamp — always emitted, safe to share with any MCP consumer.
- **Private trace tier**: SHA-256 hash of inputs and outputs, resource usage in milliseconds — logged but access-controlled; raw values are never stored.

Both tiers are formatted as CloudEvents (mandatory fields: `source`, `type`, `datacontenttype`, `time`). For v0.2.0 the store is in-memory. The MCP surface (`traverse-mcp`) exposes two tools — `list_traces` and `get_trace` — to query trace entries. Private tier data is returned only when the caller passes an explicit opt-in flag.

## User Scenarios & Testing

### User Story 1 — Public trace entry emitted on capability execution (Priority: P1)

As a platform operator, I want every capability execution to emit a public trace entry in CloudEvents format so that I can audit what ran, where it was placed, and whether it succeeded — without exposing sensitive payload data.

**Why this priority**: The public trace is the minimum observable unit of every execution. Without it no downstream consumer, MCP tool, or audit log has verifiable evidence of what happened.

**Independent Test**: Execute a registered capability via the runtime. Inspect the resulting public trace entry. Verify all CloudEvents fields are present and the entry records capability_id, placement_target, outcome, duration_ms, and timestamp.

**Acceptance Scenarios**:

1. **Given** a registered capability is executed by the runtime, **When** execution completes (success or failure), **Then** a `PublicTraceEntry` is emitted with `capability_id`, `placement_target`, `outcome`, `duration_ms`, and `timestamp` populated.
2. **Given** a `PublicTraceEntry`, **When** its CloudEvents fields are inspected, **Then** `source`, `type`, `datacontenttype`, and `time` are all present and non-empty.
3. **Given** a failed execution, **When** the public trace entry is produced, **Then** `outcome` reflects the failure classification and `duration_ms` reflects the elapsed time up to the failure point.

---

### User Story 2 — Private trace entry records hashed payload and resource usage (Priority: P1)

As a security-conscious platform operator, I want the private trace tier to record inputs/outputs only as SHA-256 hashes and resource usage in milliseconds so that sensitive capability payloads are never persisted in plain text.

**Why this priority**: Storing raw inputs and outputs creates a data-safety risk that must be eliminated structurally, not by policy alone.

**Independent Test**: Execute a capability with a known input. Retrieve the private trace entry. Verify that `inputs_hash` is the correct SHA-256 hex digest of the serialized input, `outputs_hash` is the correct SHA-256 hex digest of the serialized output, and no raw input or output values appear anywhere in the stored trace.

**Acceptance Scenarios**:

1. **Given** a capability execution completes, **When** the private trace entry is stored, **Then** `inputs_hash` equals the SHA-256 hex digest of the canonical serialized input and `outputs_hash` equals the SHA-256 hex digest of the canonical serialized output.
2. **Given** a `PrivateTraceEntry`, **When** all its fields are enumerated, **Then** no field contains the raw input value or the raw output value.
3. **Given** a capability execution, **When** `resource_usage_ms` is recorded, **Then** the value is a non-negative integer representing elapsed wall-clock milliseconds for the execution step.

---

### User Story 3 — MCP client queries execution traces (Priority: P2)

As an MCP client (agent or developer tool), I want to call `list_traces` and `get_trace` on the MCP surface so that I can inspect execution history filtered by capability or time range without requiring direct access to runtime internals.

**Why this priority**: The MCP surface is the intended external query interface for v0.2.0. Without these tools the trace store is write-only from the perspective of any consumer outside the runtime process.

**Independent Test**: Execute several capabilities. Call `list_traces` with a `capability_id` filter. Verify only matching public entries are returned. Call `get_trace` without the opt-in flag and verify only the public tier is returned. Call `get_trace` with `include_private: true` and verify the private tier is also returned.

**Acceptance Scenarios**:

1. **Given** multiple trace entries in the store, **When** `list_traces` is called with a `capability_id` filter, **Then** only entries matching that capability_id are returned, each containing public trace fields only.
2. **Given** a trace_id, **When** `get_trace` is called without `include_private: true`, **Then** only the `PublicTraceEntry` for that trace_id is returned.
3. **Given** a trace_id, **When** `get_trace` is called with `include_private: true`, **Then** both the `PublicTraceEntry` and the `PrivateTraceEntry` for that trace_id are returned.
4. **Given** `list_traces` is called with a time range filter, **Then** only entries whose `timestamp` falls within the range are returned.

---

## Requirements

- **FR-001**: `PublicTraceEntry` MUST contain `capability_id` (String), `placement_target` (PlacementDecision from spec 010), `outcome` (String), `duration_ms` (u64), and `timestamp` (RFC 3339 String). It MUST also carry CloudEvents fields `source` (String), `type` (String), `datacontenttype` (String), and `time` (RFC 3339 String).
- **FR-002**: `PrivateTraceEntry` MUST contain `capability_id` (String), `inputs_hash` (SHA-256 hex String), `outputs_hash` (SHA-256 hex String), and `resource_usage_ms` (u64). It MUST NOT contain raw input or output values.
- **FR-003**: `TraceStore` MUST hold both tiers in memory, keyed by `trace_id` (UUID v4), as `HashMap<Uuid, (PublicTraceEntry, Option<PrivateTraceEntry>)>`.
- **FR-004**: MCP tool `list_traces` MUST return public trace entries, optionally filtered by `capability_id` (exact match) and/or a time range (`from` / `to` in RFC 3339).
- **FR-005**: MCP tool `get_trace` MUST return the `PublicTraceEntry` for a given `trace_id`. It MUST also return the `PrivateTraceEntry` when the caller passes `include_private: true`. When `include_private` is absent or false, the private tier MUST NOT be returned.
- **FR-006**: No raw input or output values MUST be stored in any trace tier at any point. The SHA-256 hash of the canonical serialized form is the only representation permitted.

## Success Criteria

- **SC-001**: `cargo test` passes with no panics, unwraps, or TODOs across the trace module and MCP tools.
- **SC-002**: 100% line coverage for `crates/traverse-runtime/src/trace/` and `crates/traverse-mcp/src/tools/traces.rs`.
- **SC-003**: Tests assert all CloudEvents fields (`source`, `type`, `datacontenttype`, `time`) are non-empty on every `PublicTraceEntry`.
- **SC-004**: Tests assert `inputs_hash` and `outputs_hash` match the expected SHA-256 hex digests for known inputs and outputs.
- **SC-005**: Tests assert that `get_trace` without opt-in returns no private tier data, and with opt-in returns both tiers.
- **SC-006**: Tests assert that no raw input or output value appears in any stored trace entry field.

## Assumptions

- The `PlacementDecision` type is defined in spec 010 (`010-runtime-state-machine`) and is already available in `traverse-runtime`.
- SHA-256 hashing uses the `sha2` crate (already approved for use in the workspace or added as a dependency in this spec's implementation phase).
- The in-memory `TraceStore` is not persisted to disk in v0.2.0; persistence is out of scope.
- CloudEvents `source` is set to `traverse-runtime/<capability_id>` and `type` to `dev.traverse.execution.completed` (or equivalent failure variant) by the runtime at emission time.
- MCP tools in `traverse-mcp` call into `TraceStore` via a shared reference; the concurrency model for v0.2.0 is single-threaded or protected by a `Mutex`.

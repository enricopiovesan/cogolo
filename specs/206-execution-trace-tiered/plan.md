# Implementation Plan: Tiered Execution Trace

**Branch**: `206-execution-trace-tiered` | **Date**: 2026-04-08 | **Spec**: [spec.md](spec.md)

## Summary

Add a two-tier execution trace model to `traverse-runtime` (public + private, CloudEvents-formatted, SHA-256 hashed payloads) and expose it via `list_traces` and `get_trace` MCP tools in `traverse-mcp`. The `TraceStore` is in-memory for v0.2.0. No raw inputs or outputs are stored in any tier.

## Technical Context

**Language/Version**: Rust 1.94+
**Primary Dependencies**: `uuid` (v4), `sha2`, `serde`, `chrono` (RFC 3339 timestamps), existing `traverse-runtime` and `traverse-mcp` crates
**Storage**: In-memory `HashMap<Uuid, (PublicTraceEntry, Option<PrivateTraceEntry>)>`
**Testing**: `cargo test` — 100% line coverage on trace module and MCP tools
**Target Platform**: Native (v0.2.0); WASM compatibility not required for this slice
**Constraints**: No `unsafe`, no `unwrap()`, no `panic!()`, no TODO in code; no raw inputs or outputs stored anywhere

## Constitution Check

Extends approved spec `006-runtime-request-execution`. MCP surface governed by `001-foundation-v0-1`. All spec-alignment gate requirements apply. Must be declared in PR body under both spec IDs.

## Project Structure

### Documentation (this feature)

```text
specs/206-execution-trace-tiered/
├── spec.md
└── plan.md              # This file
```

### Files touched

```text
crates/traverse-runtime/src/trace/mod.rs         # CREATED — trace module root; re-exports PublicTraceEntry, PrivateTraceEntry, TraceStore
crates/traverse-runtime/src/trace/public.rs      # CREATED — PublicTraceEntry struct with CloudEvents fields
crates/traverse-runtime/src/trace/private.rs     # CREATED — PrivateTraceEntry struct (inputs_hash, outputs_hash, resource_usage_ms)
crates/traverse-runtime/src/trace/store.rs       # CREATED — TraceStore (HashMap<Uuid, (PublicTraceEntry, Option<PrivateTraceEntry>)>)
crates/traverse-mcp/src/tools/traces.rs          # CREATED — list_traces + get_trace MCP tool handlers
crates/traverse-runtime/tests/trace_tests.rs     # CREATED — integration tests for both tiers and MCP tool output
```

## Phase 0: Research

- Confirm `PlacementDecision` type path in `crates/traverse-runtime/src/` (expected from spec 010 work).
- Confirm `sha2` crate availability in `Cargo.toml` workspace; add if absent.
- Confirm `uuid` crate with `v4` feature is in workspace; add if absent.
- Confirm `chrono` or equivalent RFC 3339 timestamp source is available.
- Confirm `traverse-mcp` tool registration pattern (how existing tools are wired).

## Phase 1: Define PublicTraceEntry + PrivateTraceEntry with CloudEvents fields

**Files**: `public.rs`, `private.rs`, `mod.rs`

`PublicTraceEntry`:
- CloudEvents mandatory fields: `source: String`, `type_: String`, `datacontenttype: String`, `time: String` (RFC 3339)
- Domain fields: `capability_id: String`, `placement_target: PlacementDecision`, `outcome: String`, `duration_ms: u64`, `timestamp: String` (RFC 3339, same as `time`)

`PrivateTraceEntry`:
- `capability_id: String`, `inputs_hash: String` (SHA-256 hex), `outputs_hash: String` (SHA-256 hex), `resource_usage_ms: u64`
- Constructor accepts raw bytes / serialized form; hashes internally; never stores raw values.

Both types derive `serde::Serialize` / `serde::Deserialize`.

## Phase 2: Implement TraceStore

**File**: `store.rs`

- `TraceStore` wraps `HashMap<Uuid, (PublicTraceEntry, Option<PrivateTraceEntry>)>`
- `insert(trace_id: Uuid, public: PublicTraceEntry, private: Option<PrivateTraceEntry>)`
- `get(trace_id: &Uuid) -> Option<&(PublicTraceEntry, Option<PrivateTraceEntry>)>`
- `list(capability_id: Option<&str>, from: Option<&str>, to: Option<&str>) -> Vec<&PublicTraceEntry>` — filters by capability_id (exact) and RFC 3339 time range on `timestamp`

## Phase 3: Integrate trace emission into the runtime execution path

- In the capability execution path (`traverse-runtime`), after execution completes (success or failure):
  1. Generate a `Uuid::new_v4()` as `trace_id`.
  2. Compute `inputs_hash` and `outputs_hash` via SHA-256 of the canonical serialized input/output bytes.
  3. Construct `PublicTraceEntry` with CloudEvents fields set (`source = traverse-runtime/<capability_id>`, `type_ = dev.traverse.execution.completed` or `dev.traverse.execution.failed`).
  4. Construct `PrivateTraceEntry` from hashes and `resource_usage_ms`.
  5. Insert both into `TraceStore`.

## Phase 4: Implement MCP tools list_traces + get_trace

**File**: `crates/traverse-mcp/src/tools/traces.rs`

`list_traces`:
- Params: `capability_id: Option<String>`, `from: Option<String>`, `to: Option<String>`
- Delegates to `TraceStore::list(...)` and serializes results as JSON array of public entries.

`get_trace`:
- Params: `trace_id: String`, `include_private: Option<bool>`
- Returns `PublicTraceEntry` always; includes `PrivateTraceEntry` only when `include_private == Some(true)`.
- Returns structured error when `trace_id` is not found.

Register both tools in the `traverse-mcp` tool registry.

## Phase 5: Tests

**File**: `crates/traverse-runtime/tests/trace_tests.rs`

Test cases:
1. Execute a capability → assert `PublicTraceEntry` emitted with all CloudEvents fields non-empty.
2. Execute a capability with known input → assert `inputs_hash` matches SHA-256 hex of that input.
3. Execute a capability with known output → assert `outputs_hash` matches SHA-256 hex of that output.
4. Assert no raw input or output value appears in any field of `PublicTraceEntry` or `PrivateTraceEntry`.
5. `list_traces` with `capability_id` filter → only matching entries returned.
6. `list_traces` with time range → only entries within range returned.
7. `get_trace` without `include_private` → only public tier returned.
8. `get_trace` with `include_private: true` → both tiers returned.
9. Failed execution → `PublicTraceEntry.outcome` reflects failure; `duration_ms` is non-zero.
10. Unknown `trace_id` in `get_trace` → structured error returned, no panic.

## Verification

```bash
cargo build
cargo test
bash scripts/ci/spec_alignment_check.sh
```

All three commands must pass with zero errors before the PR is opened.

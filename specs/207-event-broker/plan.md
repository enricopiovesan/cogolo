# Implementation Plan: In-Process Event Broker with ECCA-Aligned Catalog

**Branch**: `207-event-broker` | **Date**: 2026-04-08 | **Spec**: [spec.md](spec.md)

## Summary

Add an in-process EventBroker and ECCA-aligned EventCatalog to `traverse-runtime`, expose the catalog and agent subscription via two MCP tools in `traverse-mcp`, and cover everything with integration tests. All events carry CloudEvents fields + governance metadata; lifecycle enforcement (no publishing deprecated or draft types) is handled at broker publish time.

## Technical Context

**Language/Version**: Rust 1.94+
**Primary Dependencies**: `uuid` (v4 feature), `chrono` (RFC 3339 timestamps), `serde` / `serde_json`, existing `traverse-runtime` and `traverse-mcp` crates
**Storage**: In-memory `HashMap<String, Vec<SubscriberFn>>` for subscriptions; `HashMap<String, EventCatalogEntry>` for the catalog — no persistence in v0.2.0
**Delivery model**: Synchronous, in-process, insertion-order deterministic
**Testing**: `cargo test` — all test cases in `crates/traverse-runtime/tests/event_broker_tests.rs`
**Target Platform**: Native (v0.2.0); WASM compatibility not required for this slice
**Constraints**: No `unsafe`, no `unwrap()`, no `panic!()`, no TODO in code; no raw event data stored in the catalog

## Constitution Check

Governed by approved spec `003-event-contracts` (event contract format). Extends approved spec `006-runtime-request-execution`. MCP surface governed by `001-foundation-v0-1`. All spec-alignment gate requirements apply. PR body must reference spec IDs 003, 006, and 013. Depends on issue #208 (`subscribable` service_type); the broker operates on event type strings and does not enforce service_type directly.

## Files Touched

```text
crates/traverse-runtime/src/events/mod.rs           # CREATED — events module root; re-exports TraverseEvent, EventMetadata, EventCatalogEntry, EventBroker trait, InProcessBroker, EventCatalog, EventError
crates/traverse-runtime/src/events/types.rs         # CREATED — TraverseEvent (CloudEvents fields + EventMetadata), EventMetadata, EventCatalogEntry, LifecycleStatus, SubscriptionId
crates/traverse-runtime/src/events/broker.rs        # CREATED — EventBroker trait + InProcessBroker implementation (HashMap<EventType, Vec<SubscriberFn>>)
crates/traverse-runtime/src/events/catalog.rs       # CREATED — EventCatalog (HashMap<String, EventCatalogEntry>); register/list/get; lifecycle enforcement called by broker
crates/traverse-mcp/src/tools/events.rs             # CREATED — list_event_types + subscribe MCP tool handlers
crates/traverse-runtime/tests/event_broker_tests.rs # CREATED — integration tests: publish→receive, lifecycle rejection, catalog query, MCP output shape
```

## Phase 0: Research

- Confirm `uuid` crate with `v4` feature is in `Cargo.toml` workspace; add if absent.
- Confirm `chrono` or equivalent RFC 3339 timestamp source is available; add if absent.
- Confirm `traverse-mcp` tool registration pattern (how existing tools in `crates/traverse-mcp/src/tools/` are registered and wired) — reference `traces.rs` from spec 012 if already merged, otherwise inspect existing tool files.
- Confirm whether `traverse-runtime/src/lib.rs` exposes sub-modules via `pub mod`; determine how to wire in the new `events` module.
- Confirm no existing `events` module exists in `traverse-runtime/src/` that would conflict.

## Phase 1: Types and Traits

**Files**: `types.rs`, `mod.rs` (skeleton)

Define all shared types first so later phases have stable imports:

`LifecycleStatus`:
- Enum: `Draft`, `Active`, `Deprecated`
- Derives: `Debug`, `Clone`, `PartialEq`, `serde::Serialize`, `serde::Deserialize`

`EventMetadata`:
- Fields: `owner: String` (capability_id), `version: String` (semver), `lifecycle_status: LifecycleStatus`
- Derives: `Debug`, `Clone`, `serde::Serialize`, `serde::Deserialize`

`TraverseEvent`:
- CloudEvents fields: `source: String`, `event_type: String` (maps to CloudEvents `type`), `id: String` (UUID v4), `time: String` (RFC 3339), `datacontenttype: String`, `data: serde_json::Value`
- Governance envelope: `metadata: EventMetadata`
- Derives: `Debug`, `Clone`, `serde::Serialize`, `serde::Deserialize`
- Constructor `TraverseEvent::new(...)` generates `id` via `Uuid::new_v4()` and `time` via `chrono::Utc::now()`

`EventCatalogEntry`:
- Fields: `event_type: String`, `owner: String`, `version: String`, `lifecycle_status: LifecycleStatus`, `consumer_count: usize`
- Derives: `Debug`, `Clone`, `serde::Serialize`, `serde::Deserialize`
- No payload schema or raw data fields

`SubscriptionId`:
- Newtype over `u64`; derives `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`

`EventError`:
- Enum variants: `UnregisteredEventType(String)`, `LifecycleViolation { event_type: String, status: LifecycleStatus }`, `SubscriptionNotFound(SubscriptionId)`, `CatalogEntryAlreadyExists(String)`
- Derives: `Debug`; implements `std::error::Error` and `std::fmt::Display` — no `unwrap`, no panic

`EventBroker` trait:
- `fn publish(&self, event: TraverseEvent) -> Result<(), EventError>`
- `fn subscribe(&self, event_type: &str, handler: Box<dyn Fn(&TraverseEvent) + Send + Sync>) -> Result<SubscriptionId, EventError>`
- `fn unsubscribe(&self, id: SubscriptionId) -> Result<(), EventError>`

## Phase 2: InProcessBroker Implementation

**File**: `broker.rs`

`InProcessBroker`:
- Internal state: `subscriptions: HashMap<String, Vec<(SubscriptionId, Box<dyn Fn(&TraverseEvent) + Send + Sync>)>>`
- Internal state: `next_id: u64` (monotonically incrementing `SubscriptionId` source)
- Holds a shared reference to `EventCatalog` (e.g., `Arc<Mutex<EventCatalog>>` or equivalent)
- Wrapped in `Mutex` for interior mutability; no `unwrap` — use `map_err` / `?` on lock results

`publish` implementation:
1. Lock catalog; look up entry by `event_type` — return `Err(EventError::UnregisteredEventType)` if absent.
2. Check `lifecycle_status` — return `Err(EventError::LifecycleViolation)` if `Draft` or `Deprecated`.
3. Populate `event.metadata` from the catalog entry (override caller-supplied values for `owner`, `version`, `lifecycle_status`).
4. Look up subscribers for `event_type`; call each handler in insertion order. No short-circuit on handler errors (handlers are infallible in v0.2.0).
5. Return `Ok(())`.

`subscribe` implementation:
1. Look up catalog entry — return `Err(EventError::UnregisteredEventType)` if absent (subscribers can only register for known event types).
2. Generate next `SubscriptionId`.
3. Push `(id, handler)` to the subscriber list for `event_type`.
4. Increment `consumer_count` on the catalog entry.
5. Return `Ok(subscription_id)`.

`unsubscribe` implementation:
1. Search all event types for the given `SubscriptionId`.
2. If found, remove it and decrement `consumer_count` on the catalog entry.
3. If not found, return `Err(EventError::SubscriptionNotFound)`.

## Phase 3: EventCatalog

**File**: `catalog.rs`

`EventCatalog`:
- Internal state: `entries: HashMap<String, EventCatalogEntry>` keyed by `event_type`
- No raw event data stored — only `EventCatalogEntry` structs

Methods:
- `register(entry: EventCatalogEntry) -> Result<(), EventError>` — returns `Err(EventError::CatalogEntryAlreadyExists)` if the type is already registered. To update an entry, the caller must use a dedicated `update` method (or re-register after removal — TBD in implementation; at minimum, re-registration replaces the existing entry with a warning log in v0.2.0, flagged as a `CatalogEntryAlreadyExists` error so callers are explicit).
- `list() -> Vec<&EventCatalogEntry>` — returns all entries; order is insertion order (preserve with `IndexMap` if determinism matters, else `HashMap` with sorted output in MCP layer).
- `get(event_type: &str) -> Option<&EventCatalogEntry>` — single lookup.
- `get_mut(event_type: &str) -> Option<&mut EventCatalogEntry>` — used internally by broker to increment/decrement `consumer_count`.

## Phase 4: MCP Tools

**File**: `crates/traverse-mcp/src/tools/events.rs`

`list_event_types`:
- Params: none (returns full catalog)
- Acquires shared catalog reference; calls `EventCatalog::list()`
- Serializes result as JSON array of `EventCatalogEntry` objects sorted by `event_type` for deterministic output
- Returns structured JSON; never panics on empty catalog (returns `[]`)

`subscribe`:
- Params: `event_type: String`
- Calls `InProcessBroker::subscribe(event_type, handler)` where handler is a no-op stub for MCP-side subscriptions in v0.2.0 (actual agent callback delivery is out of scope)
- Returns `{ "subscription_id": <u64> }` on success
- Returns structured error JSON on `EventError::UnregisteredEventType`

Register both tools in the `traverse-mcp` tool registry following the same wiring pattern as existing tools.

## Phase 5: Tests

**File**: `crates/traverse-runtime/tests/event_broker_tests.rs`

Test cases (all must pass without `unwrap` or `panic`):

1. **publish_delivers_to_subscriber**: Register an active event type. Subscribe a handler that records received events. Publish one event. Assert handler called once with all CloudEvents fields non-empty.
2. **publish_delivers_to_multiple_subscribers_in_insertion_order**: Register two handlers. Publish one event. Assert both called; assert order matches registration order.
3. **publish_populates_governance_metadata_from_catalog**: Register an event type with specific owner/version. Publish with incorrect metadata in caller payload. Assert received event carries catalog-sourced owner/version/lifecycle_status.
4. **publish_rejects_deprecated_event_type**: Register event type with `lifecycle_status = Deprecated`. Assert `publish` returns `Err(EventError::LifecycleViolation)`. Assert no handler invoked.
5. **publish_rejects_draft_event_type**: Register event type with `lifecycle_status = Draft`. Assert `publish` returns `Err(EventError::LifecycleViolation)`. Assert no handler invoked.
6. **publish_rejects_unregistered_event_type**: Publish for a type not in the catalog. Assert `Err(EventError::UnregisteredEventType)`.
7. **subscribe_increments_consumer_count**: Subscribe to an event type. Assert `EventCatalog::get` shows `consumer_count = 1`. Subscribe again. Assert `consumer_count = 2`.
8. **unsubscribe_decrements_consumer_count**: Subscribe, then unsubscribe. Assert `consumer_count` returns to 0 and the handler is no longer invoked on publish.
9. **list_event_types_mcp_output**: Register three event types (one deprecated, two active). Call the `list_event_types` tool handler. Assert JSON output contains all three entries with correct fields. Assert no raw data payload appears in any entry.
10. **subscribe_mcp_tool_returns_subscription_id**: Call the `subscribe` MCP tool for a registered active event type. Assert response contains a valid `subscription_id` integer.

## Implementation Sequence

1. Phase 0: Research (workspace deps, existing module structure, MCP wiring pattern)
2. Phase 1: `types.rs` — all shared types and `EventBroker` trait (no logic yet)
3. Phase 1: `mod.rs` skeleton — wire `pub mod types; pub mod broker; pub mod catalog;` and re-exports
4. Phase 3: `catalog.rs` — `EventCatalog` impl (no broker dependency)
5. Phase 2: `broker.rs` — `InProcessBroker` impl (depends on `EventCatalog`)
6. Phase 4: `crates/traverse-mcp/src/tools/events.rs` — `list_event_types` + `subscribe`
7. Phase 5: `event_broker_tests.rs` — all 10 test cases
8. Wire `events` module into `traverse-runtime/src/lib.rs`; wire MCP tools into tool registry
9. `cargo build` — fix any compile errors
10. `cargo test` — all tests must pass
11. `bash scripts/ci/spec_alignment_check.sh` — CI gate must pass

## Verification

```bash
cargo build
cargo test
bash scripts/ci/spec_alignment_check.sh
```

All three commands must pass with zero errors before the PR is opened.

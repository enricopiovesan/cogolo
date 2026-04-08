# Implementation Plan: Runtime Placement Router

**Branch**: `210-runtime-placement-router` | **Date**: 2026-04-08 | **Spec**: [spec.md](spec.md)

## Summary

Wire the v0.2.0 execution pipeline into a single PlacementRouter struct: evaluator → executor selection → run → trace → event publish. Replace any ad-hoc execution wiring in traverse-runtime.

## Technical Context

**Language/Version**: Rust 1.94+
**Primary Dependencies**: traverse-contracts, traverse-registry, and all sub-systems from #204–#208
**Storage**: Delegates to TraceStore (in-memory)
**Testing**: Integration tests exercising the full pipeline in-process
**Target Platform**: All
**Project Type**: Integration / orchestration layer
**Constraints**: No global state; all dependencies injected; depends on #204 #205 #206 #207 #208

## Constitution Check

Extends spec 006-runtime-request-execution as declared. All CI gates pass.

## Files Touched

```text
crates/traverse-runtime/src/router.rs               # CREATED — PlacementRouter
crates/traverse-runtime/src/executor/registry.rs    # CREATED — CapabilityExecutorRegistry
crates/traverse-runtime/src/lib.rs                  # MODIFIED — expose PlacementRouter as primary API
crates/traverse-runtime/tests/router_integration.rs # CREATED — end-to-end integration tests
```

## Phase 0: Research

- Map current execute() entry point in traverse-runtime — identify what to replace
- Confirm artifact_type field exists on CapabilityContract; if absent, add ArtifactType enum (Native | Wasm)
- Confirm PlacementConstraintEvaluator, TraceStore, EventBroker, NativeExecutor, WasmExecutor APIs from preceding specs
- Identify any global state to eliminate

## Phase 1: CapabilityExecutorRegistry

HashMap<ArtifactType, Box<dyn CapabilityExecutor + Send + Sync>> with register() and get() methods.

## Phase 2: PlacementRouter

```rust
pub struct PlacementRouter {
    evaluator: PlacementConstraintEvaluator,
    executors: CapabilityExecutorRegistry,
    trace_store: Arc<TraceStore>,
    event_broker: Arc<dyn EventBroker>,
}

impl PlacementRouter {
    pub fn execute(&self, request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError>;
}
```

execute() steps: (1) evaluate placement → PlacementDecision or RuntimeError::PlacementFailed, (2) select executor by artifact_type, (3) run capability, (4) write trace entries, (5) if Subscribable, publish events.

## Phase 3: Wire into lib.rs

Make PlacementRouter the exported execute surface. Deprecate/remove any prior ad-hoc wiring.

## Phase 4: Integration tests

- test_native_end_to_end: register native cap → execute → assert trace written + PlacementDecision present
- test_wasm_end_to_end: register WASM cap (synthetic fixture) → execute → assert WasmExecutor used + trace written
- test_placement_failure_no_trace: permitted_targets mismatch → RuntimeError::PlacementFailed + TraceStore empty
- test_subscribable_events_published: subscribable cap → execute → assert EventBroker received events

## Implementation Sequence

1. Read current traverse-runtime execute path
2. Add ArtifactType enum if missing
3. Implement CapabilityExecutorRegistry (Phase 1)
4. Implement PlacementRouter (Phase 2)
5. Wire PlacementRouter into lib.rs (Phase 3)
6. Write integration tests (Phase 4)
7. cargo test all crates
8. bash scripts/ci/spec_alignment_check.sh
9. Commit + push + open PR declaring spec 006 extension

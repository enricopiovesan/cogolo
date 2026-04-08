# Feature Specification: Runtime Placement Router

**Feature Branch**: `210-runtime-placement-router`
**Spec ID**: 016
**Created**: 2026-04-08
**Status**: Draft
**Input**: GitHub issue #210

> **Governance note**: Extends spec 006-runtime-request-execution. Integrates #204 (placement evaluator), #205 (executor adapter), #206 (trace), #207 (event broker). This is the final integration step for the v0.2.0 execution path.

## Context

The v0.2.0 execution pipeline introduces placement-aware routing as the authoritative path for all capability execution in traverse-runtime. Prior to this spec, execution wiring was ad-hoc — callers directly invoked executor logic without a unified orchestration layer. The PlacementRouter closes this gap: it accepts a RuntimeRequest, delegates placement resolution to PlacementConstraintEvaluator, selects the correct executor based on artifact type, records a two-tier trace entry, and conditionally publishes events through the EventBroker. Every sub-system concern flows through this single entry point, making the execution path explicit, auditable, and governed by spec.

Structuring the router around dependency injection — rather than global state or hard-coded sub-system references — is what makes the system both testable and replaceable. Each collaborator (PlacementConstraintEvaluator, CapabilityExecutorRegistry, TraceStore, EventBroker) is passed in at construction time as a trait object or Arc-wrapped concrete type. Integration tests can substitute any collaborator with a deterministic in-process fake without touching production code. Future implementations (e.g., a remote PlacementConstraintEvaluator or a durable TraceStore) can be swapped in transparently, because PlacementRouter depends on interfaces, not implementations.

## User Scenarios & Testing

### User Story 1 — Native capability executes end-to-end with trace (Priority: P1)
**Acceptance Scenarios**:
1. **Given** a registered native capability, **When** execute() is called, **Then** PlacementDecision is recorded, NativeExecutor runs it, public + private trace entries are written to TraceStore.

### User Story 2 — WASM capability routes to WasmExecutor (Priority: P1)
**Acceptance Scenarios**:
1. **Given** a registered WASM capability (artifact_type: wasm), **When** execute() is called, **Then** WasmExecutor is selected, capability runs, trace is written.

### User Story 3 — Placement constraint blocks ineligible target (Priority: P2)
**Acceptance Scenarios**:
1. **Given** a capability with permitted_targets: [Cloud] and runtime only has a Browser target, **When** execute() is called, **Then** RuntimeError::PlacementFailed is returned and no trace entry is written.

### User Story 4 — Subscribable capability emits events after execution (Priority: P2)
**Acceptance Scenarios**:
1. **Given** a subscribable capability that emits events, **When** execute() succeeds, **Then** emitted events are published to EventBroker.

## Requirements

- **FR-001**: PlacementRouter struct holds injected references: PlacementConstraintEvaluator, CapabilityExecutorRegistry (maps artifact_type → CapabilityExecutor impl), TraceStore, EventBroker.
- **FR-002**: execute() calls PlacementConstraintEvaluator first; on PlacementError returns RuntimeError::PlacementFailed without executing.
- **FR-003**: execute() selects executor by capability artifact_type (native → NativeExecutor, wasm → WasmExecutor).
- **FR-004**: execute() writes PublicTraceEntry + PrivateTraceEntry to TraceStore after execution.
- **FR-005**: execute() publishes emitted events to EventBroker if service_type == Subscribable.
- **FR-006**: PlacementRouter is the single public entry point for all capability execution in traverse-runtime.
- **FR-007**: All sub-system dependencies are injected — no global state.

## Success Criteria

- **SC-001**: End-to-end integration test: native capability executes, trace written, PlacementDecision in public entry.
- **SC-002**: End-to-end integration test: WASM capability (synthetic fixture) executes via WasmExecutor.
- **SC-003**: Placement failure returns RuntimeError::PlacementFailed, no trace written.
- **SC-004**: cargo test passes for all crates.
- **SC-005**: No regression in existing traverse-runtime tests.

## Assumptions

- #204, #205, #206, #207, #208 all land before #210.
- artifact_type field exists on CapabilityContract or CapabilityRegistration — if not, add as enum (Native | Wasm) in this spec.
- PlacementRouter replaces any ad-hoc execution wiring currently in traverse-runtime.

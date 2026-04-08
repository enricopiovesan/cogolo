# Feature Specification: Expedition Example Domain — WASM Port

**Feature Branch**: `211-expedition-wasm-port`
**Spec ID**: 027
**Created**: 2026-04-08
**Status**: Draft
**Input**: GitHub issue #211

> **Governance note**: Extends spec 008-expedition-example-domain and spec 009-expedition-example-artifacts. Validates the WasmExecutor from spec 011 (#205) and PlacementRouter from spec 016 (#210) end-to-end. Depends on #205 and #210 landing first.

## Context

The expedition example domain, defined in specs 008 and 009, has served as Traverse's canonical demonstration of capability contracts and runtime execution. Because it encodes a realistic, self-contained business workflow with well-defined JSON input/output boundaries, it is the ideal first candidate for compilation to a WASM binary: its logic is pure, its I/O is already JSON-serialized, and its contract metadata is already governed. Choosing expedition as the first WASM capability minimizes the risk of uncovering unknown coupling between business logic and host OS primitives, and gives the team a high-confidence integration signal at the start of the v0.2.0 WASM work.

Compiling the expedition domain to a `wasm32-wasi` binary and executing it through the `WasmExecutor` (spec 011, #205) proves the full v0.2.0 WASM portability story end-to-end. When a `PlacementRouter` call successfully routes an expedition request to `WasmExecutor`, loads the `.wasm` artifact, deserializes the response, and emits a correct execution trace, every layer of the WASM stack — contract metadata, artifact resolution, checksum validation, WASI stdio I/O, and response unmarshaling — has been exercised in one path. This makes the expedition WASM port the definitive integration validation for v0.2.0: if it passes, the WASM execution story is proven.

## User Scenarios & Testing

### User Story 1 — Expedition capability compiles to WASM (Priority: P1)
**Acceptance Scenarios**:
1. **Given** the expedition business logic crate, **When** compiled with `cargo build --target wasm32-wasi`, **Then** a valid .wasm binary is produced with no linker errors.

### User Story 2 — Expedition WASM capability executes via WasmExecutor (Priority: P1)
**Acceptance Scenarios**:
1. **Given** the expedition .wasm binary registered in the capability registry, **When** PlacementRouter.execute() is called with a valid expedition request, **Then** WasmExecutor runs it and returns a valid expedition response.

### User Story 3 — Expedition WASM capability has correct contract metadata (Priority: P2)
**Acceptance Scenarios**:
1. **Given** the expedition capability contract, **When** queried via list_capabilities MCP tool, **Then** service_type: Stateless, permitted_targets: [Cloud, Edge, Device], artifact_type: Wasm are returned.

## Requirements

- **FR-001**: A new crate (traverse-expedition-wasm or expedition/wasm/) contains the expedition business logic compiled to wasm32-wasi target.
- **FR-002**: The expedition capability contract declares artifact_type: Wasm, service_type: Stateless, permitted_targets: [Cloud, Edge, Device].
- **FR-003**: Input and output are JSON-serialized, passed via WASI stdio.
- **FR-004**: The .wasm binary is checked into contracts/expedition/ or a build artifact path declared in the contract.
- **FR-005**: WasmExecutor loads, validates checksum, and runs the expedition binary end-to-end.
- **FR-006**: The expedition WASM capability is registered in the capability registry at runtime startup.
- **FR-007**: All existing expedition native tests continue to pass (no regression).

## Success Criteria

- **SC-001**: `cargo build --target wasm32-wasi -p traverse-expedition-wasm` produces a .wasm binary.
- **SC-002**: End-to-end integration test: PlacementRouter routes expedition request to WasmExecutor, returns valid response.
- **SC-003**: Expedition contract returns correct service_type, permitted_targets, artifact_type via MCP list_capabilities.
- **SC-004**: cargo test passes for all crates.
- **SC-005**: No regression in existing expedition native tests.

## Assumptions

- wasm32-wasi target is installed in the Rust toolchain (requires `rustup target add wasm32-wasi`).
- WASI stdio is sufficient for JSON input/output in v0.2.0 (no WASI filesystem or network needed).
- The .wasm binary can be committed to the repo or produced at test time via a build script.
- Existing expedition business logic is pure enough (no OS threads, no tokio) to compile to wasm32-wasi cleanly.

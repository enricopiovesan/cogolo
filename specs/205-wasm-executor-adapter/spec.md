# Feature Specification: WASM Executor Adapter

**Feature Branch**: `205-wasm-executor-adapter`
**Spec ID**: 025
**Created**: 2026-04-08
**Status**: Draft
**Input**: GitHub issue #205

> **Governance note**: Extends spec 006-runtime-request-execution. The `local` placement defined in spec 006 is now backed by a pluggable `CapabilityExecutor` trait with two implementations: `NativeExecutor` (existing Rust path) and `WasmExecutor` (Wasmtime-backed). This spec does not change the runtime request boundary, state machine, or trace contract — it refactors the execution layer beneath selection. The expedition example domain (#211) is the first real WASM capability and validates the WasmExecutor end-to-end.

## Context

Spec 006 defines a `local` placement execution path but does not prescribe how that execution is dispatched. All current capabilities are Rust-native and are called directly within the runtime. To support WASM capabilities, the runtime needs an abstraction that lets the selection logic remain stable while the execution mechanism varies per capability type.

The chosen WASM runtime is **Wasmtime**: Rust-native, full WASI support, and detailed introspection. This aligns with the project's Rust-centered architecture. WasmEdge was considered and rejected because Wasmtime's Rust integration is tighter and its sandboxing model maps cleanly to per-contract WASI scoping.

The `CapabilityExecutor` trait is introduced in `traverse-runtime` as the single dispatch boundary. `NativeExecutor` wraps the existing Rust execution path with no behavioral change. `WasmExecutor` loads a `.wasm` binary from the capability registry path, validates its checksum against the registered artifact metadata, instantiates it in a Wasmtime sandbox with WASI capabilities scoped to what the capability contract declares, and returns a structured execution result.

WASI capabilities are granted per contract and per execution — no ambient authority is carried between executions or inherited from the host process.

## User Scenarios & Testing

### User Story 1 — Native capability executes via NativeExecutor (Priority: P1)

As a platform developer, I want all existing Rust-native capabilities to continue executing correctly after the executor refactor so that the introduction of WASM support does not regress any existing behavior.

**Why this priority**: The NativeExecutor refactor is a prerequisite for the WasmExecutor. Any regression here invalidates the entire runtime slice.

**Independent Test**: Run the full existing capability test suite after the NativeExecutor refactor. All tests must pass without modification to test logic.

**Acceptance Scenarios**:

1. **Given** a registered native capability and a valid runtime request, **When** the runtime selects the capability and dispatches to `NativeExecutor`, **Then** execution completes and returns the same result as before the refactor.
2. **Given** a native capability that returns invalid output, **When** `NativeExecutor` returns, **Then** the runtime applies output contract validation and surfaces a failure result — identical to pre-refactor behavior.
3. **Given** the runtime is initialized, **When** a native capability is dispatched, **Then** no WASM engine, store, or linker is instantiated — `NativeExecutor` does not pay the WasmExecutor initialization cost.

### User Story 2 — WASM capability loads and executes via WasmExecutor (Priority: P1)

As a platform developer, I want to register a WASM capability and have the runtime load, sandbox, and execute it via Wasmtime so that Traverse supports portable, isolated capability implementations beyond Rust-native code.

**Why this priority**: This is the primary deliverable of this spec. Without a working WasmExecutor, the WASM execution path does not exist.

**Independent Test**: Register a minimal synthetic `.wasm` fixture capability. Submit a valid runtime request targeting it. Verify the runtime dispatches to `WasmExecutor`, executes the WASM binary in a Wasmtime sandbox, and returns a structured result including trace evidence of WASM dispatch.

**Acceptance Scenarios**:

1. **Given** a registered WASM capability with a valid `.wasm` binary and matching checksum, **When** the runtime selects it, **Then** `WasmExecutor` loads the binary, instantiates it in Wasmtime with scoped WASI, executes it, and returns a successful result.
2. **Given** a successful WASM execution, **When** the trace is inspected, **Then** it records the executor type (`wasm`), the WASM binary path, and the Wasmtime instantiation metadata.
3. **Given** WASI capabilities declared in the capability contract, **When** `WasmExecutor` instantiates the module, **Then** only those declared WASI capabilities are granted — no ambient host access is permitted.
4. **Given** a WASM capability execution completes, **When** the result is returned, **Then** the runtime applies the same output contract validation as for native capabilities before returning success.

### User Story 3 — Checksum validation rejects tampered binary (Priority: P2)

As a platform developer, I want the runtime to reject a WASM binary whose checksum does not match the registered artifact metadata so that Traverse does not execute untrusted or corrupted WASM modules.

**Why this priority**: Checksum validation is a security invariant for the WASM execution path. A tampered binary must never reach Wasmtime instantiation.

**Independent Test**: Register a WASM capability with a valid checksum. Replace the binary on disk with a tampered version. Submit a runtime request and verify the runtime rejects execution with a checksum mismatch failure before Wasmtime is invoked.

**Acceptance Scenarios**:

1. **Given** a WASM capability whose binary checksum does not match the registered artifact metadata, **When** `WasmExecutor` validates the binary before instantiation, **Then** the runtime returns a failure result with `checksum_mismatch` classification and does not invoke Wasmtime.
2. **Given** a checksum mismatch failure, **When** the trace is inspected, **Then** it records the expected checksum, the observed checksum, and the binary path — with no execution evidence, confirming Wasmtime was never reached.
3. **Given** a WASM binary whose checksum matches the registered artifact metadata, **When** `WasmExecutor` validates it, **Then** validation passes and execution proceeds normally.

## Requirements

- **FR-001**: The runtime MUST introduce a `CapabilityExecutor` trait in `traverse-runtime` with an `execute` method that accepts execution context and input, and returns a structured execution result.
- **FR-002**: `NativeExecutor` MUST implement `CapabilityExecutor` and wrap the existing Rust capability execution path with no behavioral change to any currently passing test.
- **FR-003**: `WasmExecutor` MUST implement `CapabilityExecutor` and dispatch WASM capabilities using the Wasmtime crate.
- **FR-004**: `WasmExecutor` MUST load the `.wasm` binary from the path declared in the capability's registered artifact metadata.
- **FR-005**: `WasmExecutor` MUST validate the SHA-256 checksum of the loaded binary against the registered artifact metadata before invoking Wasmtime. If validation fails, execution MUST NOT proceed.
- **FR-006**: `WasmExecutor` MUST instantiate each WASM module in an isolated Wasmtime `Store` with a `Linker` and WASI context scoped to the WASI capabilities declared in the capability contract.
- **FR-007**: No ambient WASI authority MUST be granted — file system access, environment variables, network, and clock access MUST each be explicitly declared in the capability contract and denied by default.
- **FR-008**: The runtime's selection and dispatch logic MUST route to `NativeExecutor` or `WasmExecutor` based on the execution type declared in the capability's registered execution metadata.
- **FR-009**: The execution trace MUST record the executor type used (`native` or `wasm`) for each execution attempt.
- **FR-010**: The `wasmtime` crate MUST be added to the `traverse-runtime` dependency in `Cargo.toml` at the workspace level.
- **FR-011**: No `unsafe` code MUST be introduced. The Wasmtime Rust API provides a safe interface and MUST be used exclusively.
- **FR-012**: The expedition example domain (#211) MUST be usable as the first real WASM capability exercising the full `WasmExecutor` path once available. This spec's tests use a minimal synthetic fixture.

## Success Criteria

- **SC-001**: The expedition example domain (#211) runs end-to-end via `WasmExecutor` once integrated — load, checksum validation, Wasmtime instantiation, scoped WASI, execution, output validation, and trace.
- **SC-002**: All existing native capability tests pass without modification after the `NativeExecutor` refactor.
- **SC-003**: A tampered WASM binary is rejected before Wasmtime instantiation with a structured `checksum_mismatch` failure and trace evidence.
- **SC-004**: No `unsafe` code is present in any new or modified file.
- **SC-005**: `cargo test` passes with 100% line coverage on new executor logic.
- **SC-006**: The spec-alignment CI gate (`scripts/ci/spec_alignment_check.sh`) passes with this spec registered in `specs/governance/approved-specs.json`.

## Assumptions

- The Wasmtime crate is available on crates.io at a version compatible with Rust 1.94+ and the existing workspace.
- WASM capabilities declare their WASI requirements in a structured field within the capability contract (definition of that field may require a minor contract amendment coordinated with spec 002).
- The expedition example domain (#211) is implemented in a separate issue and PR; this spec's test suite uses a minimal synthetic `.wasm` fixture compiled from a trivial WAT module.
- The capability registry already stores artifact metadata including a binary path and checksum field (or will have that field added as part of this work without requiring a new spec).
- SHA-256 is the checksum algorithm; the hash is stored as a hex string in the artifact metadata.

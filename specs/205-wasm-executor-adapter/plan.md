# Implementation Plan: WASM Executor Adapter

**Branch**: `205-wasm-executor-adapter` | **Date**: 2026-04-08 | **Spec**: [spec.md](spec.md)

## Summary

Introduce a `CapabilityExecutor` trait in `traverse-runtime` with two implementations: `NativeExecutor` (wraps the existing Rust execution path, zero behavioral change) and `WasmExecutor` (Wasmtime-backed, loads `.wasm` binaries from the registry, validates SHA-256 checksums, runs in an isolated Wasmtime sandbox with WASI capabilities scoped per contract). Add `wasmtime` to the workspace dependency. Tests use a minimal synthetic `.wasm` fixture compiled from WAT. Depends on #211 for the first real capability, but is independently testable and mergeable.

## Technical Context

**Language/Version**: Rust 1.94+
**Primary Dependencies**: `wasmtime` crate (with `wasi` feature), `traverse-contracts`, `traverse-registry`
**Storage**: WASM binary files loaded from the path declared in capability artifact metadata
**Testing**: Unit tests in `crates/traverse-runtime/tests/executor_tests.rs` using a minimal synthetic `.wasm` fixture compiled from WAT inline or stored under `crates/traverse-runtime/tests/fixtures/`
**Target Platform**: All (native host only; WASM is the guest)
**Project Type**: Core runtime extension
**Constraints**: No `unsafe`; no ambient WASI authority; WASI capabilities scoped per contract; all existing tests must pass unchanged

## Constitution Check

- Governed by spec 006-runtime-request-execution (extends execution layer).
- New spec ID 011 must be registered in `specs/governance/approved-specs.json` before merge.
- No contract boundary changes to runtime request, state machine, or trace shape — this spec refactors beneath selection.
- Minor amendment to capability artifact metadata (checksum + binary path fields) coordinated with spec 002 if not already present.
- CI gate check: `bash scripts/ci/spec_alignment_check.sh` must pass.

## Files Touched

```text
Cargo.toml                                                    # MODIFIED — add wasmtime workspace dependency
crates/traverse-runtime/src/executor/mod.rs                   # CREATED — CapabilityExecutor trait + executor routing logic
crates/traverse-runtime/src/executor/native.rs                # CREATED — NativeExecutor (refactor of existing execution path)
crates/traverse-runtime/src/executor/wasm.rs                  # CREATED — WasmExecutor (Wasmtime engine, store, linker, WASI scoping, checksum validation)
crates/traverse-runtime/tests/executor_tests.rs               # CREATED — unit tests for NativeExecutor, WasmExecutor, checksum rejection
crates/traverse-runtime/tests/fixtures/minimal.wasm           # CREATED — minimal synthetic WASM fixture (compiled from WAT)
specs/governance/approved-specs.json                          # MODIFIED — register spec ID 011
specs/205-wasm-executor-adapter/spec.md                       # CREATED (this spec)
specs/205-wasm-executor-adapter/plan.md                       # CREATED (this file)
```

## Phase 0: Research

- Read `crates/traverse-runtime/src/` to map the current execution call site that will become `NativeExecutor`.
- Read `crates/traverse-runtime/Cargo.toml` and the workspace `Cargo.toml` to determine the correct place to add the `wasmtime` dependency and confirm Rust edition compatibility.
- Read `contracts/` to determine whether capability artifact metadata already has `wasm_path` and `sha256_checksum` fields; if not, identify the minimal amendment needed.
- Read `specs/governance/approved-specs.json` to confirm the next available spec slot and verify spec ID 011 is free.
- Confirm the Wasmtime crate version that supports Rust 1.94+ and provides the `wasmtime-wasi` feature for WASI scoping.

## Phase 1: CapabilityExecutor trait

Define the `CapabilityExecutor` trait in `crates/traverse-runtime/src/executor/mod.rs`.

```rust
pub trait CapabilityExecutor {
    fn execute(
        &self,
        context: &ExecutionContext,
        input: &serde_json::Value,
    ) -> Result<ExecutionResult, ExecutionError>;
}

pub enum ExecutorKind {
    Native(NativeExecutor),
    Wasm(WasmExecutor),
}
```

- `ExecutionContext` carries the selected capability record, artifact metadata, and WASI scope declared by the contract.
- `ExecutionResult` is the existing result type from spec 006 — no shape change.
- `ExecutionError` covers both native errors and WASM-specific errors (`ChecksumMismatch`, `WasmInstantiationError`, `WasiScopeViolation`).
- The `executor/mod.rs` module also exposes a `route_executor(capability: &CapabilityRecord) -> ExecutorKind` function that reads `execution_metadata.executor_type` to dispatch.
- Register `mod executor;` in `crates/traverse-runtime/src/lib.rs` (or `main.rs` as appropriate).

## Phase 2: NativeExecutor refactor

Extract the existing Rust execution call site into `NativeExecutor` in `crates/traverse-runtime/src/executor/native.rs`.

- Identify the exact lines in the current runtime that call capability logic directly.
- Move that logic into `NativeExecutor::execute()` with identical semantics.
- Update the call site in the runtime to call `ExecutorKind::Native(NativeExecutor).execute(...)` instead.
- Run `cargo test` at the end of this phase — all existing tests must pass before proceeding to Phase 3.
- `NativeExecutor` MUST NOT initialize any Wasmtime state. It is a zero-cost wrapper for the native path.

## Phase 3: WasmExecutor implementation

Implement `WasmExecutor` in `crates/traverse-runtime/src/executor/wasm.rs`.

**3a — Checksum validation (runs before Wasmtime is touched):**
- Read the binary from `context.artifact_metadata.wasm_path`.
- Compute SHA-256 of the loaded bytes using the `sha2` crate (add to workspace deps if not present).
- Compare against `context.artifact_metadata.sha256_checksum` (hex string).
- If mismatch: return `ExecutionError::ChecksumMismatch { expected, observed, path }` immediately. Do not proceed.

**3b — Wasmtime instantiation:**
- Create a `wasmtime::Engine` with default config (or a shared engine if performance testing warrants it — default first).
- Create a `wasmtime::Store<WasiCtx>` where `WasiCtx` is built from the WASI capabilities declared in `context.wasi_scope`:
  - Default: no ambient authority (no FS, no env, no network, no random, no clock).
  - Each declared WASI capability adds exactly the corresponding preopened dir / env var / etc.
- Create a `wasmtime::Linker<WasiCtx>` and call `wasmtime_wasi::add_to_linker()` to link WASI functions.
- Compile the module with `wasmtime::Module::from_binary()`.
- Instantiate and call the capability's exported function (name taken from `execution_metadata.wasm_export`).
- Deserialize the return value into `ExecutionResult`.

**3c — Result and trace:**
- On success, return `ExecutionResult` with `executor_type: "wasm"` recorded in the trace extension.
- On any Wasmtime error, return `ExecutionError::WasmInstantiationError` or `ExecutionError::WasmTrapError` with structured detail.

## Phase 4: Tests

Create `crates/traverse-runtime/tests/executor_tests.rs` and a minimal WASM fixture.

**Fixture:**
- Write a WAT module (`tests/fixtures/minimal.wat`) that exports a function matching the expected interface, receives a JSON input, and returns a fixed JSON output.
- Compile to `tests/fixtures/minimal.wasm` using `wat::parse_file()` in a build script or inline using the `wat` crate in dev-dependencies. Record its SHA-256 for the test constants.

**Test cases:**
1. `native_executor_routes_correctly` — register a native capability, dispatch via `NativeExecutor`, verify result matches expected output.
2. `wasm_executor_loads_and_executes` — register the minimal fixture, dispatch via `WasmExecutor`, verify correct output and `executor_type: "wasm"` in trace.
3. `wasm_executor_rejects_tampered_binary` — load the fixture, flip one byte, write to a temp path, dispatch; verify `ChecksumMismatch` error is returned and no Wasmtime trap evidence exists.
4. `wasm_executor_scopes_wasi_to_contract` — register the fixture with an empty WASI scope, attempt an execution that would require FS access; verify the trap is caught and returned as a structured error, not a panic.
5. `all_existing_runtime_tests_pass` — covered by running `cargo test` with no changes to existing test files.

## Implementation Sequence

1. Phase 0: Research — read current executor call site, Cargo.toml, contracts, approved-specs.json.
2. Phase 1: Define `CapabilityExecutor` trait and `executor/mod.rs`. Add `mod executor;`. `cargo build` must succeed.
3. Phase 2: Extract `NativeExecutor`. Run `cargo test` — all existing tests must pass before continuing.
4. Phase 3a: Implement checksum validation in `WasmExecutor`. Unit test for rejection path only. `cargo test` must pass.
5. Phase 3b–3c: Implement Wasmtime instantiation and WASI scoping. Add `wasmtime` to `Cargo.toml`.
6. Phase 4: Write fixture and full test suite. `cargo test` must pass with 100% coverage on new executor files.
7. Register spec ID 011 in `specs/governance/approved-specs.json`.
8. Run `bash scripts/ci/spec_alignment_check.sh` — gate must pass.
9. Commit, push, open PR referencing #205. Note dependency on #211 in PR description.

**Note on #211 dependency**: This plan is independently completable and mergeable using the synthetic fixture. The expedition example domain (#211) is the first real WASM capability and will exercise the full `WasmExecutor` path in its own PR. No coordination gate is required here — #211 simply adds a new capability registration that routes to the already-merged `WasmExecutor`.

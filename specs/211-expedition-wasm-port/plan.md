# Implementation Plan: Expedition Example Domain — WASM Port

**Branch**: `211-expedition-wasm-port` | **Date**: 2026-04-08 | **Spec**: [spec.md](spec.md)

## Summary

Port the expedition example domain to a wasm32-wasi binary, register it in the capability registry with correct contract metadata, and validate the full WASM execution path through PlacementRouter and WasmExecutor.

## Technical Context

**Language/Version**: Rust 1.94+, wasm32-wasi target
**Primary Dependencies**: wasmtime (via WasmExecutor from #205), serde_json, traverse-contracts
**Storage**: .wasm binary (committed or build-script produced)
**Testing**: Integration test running expedition through PlacementRouter end-to-end
**Target Platform**: wasm32-wasi (compile target); host: any
**Project Type**: Example domain port + validation
**Constraints**: No tokio, no OS threads in WASM crate; depends on #205 and #210

## Constitution Check

Extends specs 008 and 009 as declared. Validates spec 011 (WasmExecutor) end-to-end. All CI gates pass.

## Files Touched

```text
crates/traverse-expedition-wasm/src/main.rs         # CREATED — WASM entrypoint (stdin→process→stdout)
crates/traverse-expedition-wasm/src/lib.rs          # CREATED — expedition business logic (no_std compatible)
crates/traverse-expedition-wasm/Cargo.toml          # CREATED — wasm32-wasi crate config
Cargo.toml                                          # MODIFIED — add traverse-expedition-wasm to workspace
contracts/expedition/expedition.json                # MODIFIED — add artifact_type, service_type, permitted_targets
crates/traverse-runtime/tests/expedition_wasm_e2e.rs # CREATED — end-to-end integration test
```

## Phase 0: Research

- Read existing expedition business logic in traverse-cli or traverse-runtime to understand what to extract
- Identify any tokio/async or OS-specific code that would block wasm32-wasi compilation
- Read current expedition contract file(s) in contracts/expedition/ to know what fields to add
- Confirm WasmExecutor API from #205 spec — how it receives the .wasm path and JSON input
- Confirm wasm32-wasi toolchain is available; note setup step for contributors

## Phase 1: WASM crate scaffold

Create `crates/traverse-expedition-wasm/Cargo.toml`:
- `[lib] crate-type = ["cdylib"]` or `[[bin]]` depending on WASI stdio approach
- No tokio, no std threads
- Dependencies: serde, serde_json only

## Phase 2: Business logic extraction

Move/copy expedition logic into `lib.rs` as pure functions (no async, no OS dependencies). Implement `main.rs` as:
1. Read JSON from stdin
2. Call expedition logic
3. Write JSON response to stdout
4. Exit 0 on success, 1 on error

## Phase 3: Contract update

Add to `contracts/expedition/expedition.json`:
- `"artifact_type": "wasm"`
- `"service_type": "stateless"`
- `"permitted_targets": ["cloud", "edge", "device"]`
- `"wasm_binary_path": "target/wasm32-wasi/debug/traverse_expedition_wasm.wasm"` (or release path)

## Phase 4: Integration test

In `tests/expedition_wasm_e2e.rs`:
1. Build the WASM binary at test time using `cargo build --target wasm32-wasi -p traverse-expedition-wasm` if not already built
2. Register the expedition capability with the WASM contract
3. Call PlacementRouter.execute() with a valid expedition request
4. Assert response is valid expedition output
5. Assert public trace entry exists with artifact_type: Wasm

## Implementation Sequence

1. Run `rustup target add wasm32-wasi` (note in CONTRIBUTING)
2. Scaffold WASM crate (Phase 1)
3. Extract expedition business logic (Phase 2)
4. `cargo build --target wasm32-wasi -p traverse-expedition-wasm` — fix any compilation errors
5. Update expedition contract (Phase 3)
6. Write end-to-end integration test (Phase 4)
7. cargo test all crates
8. bash scripts/ci/spec_alignment_check.sh
9. Commit + push + open PR declaring spec 008/009 extension

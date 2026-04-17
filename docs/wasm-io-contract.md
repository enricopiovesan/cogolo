# Traverse WASM Capability Stdin/Stdout I/O Contract

This document defines the current stdin/stdout JSON contract for WASM-backed capability execution in Traverse.

It is the normative authoring contract for capability packages executed through the `WasmExecutor` in `traverse-runtime`.

For guided package authoring, also see:

- [docs/wasm-agent-authoring-guide.md](wasm-agent-authoring-guide.md)
- [docs/wasm-microservice-authoring-guide.md](wasm-microservice-authoring-guide.md)

## Normative Runtime Behavior

Traverse executes a governed WASM capability by:

1. verifying that the resolved capability artifact type is `Wasm`
2. loading the configured `.wasm` binary from `wasm_binary_path`
3. verifying the configured SHA-256 digest when `wasm_checksum` is present
4. serializing the runtime input payload as JSON
5. passing that JSON payload to the WASM module through WASI stdin
6. capturing WASI stdout as the module's result channel
7. parsing stdout as JSON and returning that parsed value as the execution result

Each invocation uses a fresh Wasmtime store. Traverse does not preserve mutable WASM process state across calls.

## Input Contract

The runtime input payload is serialized with `serde_json` and written to stdin as UTF-8 JSON bytes.

Authors should assume:

- stdin contains exactly one JSON value representing the governed runtime input
- the payload shape is defined by the approved capability contract, not by ad hoc host conventions
- input should be read from stdin, not from environment variables, files, or network bootstrapping

Traverse does not currently add framing, headers, or side-channel metadata around the JSON payload.

## Output Contract

The WASM capability result must be written to stdout as valid UTF-8 JSON.

Authors should assume:

- stdout is the authoritative result channel
- the emitted JSON value becomes the runtime result payload
- the output shape must satisfy the capability's governed output contract

If stdout is not valid JSON, execution fails with `OutputDeserializationFailed`.

## Execution Boundary

The current WASM execution path is deny-by-default:

- stdin is provided
- stdout is captured
- no ambient filesystem authority is granted
- no ambient network authority is granted
- no ambient environment-variable contract is granted

Capability authors should treat any dependency on extra host authority as unsupported unless Traverse explicitly documents and governs it elsewhere.

## Failure Modes

The current execution path fails explicitly when:

- the resolved capability is not a WASM artifact
- `wasm_binary_path` is missing
- the binary cannot be loaded from disk
- the configured checksum does not match the loaded binary
- the module cannot be compiled or linked
- the module traps or otherwise fails during execution
- stdout cannot be parsed as JSON

These failure cases are part of the supported runtime behavior. They should not be masked with fallback bootstrap logic.

## Authoring Guidance

When creating a new WASM capability:

- read the governed input from stdin
- write exactly the governed result JSON to stdout
- keep the host interaction model narrow and explicit
- validate the binary digest after rebuilding the deterministic fixture
- treat stdin/stdout JSON as the stable execution boundary unless a newer approved spec says otherwise

## What Is Not Normative

The checked-in tests use tiny WAT fixtures such as:

- an echo module that reads stdin JSON and writes it back to stdout
- a module that intentionally writes invalid JSON to prove failure behavior

Those fixtures are test scaffolding. They demonstrate the contract, but they are not themselves part of the authoring surface or package model.

## Validation Sources

This document is grounded in:

- [crates/traverse-runtime/src/executor/wasm.rs](../crates/traverse-runtime/src/executor/wasm.rs)
- [crates/traverse-runtime/src/executor/mod.rs](../crates/traverse-runtime/src/executor/mod.rs)
- [crates/traverse-runtime/tests/executor_tests.rs](../crates/traverse-runtime/tests/executor_tests.rs)

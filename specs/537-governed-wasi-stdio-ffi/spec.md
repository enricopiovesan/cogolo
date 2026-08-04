# Feature Specification: Governed WASI Stdio FFI Boundary

**Status**: Approved
**Canonical governing ID**: `090-governed-wasi-stdio-ffi`
**Extends**: `038-wasi-host-insulation` (without changing Host ABI v1) and
`001-foundation-v0-1` (workspace safety policy)
**Decision evidence**: Traverse #921 decision record, 2026-08-04

## Purpose

Define the single guest-side Rust FFI exception needed for the expedition
WASM capability to consume JSON from WASI stdin, emit JSON to WASI stdout, and
return a deterministic process status. The exception is an implementation
boundary, not a new host capability or an expansion of Traverse Host ABI v1.

## Scope

In scope:

- one audited module at `crates/traverse-expedition-wasm/src/wasi_stdio.rs`;
- direct WASI Preview 1 imports of exactly `fd_read`, `fd_write`, and
  `proc_exit` from `wasi_snapshot_preview1`;
- a narrow lint exception that applies only to that module;
- deterministic CI checks of the Rust source boundary and compiled artifact
  import whitelist;
- ADR-0031's owner, review date, removal criteria, and security constraints.

Out of scope:

- adding `environ_get` or any other import to Traverse Host ABI v1;
- filesystem, environment, network, clock, random, socket, process-spawn, or
  host-defined capability access;
- a reusable generic unsafe-WASI crate or a workspace-wide unsafe exception;
- changing the host executor's existing ABI whitelist.

## Requirements

- **FR-001**: The workspace-level `unsafe_code = "deny"` lint MUST remain
  unchanged.
- **FR-002**: Only `crates/traverse-swift-host/src/lib.rs` and the expedition
  guest's `wasi_stdio.rs` module MAY contain unsafe syntax. The Swift boundary
  remains governed by ADR-0013/ADR-0015; this spec does not broaden it.
- **FR-003**: The expedition crate MUST scope its `unsafe_code` allowance to
  the `wasi_stdio` module only. No crate-wide guest opt-out is permitted.
- **FR-004**: The guest module MUST import exactly `fd_read`, `fd_write`, and
  `proc_exit` from `wasi_snapshot_preview1`; it MUST import no other WASI or
  host function.
- **FR-005**: `fd_read` MAY be used only to receive the bounded JSON request
  through stdin (file descriptor 0), `fd_write` only to emit the bounded JSON
  result/error through stdout (file descriptor 1), and `proc_exit` only to
  report the final bounded status.
- **FR-006**: All pointer, length, and I/O-result conversion must be contained
  in `wasi_stdio.rs`; business logic, JSON domain serialization, and capability
  dispatch MUST remain safe Rust outside that module.
- **FR-007**: Any nonzero WASI errno, oversized request/response, malformed
  input, or short write MUST become a deterministic, bounded capability error;
  it MUST NOT invoke undefined behavior, panic, or ambient fallback.
- **FR-008**: `scripts/ci/scoped_unsafe_boundary_check.sh` MUST fail when the
  workspace lint changes, unsafe syntax appears outside the two approved
  boundaries, the guest module lacks its local allowance, or its import set
  differs from FR-004.
- **FR-009**: Built expedition artifacts MUST continue to pass the existing
  Host ABI import whitelist verification. `environ_get` is explicitly denied.

## Acceptance Scenarios

1. Given a valid JSON request on stdin, when the expedition guest runs, then
   it reads only fd 0, emits one bounded JSON response only on fd 1, and exits
   deterministically.
2. Given a source change that adds `environ_get`, a filesystem import, or an
   unsafe block outside `wasi_stdio.rs`, when the scoped-boundary check runs,
   then CI fails and identifies the boundary violation.
3. Given a compiled expedition artifact, when Host ABI verification runs, then
   it accepts only the three declared Preview 1 stdio/process imports and
   rejects every other import before execution.
4. Given a malformed or too-large request, when the guest runs, then it emits
   the governed error response or exit status without a panic or host access.

## Quality Gates

- **QG-001**: The scoped unsafe-boundary script must run in required Rust CI.
- **QG-002**: Focused unit tests cover each errno/short-I/O/size-limit branch
  in the safe wrapper exposed by `wasi_stdio.rs`.
- **QG-003**: An integration test invokes the built guest through
  `WasmExecutor` and validates its artifact imports and stdin/stdout behavior.
- **QG-004**: `bash scripts/ci/spec_alignment_check.sh` and Host ABI artifact
  verification pass with this spec declared in the implementation PR.

## Compatibility and Removal

This is a source-level exception only. It does not version or alter Host ABI
v1. A future safe WASI guest-I/O abstraction may replace this module only via a
successor immutable spec and ADR. Until then, changes to its import set,
pointer handling, or scope require a successor ADR and explicit review.

## Implementation Tickets

- Traverse #936 — establish this governing spec, ADR, and CI policy.
- Traverse #921 — implement the input-driven expedition guest under this
  boundary after #936 merges.

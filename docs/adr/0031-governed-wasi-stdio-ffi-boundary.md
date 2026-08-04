# ADR-0031: Retain a Governed WASI Stdio FFI Boundary

- Status: Accepted
- Date: 2026-08-04
- Governing spec: `090-governed-wasi-stdio-ffi`
- Owner: Traverse maintainers
- Review by: 2026-11-04

## Context

Traverse #921 must replace the expedition guest's static fixture behavior with
real, input-driven logic. On `wasm32-wasip1`, Rust's normal `std::io` path
introduces `wasi_snapshot_preview1::environ_get`, which is not in the existing
Host ABI v1 whitelist. Direct stdio calls require Rust `unsafe`, while the
workspace correctly denies unsafe code by default.

## Decision

Keep the workspace deny-by-default policy. Permit exactly one additional,
guest-local exception: `crates/traverse-expedition-wasm/src/wasi_stdio.rs`.
It may use direct FFI only for `wasi_snapshot_preview1::fd_read`, `fd_write`,
and `proc_exit`.

The module is limited to bounded stdin/stdout/status translation. It may not
perform pointer work outside its reviewed helpers, import any other WASI or
Traverse host function, access environment/filesystem/network/clock/random
state, spawn processes, use mutable globals, or expose a general-purpose FFI
API. Business logic remains in safe Rust outside the module. The source and
compiled import boundaries are CI-enforced.

## Consequences

- #921 can provide portable JSON request/response execution without adding
  ambient environment authority or changing Host ABI v1.
- Reviewers have a finite source and import set to audit.
- The exception is not reusable by another guest. A second guest or any new
  import requires a successor ADR, spec amendment, owner, review date, and
  security review.

## Alternatives Considered

- Add `environ_get` to Host ABI v1: rejected because it expands ambient
  authority solely to accommodate the Rust standard-library I/O path.
- Build a shared safe host-I/O abstraction first: deferred because it creates
  a larger public surface than the expedition capability requires.
- Permit unsafe code workspace-wide: rejected because it removes the
  auditable, capability-specific boundary.

## Removal Criteria

Remove this exception when a reviewed safe guest-I/O abstraction can preserve
the same bounded, ABI-v1-compatible behavior without direct FFI. Removal must
delete the module allowance and tighten the CI allowlist in the same change.

# Feature Specification: No-Std WASI Guest Profile

**Status**: Approved
**Canonical governing ID**: `091-no-std-wasi-guest-profile`
**Extends**: `090-governed-wasi-stdio-ffi`
**Decision evidence**: Traverse #921 decision record, 2026-08-04

## Purpose

Define the no-`std` guest profile required for the expedition WASM component.
Normal Rust `std` startup imports `wasi_snapshot_preview1::environ_get`, which
Host ABI v1 deliberately denies. This profile removes that import without
expanding the Host ABI or weakening the audited FFI boundary.

## Requirements

- **FR-001**: `traverse-expedition-wasm` MUST compile for `wasm32-wasip1`
  with `#![no_std]` and `extern crate alloc`; it MUST NOT link Rust `std`.
- **FR-002**: The guest MUST supply an explicitly reviewed allocator and panic
  strategy. Allocation failure, panic, malformed input, and size-limit failure
  MUST terminate deterministically through the governed status path.
- **FR-003**: Dynamic allocation MUST be bounded by a documented guest memory
  limit; business input and output each remain bounded by Spec 090's I/O
  limits.
- **FR-004**: The compiled guest MUST import exactly
  `wasi_snapshot_preview1::{fd_read,fd_write,proc_exit}` and no other WASI or
  host symbol. `environ_get` remains forbidden.
- **FR-005**: The allocator, panic handler, and startup code MUST be guest
  local; they MUST NOT create a reusable host API, ambient authority, or new
  public runtime surface.
- **FR-006**: The existing source boundary check and artifact ABI verifier
  MUST be required validation for every change to the guest profile.

## Acceptance Scenarios

1. Given the built expedition guest, when ABI verification runs, then the
   artifact contains only the three allowed Preview 1 imports.
2. Given a valid bounded JSON request on stdin, when the guest executes via
   `WasmExecutor`, then it produces the contract-valid output without `std` or
   environment access.
3. Given an oversized request, allocation failure, or panic, when the guest
   executes, then it returns a deterministic bounded error/status without a
   trap that can crash the host.

## Quality Gates

- **QG-001**: A compile-time check rejects a guest build that links `std` or
  imports `environ_get`.
- **QG-002**: Focused tests cover allocator exhaustion, panic handling, I/O
  errno/short write, and malformed input behavior.
- **QG-003**: The end-to-end CLI/ArtifactRouter/WasmExecutor path runs the
  compiled guest and captures its trace evidence.

## Out of Scope

- Any Host ABI v1 addition, including `environ_get`.
- A shared allocator crate or a general no-`std` framework.
- Filesystem, environment, network, clock, random, or process-spawn access.

## Implementation Tickets

- Traverse #940 — establish this profile and ADR.
- Traverse #921 — migrate the expedition guest and prove the real artifact.

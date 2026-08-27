# Feature Specification: Bounded Native WASI Profile

**Status**: Approved
**Canonical governing ID**: `121-bounded-native-wasi-profile`
**Extends**: `071-native-runtime-wasm-bridge`, `090-governed-wasi-stdio-ffi`, and ADR-0014
**Tracks**: #1180; unblocks #1165

## Purpose

Define a separate, public native-host profile for digest-pinned WASI command
artifacts. It does not alter the Traverse bridge ABI and grants no ambient host
authority.

## Requirements

- **FR-001**: Before instantiation, a host MUST verify the artifact SHA-256,
  declared binary format, and configured maximum artifact size.
- **FR-002**: The profile MUST allow only `wasi_snapshot_preview1::fd_read`,
  `fd_write`, and `proc_exit`; every other WASI or host import fails closed.
- **FR-003**: Stdin, stdout, and exit status MUST be bounded by configured
  input and output byte limits and produce stable, secret-free errors.
- **FR-004**: The host MUST enforce a memory maximum and fuel budget. A host
  lacking a documented execution interruption control MUST reject execution
  rather than claim bounded execution.
- **FR-005**: Filesystem, network, environment, clock, random, process-spawn,
  and arbitrary host imports MUST remain unavailable.
- **FR-006**: Evidence MUST contain artifact identity, host/engine/version,
  configured resource-limit identifiers, outcome, and redacted projections;
  it MUST exclude payloads, paths, and credentials.
- **FR-007**: The profile MUST execute the repository-pinned cross-host
  fixture success, invalid-input, and artifact-identity-failure cases.

## Compatibility

This is an additive profile. Bridge-ABI artifacts remain governed by Spec 071
and cannot be executed by this profile merely by sharing an engine.

## Out of scope

Changing the bridge ABI, adding ambient WASI services, SPI/watchdog controls,
or making a five-platform certification claim.

## Acceptance scenarios

1. The native profile runs the pinned hello-world WASI artifact with matching
   canonical output and redacted trace projections.
2. A digest mismatch, forbidden import, oversized I/O, memory violation, or
   fuel exhaustion fails before success evidence is emitted.
3. The native evidence compares with browser and CLI records through the
   #1161 comparison contract.

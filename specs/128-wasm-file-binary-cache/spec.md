# Feature Specification: WASM File-Binary Cache

**Status**: Approved
**Canonical governing ID**: `128-wasm-file-binary-cache`
**Extends**: `061-wasm-module-cache`
**Decision evidence**: Traverse #1225 approval, 2026-09-05

## Purpose

Avoid repeated disk reads and SHA-256 calculations when a host repeatedly
executes the same unchanged file-backed WASM capability. The host retains the
bytes and verified checksum only while the file identity remains unchanged.

## Requirements

- **FR-001**: `WasmExecutor` MUST retain a bounded in-memory cache of
  file-backed WASM bytes and their SHA-256 checksum.
- **FR-002**: A cache entry MUST be reusable only when the capability path,
  file size, and last-modified timestamp still match the file on disk.
- **FR-003**: A matching entry MUST avoid another full binary read and SHA-256
  calculation, while preserving the existing compiled-module cache behavior.
- **FR-004**: A changed, missing, unreadable, or checksum-mismatched binary
  MUST follow the existing typed failure path; a cache hit MUST NOT conceal a
  normal file change detected by the file identity check.
- **FR-005**: The binary cache MUST use deterministic oldest-entry eviction
  and share the configured entry bound of the compiled-module cache.
- **FR-006**: Cache counters MUST expose enough evidence to verify first-load,
  cache-hit, and eviction behavior.
- **FR-007**: Each execution MUST continue to create a fresh WASI context,
  Store, and resource-limit state.

## Acceptance Scenarios

1. Given an unchanged file-backed capability invoked twice, when the second
   invocation starts, then it reuses the cached bytes and checksum without a
   second binary read or SHA-256 pass.
2. Given a capability file modified after its first invocation, when it is
   invoked again, then the file identity invalidates the entry and checksum
   validation receives the modified bytes.
3. Given the binary cache has reached its configured bound, when another
   distinct file is loaded, then the oldest entry is evicted deterministically.

## Quality Gates

- **QG-001**: Tests prove an unchanged repeated invocation records one load,
  one hash, and one cache hit.
- **QG-002**: Tests prove a modified file still produces the existing checksum
  mismatch failure when its declared checksum no longer matches.
- **QG-003**: The protected runtime coverage gate, executor tests, lint, and
  spec-alignment gate MUST pass.

## Out of Scope

- Persistent caches across process restarts.
- Caching a file when its identity cannot be read.
- Any change to the host ABI, capability contracts, or artifact trust model.

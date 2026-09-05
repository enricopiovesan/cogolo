# ADR-0057: Cache Unchanged File-Backed WASM Binaries

- Status: Accepted
- Date: 2026-09-05
- Governing spec: `128-wasm-file-binary-cache`
- Extends: `061-wasm-module-cache`
- Related issue: #1225

## Context

The existing module cache avoids recompiling identical WASM modules, but each
file-backed invocation still reads the complete binary and computes its SHA-256
checksum before reaching that cache. This creates repeat I/O and CPU cost for
unchanged capabilities. Spec 061 deliberately left file-read avoidance out of
scope; this ADR records the approved extension.

## Decision

Retain a bounded host-local cache of binary bytes and checksum keyed by path,
file size, and last-modified timestamp. Revalidate file identity before every
use. On a matching identity, reuse the bytes and checksum; otherwise reload
and validate the file through the existing error paths. Use deterministic
oldest-entry eviction with the existing module-cache entry bound.

## Consequences

Repeated invocations of unchanged local artifacts avoid a full read and hash,
without retaining Wasmtime Store or WASI state between calls. Normal file
changes invalidate the cache before checksum validation. Filesystems that do
not expose usable modification time fail through the existing typed binary-load
path rather than creating an unverifiable cache entry.

## Alternatives Considered

- Re-hash and read every invocation: rejected because it preserves the
  unnecessary cost described in #1225.
- Trust only a path: rejected because replacement of an artifact would be
  invisible.
- Persist cache state across restarts: rejected because it would add lifecycle
  and trust requirements outside this narrow runtime slice.

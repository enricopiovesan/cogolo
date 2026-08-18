# ADR-0041: Cross-Host Embedded Registry Cache Contract

- Status: Accepted
- Governing specs: `080-embedded-registry-cache`, `107-cross-host-embedded-registry-cache`
- Related issues: #826, #1071, #1072

## Context

Rust and Web embedders already support explicit preparation followed by offline
`registry_ref` resolution from a host-owned verified cache. Swift, Kotlin, and
.NET expose runtime bridge packages but have no equivalent governed cache
surface. Leaving them to materialize local paths in consumer tooling duplicates
resolution behavior and makes registry-only application bundles non-portable.

## Decision

Define one semantic cache contract for every host. Each platform provides an
idiomatic adapter and retains ownership of storage, network fetch, lifecycle,
and platform policy. The adapter's preparation operation validates the synced
index, deterministically selects a non-yanked version, verifies digests, and
atomically records non-secret evidence. Initialization and execution only read
verified entries and are offline-only.

The contract standardizes observable errors and evidence, not cache layout or
language-level API names. No adapter may fall back to local examples or a
network fetch when an offline entry is missing or invalid.

## Consequences

Swift, Kotlin, and .NET can consume registry-only bundles without an App-Refs
manifest rewrite. Hosts retain control over storage and credentials while users
receive consistent failure handling and provenance evidence across platforms.
Existing Rust/Web APIs remain compatible.

## Alternatives Considered

- Require native hosts to use Rust's file-cache implementation: rejected because
  it couples host packaging and storage policy to one language/runtime.
- Give every platform independent cache semantics: rejected because errors,
  provenance, and fail-closed behavior would drift.
- Retain App-Refs materialization as the product path: rejected because it
  duplicates resolution outside Traverse and cannot support registry-only bundles.

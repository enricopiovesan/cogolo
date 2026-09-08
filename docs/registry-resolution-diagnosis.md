# Exact-version `registry_ref` resolution failure — diagnosis

**Issue**: [#1273](https://github.com/traverse-framework/traverse/issues/1273)
(diagnosis slice of [#1272](https://github.com/traverse-framework/traverse/issues/1272))
**Governing spec**: `1258-offline-cache-activation`
**Status**: diagnosis complete; implementation is the successor slice on #1272

This document records the root cause of the opaque

```json
{"code":"registry_reference_requires_resolution","message":"registry asset download failed"}
```

failure reported for an active, signed, exact-version `registry_ref`, and
proposes the stable redacted error taxonomy the successor slice must implement.
No credentials, cache paths, authorization headers, endpoints, or artifact bytes
appear here or in the checked-in fixtures.

## Where resolution happens

`crates/traverse-cli/src/main.rs` — `SyncedRegistryComponentResolver::resolve`,
invoked from `app validate` and `app register` via
`load_application_bundle_manifest_with_resolver`. It:

1. calls `traverse_registry::resolve_synced_public_registry_range` (synced-state
   read + semver selection, never a network call, no fallback);
2. `cache_registry_asset` for the contract URL — cache-hit check, else
   `curl -fsSL <url>`, then `cache_verified_public_registry_bytes` (digest
   check + content-addressed write);
3. `parse_contract` on the fetched bytes;
4. `cache_registry_asset` for the artifact URL (same path as step 2).

## Root cause

**Every distinct failure boundary is reported under one error code with an
unstructured message, and three boundaries are not evaluated at all.**

1. **Code collapse.** `registry_resolution_failure()` hard-codes
   `ApplicationManifestErrorCode::RegistryReferenceRequiresResolution` for
   every failure — synced-state read, "no matching version", "deprecated
   only", contract HTTP failure, artifact HTTP failure, digest mismatch,
   contract parse failure, and cache write failure all share it. The
   published `traverse-registry` crate (`=0.18.0`, immutable) also exposes
   only this single `ApplicationManifestErrorCode` variant for the whole
   resolution surface, so the finer taxonomy must be produced on the Traverse
   (CLI) side before the `ApplicationManifestFailure` is constructed.
2. **Message is free text, and lossy for HTTP failures.** The `curl` branch
   returns the fixed literal `"registry asset download failed"` for *any*
   non-zero exit — the exit status, the HTTP status class, and **which asset
   (contract vs artifact)** are all discarded (`std::process::Command` output
   is taken but `stderr` is never inspected, and `-f` suppresses the body).
   A caller cannot tell a 404 on the contract from a 503 on the artifact.
3. **Cache-path leak.** On a digest mismatch,
   `cache_verified_public_registry_bytes` builds its message with
   `path.display()` — the local content-addressed cache path
   (`…/.traverse/cache/sha256/<digest>`) — and `SyncedRegistryComponentResolver`
   flattens that straight into user-facing evidence. This violates spec
   `1258-offline-cache-activation` FR-003 and the constitution's
   "no credential/path leakage" rule. Characterized by
   `registry_ref_digest_mismatch_collapses_and_leaks_cache_path`.
4. **Un-evaluated boundaries.** The resolver performs **no** signature / trust
   bundle verification, **no** Host ABI compatibility check, and **no**
   placement-target eligibility check against the record's
   `permitted_targets`. It also gates only on the `deprecated` boolean, not on
   the record `lifecycle` field, so a `yanked` / `draft` / `inactive` exact
   version is not rejected with a distinct reason.
5. **Un-governed fetch.** Fetching happens through a `curl` subprocess during
   `app validate` / `app register`. Spec `1258-offline-cache-activation`
   FR-001 requires preparation/activation to consume an already-verified
   host-owned cache entry and make no network request; the current path does
   neither, and network/host/permission policy is not applied to the `curl`
   call.

The exact-version guarantee itself is intact: `resolve_synced_public_registry_range`
never substitutes another version for an exact `==x.y.z` miss (verified by
`exact_version_registry_ref_never_falls_back_to_another_version`).

## Affected host targets

| Target | Resolution path | Affected |
| --- | --- | --- |
| Local / native CLI (`app validate`, `app register`) | `SyncedRegistryComponentResolver` + `curl` subprocess | Yes — all findings above |
| Browser / Web | `traverse-embedder` `registry_cache.rs` (separate) | Must expose equivalent codes/evidence per spec `1258` FR-006 — taxonomy below applies there too |
| Edge / device | via embedder cache | Same as Web |

## Reproduction (deterministic, offline)

`crates/traverse-cli/src/main.rs` `#[cfg(test)] mod tests`:

- `exact_version_registry_ref_resolves_when_every_boundary_passes` — anchors a
  fully-valid exact `=1.0.1` fixture (`file://` URLs, real digests).
- `exact_version_registry_ref_never_falls_back_to_another_version`
- `registry_ref_contract_and_artifact_fetch_failures_are_indistinguishable`
- `registry_ref_digest_mismatch_collapses_and_leaks_cache_path` (characterization)
- `every_registry_resolution_boundary_collapses_to_one_code` — five boundaries,
  one code.

The taxonomy itself is an executable spec in
`crates/traverse-cli/src/registry_resolution_diagnostics.rs` (stage enum,
`classify` returning the first failing boundary in evaluation order, and
`redacted_evidence` restricted to the permitted field set), with the
machine-readable map at
`crates/traverse-cli/tests/fixtures/registry-resolution/boundary-error-map.json`.

## Proposed error-code map (contract for the #1272 implementation slice)

Evaluated in order; the first failing boundary is reported. Evidence may carry
only: `namespace`, `id`, `requested_range`, `selected_version`,
`contract_digest`, `artifact_digest`, `trust_lifecycle`, `abi`, `target`,
`placement`, `stage`, `code`, `summary`.

| # | Stage | Proposed code | Today |
| --- | --- | --- | --- |
| 1 | index selection | `registry_index_selection_failed` | collapsed |
| 2 | version range | `registry_version_range_unsatisfied` | collapsed |
| 3 | lifecycle | `registry_lifecycle_rejected` | partial (`deprecated` only) |
| 4 | contract retrieval | `registry_contract_unreachable` | collapsed, message lossy |
| 5 | contract digest | `registry_contract_digest_mismatch` | collapsed, **path leak** |
| 6 | artifact retrieval | `registry_artifact_unreachable` | collapsed, message lossy |
| 7 | artifact digest | `registry_artifact_digest_mismatch` | collapsed, **path leak** |
| 8 | signature | `registry_signature_unverified` | **not checked** |
| 9 | ABI | `registry_abi_incompatible` | **not checked** |
| 10 | target | `registry_target_incompatible` | **not checked** |
| 11 | cache persistence | `registry_cache_commit_failed` | collapsed |

## Recommendation for the successor slice

1. Introduce a Traverse-side `RegistryResolutionError { stage, code, evidence }`
   that the resolver produces before mapping to `ApplicationManifestFailure`;
   keep one manifest-level code but carry the stage/code/redacted evidence in a
   structured `details` field.
2. Replace the `curl` subprocess with consumption of a prepared verified cache
   entry (spec `1258` FR-001/FR-002); if any fetch layer remains, route it
   through the governed network/host/permission policy and record only the HTTP
   status class.
3. Redact `cache_verified_public_registry_bytes` messages — key by digest, never
   by path — or wrap them at the resolver boundary.
4. Add the three missing boundaries (signature, ABI, target) and gate on the
   record `lifecycle` field.
5. Mirror the taxonomy in the `traverse-embedder` Web resolver so Rust and Web
   expose equivalent evidence and error semantics (spec `1258` FR-006).

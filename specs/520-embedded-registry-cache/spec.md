# Feature Specification: Embedded Registry Dependency Cache

**Feature Branch**: `520-embedded-registry-cache`  
**Status**: Approved
**Canonical governing ID**: `080-embedded-registry-cache`
**Input**: Traverse #831; successors to Specs 054 and 055; production boundary from Specs 057 and 068.

## Purpose

Define the host-owned preparation and verified local-cache contract that lets a
production embedder resolve an application bundle's `registry_ref` dependencies
without a CLI sidecar, App-References manifest rewrite, or runtime network use.

This specification derives from Decision 35 in the decision log.

## Boundary and Ownership

| Concern | Owner |
| --- | --- |
| Fetch/sync index and artifacts during explicit preparation | host application or its deployment tooling |
| Validate index records, select versions, verify digests, and produce evidence | Traverse public resolver library |
| Persist/evict cache bytes and protect its storage location | host application |
| Resolve and execute dependencies after `init` | embedded Traverse runtime, local cache only |
| Publish provenance and yanked/deprecated state | Registry |
| Bundle selection and release integration | application/reference-app owner |

## Functional Requirements

- **FR-001**: The public host API MUST separate `prepare` from `init` and
  execution. Only `prepare` MAY use a network-capable source supplied by the
  host; `init`, `submit`, and `subscribe` MUST be offline-only.
- **FR-002**: Preparation input MUST include a validated synced-index snapshot,
  requested `registry_ref`, and host cache writer. It MUST select the highest
  non-yanked version satisfying `version_range` deterministically.
- **FR-003**: A cache entry MUST be content-addressed by the published artifact
  digest and contain the selected record identity/version, source release and
  index digest, artifact digest, and verification timestamp.
- **FR-004**: Preparation MUST write artifact bytes atomically and verify the
  published digest before marking an entry usable. A partial or failed write
  MUST never be executable.
- **FR-005**: Embedded initialization MUST accept only a host-provided cache
  reader and verified cache entries. It MUST NOT synthesize a cache root,
  fetch an index/artifact, or fall back to `traverse-cli serve` or
  `.traverse/server.json`.
- **FR-006**: Cache lifecycle, storage limits, encryption, backup, and eviction
  policy remain host-owned; eviction MUST preserve entries referenced by the
  active bundle or cause deterministic re-preparation before `init`.
- **FR-007**: Errors MUST be stable and secret-free: `registry_sync_missing`,
  `registry_version_not_found`, `registry_dependency_yanked`,
  `registry_prepare_failed`, `registry_artifact_digest_mismatch`, and
  `registry_cache_entry_missing`.
- **FR-008**: Resolution evidence MUST retain registry lock/provenance fields:
  namespace, id, selected version, version range, source release, index digest,
  artifact digest, cache verification timestamp, and outcome. It MUST NOT
  expose cache paths, credentials, or artifact contents.

## Acceptance Scenarios

1. Given a synced index and matching artifact, when the host prepares a Rust
   or Web bundle, then the cache contains verified content-addressed bytes and
   `init` executes offline with provenance evidence.
2. Given no cache entry, when embedded `init` sees a `registry_ref`, then it
   returns `registry_cache_entry_missing` and performs no network request.
3. Given a yanked matching record, when preparation runs, then it returns
   `registry_dependency_yanked` and leaves the previous verified cache intact.
4. Given artifact bytes with a non-matching digest, when preparation runs,
   then it returns `registry_artifact_digest_mismatch`, records no usable entry,
   and the subsequent embedded runtime cannot execute it.

## Compatibility

This is additive. Existing CLI `registry sync` and registration behavior from
Specs 054/055 remains supported for development and CI. Production embedders
gain a host-owned equivalent without changing `registry_ref` identity or
version-range semantics.

## Out of Scope

- Hosted registry APIs, a remote runtime host, or runtime network fetching.
- Cache encryption, retention, backup, and restore mechanics.
- Resolver implementation or platform package changes; those follow this spec.

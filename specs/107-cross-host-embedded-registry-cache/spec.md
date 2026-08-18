# Feature Specification: Cross-Host Embedded Registry Cache

**Status**: Approved
**Canonical governing ID**: `107-cross-host-embedded-registry-cache`
**Version**: 1.0.0
**Extends**: `080-embedded-registry-cache`, `054-public-scope-registry-ref`,
`055-registry-sync`, `068-public-platform-embedder-packages`.
**Input**: #1071, #1072; ADR-0041.

## Purpose

Provide one portable host-owned registry-cache contract for Swift, Kotlin, and
.NET embedders. Each platform exposes idiomatic APIs, but every implementation
has the same preparation, offline-resolution, evidence, error, and compatibility
semantics as the approved Rust and Web paths.

## Boundary and Ownership

| Concern | Owner |
| --- | --- |
| Fetching registry index and artifacts during explicit preparation | Host application or deployment tooling |
| Selecting a compatible record, digest verification, and evidence schema | Traverse public resolver contract |
| Cache root, persistence, encryption, backup, and eviction | Host application |
| Initialization and execution after preparation | Embedded runtime using verified local entries only |
| Registry release and lifecycle provenance | Registry |

## Functional Requirements

- **FR-001**: Swift, Kotlin, and .NET MUST expose an explicit preparation
  operation and an offline resolution operation for `registry_ref` dependencies.
  Their public APIs MAY be idiomatic, but MUST preserve this contract's semantics.
- **FR-002**: Preparation MUST accept a validated synced-index snapshot, a
  `registry_ref`, a host-supplied fetcher, and a host-owned cache writer. It MUST
  select the highest non-yanked version satisfying `version_range`
  deterministically.
- **FR-003**: Preparation MUST verify published artifact and contract digests
  before atomically making an entry usable. Partial or failed entries MUST NOT be
  executable.
- **FR-004**: Initialization, submission, subscription, and execution MUST be
  offline-only. They MUST NOT fetch an index or artifact, synthesize a cache
  root, fall back to local examples, or use `traverse-cli serve`.
- **FR-005**: A verified cache entry MUST retain namespace, capability id,
  selected version, requested version range, source release, index digest,
  artifact digest, verification timestamp, and outcome. It MUST NOT expose cache
  paths, credentials, configuration values, or artifact contents.
- **FR-006**: All platforms MUST use these stable secret-free failure codes:
  `registry_sync_missing`, `registry_version_not_found`,
  `registry_dependency_yanked`, `registry_prepare_failed`,
  `registry_artifact_digest_mismatch`, and `registry_cache_entry_missing`.
- **FR-007**: A platform adapter MUST reject cache entries whose artifact or
  contract bytes no longer match their recorded digest, and MUST fail closed
  rather than silently re-resolving or accepting a local-path substitute.
- **FR-008**: Cache lifecycle and platform storage mechanics remain host-owned.
  Traverse MUST not require a shared file layout across Swift, Kotlin, .NET,
  Rust, and Web implementations.
- **FR-009**: Cross-host conformance fixtures MUST demonstrate equivalent
  observable results for preparation success, missing cache, yanked dependency,
  and artifact digest mismatch.

## Acceptance Scenarios

1. Given a synced index and matching artifact, each native host prepares a
   `registry_ref`, then initializes and executes from verified local cache bytes
   with non-secret provenance evidence.
2. Given no verified entry, native embedded initialization returns
   `registry_cache_entry_missing` and performs no network request.
3. Given a yanked matching record, preparation returns
   `registry_dependency_yanked`, preserves prior verified entries, and makes no
   new entry usable.
4. Given an artifact whose bytes do not match its published digest, preparation
   returns `registry_artifact_digest_mismatch`; later initialization cannot use
   it.
5. Given a cache entry modified after preparation, resolution fails closed with
   `registry_artifact_digest_mismatch` and does not select another artifact.

## Compatibility

This is additive. Spec 080's Rust/Web API and evidence remain supported. Native
adapters may choose language-native method names and storage backends, but they
MUST preserve the stable error codes, required evidence fields, offline boundary,
and version-selection rules in this specification. Existing local-path bundle
loading remains available for bundles that do not declare `registry_ref`.

## Out of Scope

- Runtime network fetching, hosted registry APIs, or a remote runtime host.
- Cache encryption, retention, backup, restore, or storage-engine selection.
- Application activation-time artifact selection or connector invocation.
- App-Refs-specific materialization scripts as a supported product path.

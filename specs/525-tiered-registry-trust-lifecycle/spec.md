# Feature Specification: Tiered Registry Trust and Lifecycle

**Status**: Draft — implementation requires normal governance approval.
**Input**: Traverse #845; Decision 38; `524-production-app-readiness`; Specs
`054-public-scope-registry-ref`, `055-registry-sync`, `063-registry-contract-materialization`,
and `080-embedded-registry-cache`.

## Purpose

Define the bounded registry-facing successor to the production-readiness
baseline. It gives app authors deterministic trusted discovery and exact,
reviewable dependency resolution while leaving registry hosting, embedded-cache
implementation, and platform certification to their own successor specs.

## Capability Boundary

**Trusted capability discovery and lifecycle evaluation** accepts a locally
validated registry snapshot plus an explicit consumer policy, returns eligible
capabilities or an exact lock resolution with safe evidence, and never fetches
or changes registry state. Publication, cache preparation, and execution
consume its outputs but are not part of this specification.

## Requirements

- **FR-001**: Every published registry record MUST declare exactly one `tier`:
  `certified`, `community`, or `kit`. `kit` covers examples, starter kits, and
  other non-production learning content.
- **FR-002**: Discovery and range resolution MUST include only `certified`
  records by default. `community` and `kit` require an explicit,
  machine-readable consumer opt-in; one opt-in MUST NOT imply the other.
- **FR-003**: Discovery output and resolution evidence MUST expose tier,
  publisher identity, publisher support state, provenance reference, lifecycle
  state, and source snapshot identity. It MUST NOT expose credentials, cache
  paths, or artifact contents.
- **FR-004**: A `certified` record MUST have verified publisher identity,
  immutable signed provenance, successful automated validation, applicable
  conformance evidence, and a maintainer-approved support/deprecation policy.
  A record missing required evidence is ineligible for Certified discovery or
  resolution.
- **FR-005**: Community and Kit admission requirements and their non-Certified
  support posture MUST be visible in metadata. They MUST NOT claim Certified
  admission merely because their artifacts validate.
- **FR-006**: Production preparation MUST resolve every declared range to an
  exact immutable lock entry before initialization. An entry MUST bind
  namespace, id, exact version, artifact and contract digests, publisher,
  tier, provenance reference, source release, and index digest.
- **FR-007**: A production lockfile MUST be committed with the consuming
  bundle, deterministic for identical manifest, snapshot, and policy inputs,
  and immutable once prepared. Changing any selected identity, digest, tier,
  publisher, or provenance requires an explicit new lock generation.
- **FR-008**: Normal yanks MUST make a record ineligible for new resolution and
  preparation. They MUST NOT silently invalidate an existing active generation;
  consumers receive lifecycle evidence and upgrade guidance.
- **FR-009**: A security yank MUST publish a minimum safe version, an
  enforcement deadline, or both. A Certified consumer with locally known
  applicable policy MUST fail closed after its deadline or when its locked
  version is below the minimum safe version. Output includes safe upgrade
  guidance.
- **FR-010**: Normal deprecation of a Certified record MUST provide at least
  90 days' notice and name a replacement or migration path. It remains
  discoverable and resolvable until its stated end-of-support date; a security
  yank follows FR-009 instead.
- **FR-011**: Lifecycle transitions are monotonic and provenance-backed:
  `active` may transition to `deprecated`, `yanked`, or `security_yanked`;
  `deprecated` may transition to either yanked state; a yanked state MUST NOT
  transition back to active under the same identity and version.
- **FR-012**: Stable errors MUST include `registry_tier_opt_in_required`,
  `registry_certification_evidence_missing`, `registry_lock_resolution_failed`,
  `registry_lock_integrity_mismatch`, `registry_dependency_yanked`, and
  `registry_security_yank_enforced`. Errors are deterministic, actionable, and
  safe for host-visible output.

## Compatibility

Existing public-tier sync, private-overlay precedence, `registry_ref` identity,
and offline execution semantics remain unchanged. Tier and lifecycle metadata
is additive. A legacy record without a tier is not silently treated as
Certified; it is ineligible for production default discovery until governed
metadata exists. Exact lockfiles complement, rather than replace, manifest
semver ranges.

## Acceptance Scenarios

1. Given a valid snapshot containing Certified, Community, and Kit records,
   when discovery runs without opt-ins, then only Certified records appear in
   deterministic order with safe trust evidence.
2. Given a Community record and an explicit Community opt-in, when resolution
   runs, then it may select that record and records the opt-in and tier in lock
   evidence; without opt-in it returns `registry_tier_opt_in_required`.
3. Given a Certified candidate missing signed provenance or conformance
   evidence, when it is discovered or resolved, then it is ineligible and
   returns `registry_certification_evidence_missing`.
4. Given a declared range and valid eligible candidate, when production
   preparation resolves it, then it writes a committed exact lock entry with
   every FR-006 field and identical inputs yield byte-identical lock content.
5. Given a normal-yanked candidate and an active verified generation, when new
   preparation runs, then it returns `registry_dependency_yanked` while the
   active generation remains runnable.
6. Given a Certified lock entry subject to a locally known security-yank
   deadline that has passed, when lifecycle policy is evaluated, then it fails
   closed with `registry_security_yank_enforced` and upgrade guidance.
7. Given normal Certified deprecation, when it is published, then metadata has
   at least 90 days' notice and a replacement or migration path; a shorter
   notice is rejected.

## Governed Files and Conformance

This successor governs future registry record schemas, local synced-index
validation, deterministic discovery/resolution, production lockfile
serialization, and their contract/integration tests in `traverse-cli`,
`traverse-contracts`, and `traverse-embedder`. Independent conformance MUST
exercise default exclusion and explicit opt-ins, Certified evidence rejection,
byte-stable lock generation, normal-yank preparation rejection, security-yank
deadline/minimum-safe-version enforcement, and 90-day deprecation validation.

## Out of Scope

- Hosted registry APIs, publisher onboarding UI, or registry implementation.
- Artifact/cache preparation, activation, rollback, and runtime execution.
- Platform certification execution or cross-platform test harnesses.
- Private multi-source routing, remote synchronization, key rotation, and
  automatic lockfile updates.

## Follow-on Contracts

Implementation requires successor contract amendments that name the record and
lockfile schemas, publisher approval authority, signature verification method,
lifecycle event shapes, exact error payloads, and compatibility versioning.
No code or approved-spec registry update is authorized by this Draft.

# Feature Specification: Tiered Registry Trust and Lifecycle

**Feature Branch**: `codex/issue-845-tiered-registry-trust`
**Created**: 2026-07-28
**Status**: Draft — successor specification requiring maintainer approval before implementation.
**Input**: Issue #845, Decision 38, and `524-production-app-readiness`.

## Purpose

Define the registry metadata and deterministic policy boundary for Certified,
Community, and Kit/example content. This specification makes Certified the
production discovery default, defines publisher admission and lifecycle
evidence, and defines the lockfile information a production host must retain.
It authorizes neither registry hosting nor embedded-cache implementation.

## Capability Boundary

The registry evaluates whether a published capability record is eligible for a
requested trust profile and reports an explainable outcome. Publishers provide
admission and lifecycle evidence; consumers choose their permitted tiers.
Runtime execution, artifact caching, signing implementation, and platform
certification execution remain separate capabilities.

## Functional Requirements

- **FR-001**: Every discoverable registry record MUST expose `tier`, publisher
  identity, support state, lifecycle state, immutable provenance reference, and
  a record schema version. Tier values are `certified`, `community`, and
  `kit`; `kit` is the machine value for Kit/example content.
- **FR-002**: Discovery without an explicit tier filter MUST return only
  Certified records. A consumer MAY opt in to Community and/or Kit records by
  supplying an explicit allowed-tier set; a response MUST report that set and
  the policy outcome for every excluded candidate.
- **FR-003**: A record is eligible for Certified discovery only when its
  publisher admission evidence contains a verified publisher identity, signed
  immutable provenance, successful automated validation, conformance evidence,
  and a maintainer-approved support and deprecation policy. Missing or expired
  evidence MUST fail closed as `registry_certification_evidence_missing`.
- **FR-004**: Registry mutation MUST preserve immutable published artifact
  identity, version, digest, and provenance. A tier or lifecycle change creates
  a new signed registry-state record referencing the immutable artifact; it
  MUST NOT rewrite the artifact identity or provenance.
- **FR-005**: Lifecycle states are `active`, `deprecated`, `yanked`, and
  `security_yanked`. `deprecated` MUST include a replacement or migration path
  and a notice deadline at least 90 days after publication. Normal yanks reject
  new resolution and preparation. Security yanks include a minimum-safe version
  and/or enforcement deadline.
- **FR-006**: A production lockfile MUST record the exact namespace, id,
  version, artifact digest, publisher identity, tier, provenance reference,
  source release, index digest, lifecycle-state record digest, and resolution
  timestamp. It is immutable after preparation and is sufficient to explain an
  offline lifecycle decision without a registry request.
- **FR-007**: Given a locally known security-yank state, a Certified consumer
  MUST fail closed after its deadline or below its minimum-safe version with
  `registry_security_yank_enforced`. Before enforcement, it MUST return a
  deterministic warning/evidence outcome. A Community or Kit consumer receives
  the state as evidence but is not silently elevated to Certified policy.
- **FR-008**: Stable, secret-free errors are
  `registry_tier_opt_in_required`, `registry_certification_evidence_missing`,
  `registry_lifecycle_invalid`, `registry_dependency_deprecated`,
  `registry_dependency_yanked`, and `registry_security_yank_enforced`.
- **FR-009**: Discovery and resolution evidence MUST include the applied tier
  policy, publisher/admission evidence identifiers, lifecycle state, applicable
  deadlines or minimum-safe version, and the selected or rejected record. It
  MUST NOT expose credentials, private support contacts, or artifact bytes.

## Acceptance Scenarios

1. Given Certified, Community, and Kit versions of matching records, when a
   consumer searches without a tier filter, then only Certified records are
   returned and exclusion evidence identifies the default policy.
2. Given an explicit Community opt-in, when a matching Community record is
   discovered, then it is returned with its tier and is never represented as
   Certified.
3. Given a purported Certified record missing signed provenance or maintainer
   policy evidence, when discovery or resolution evaluates it, then it is
   rejected with `registry_certification_evidence_missing` and explanatory
   evidence.
4. Given a deprecated Certified record, when its deprecation is published, then
   it names a replacement or migration path and a notice period of at least 90
   days; an invalid notice is rejected as `registry_lifecycle_invalid`.
5. Given a normal yanked version, when new preparation resolves it, then it
   fails with `registry_dependency_yanked` while an existing lock remains
   inspectable. Given a locally known security yank after its deadline, when a
   Certified host evaluates that lock, then it fails with
   `registry_security_yank_enforced` without a network request.

## Compatibility and Governed Files

This is additive to Specs 054, 055, and 520. Existing unfiltered CLI and MCP
discovery must adopt the Certified-only default only in a coordinated versioned
surface release; an explicit compatibility note and migration guidance are
required before that behavior is enabled. Implementations governed by this
specification are limited to:

- `contracts/registry/` for tier, admission, lifecycle, and lock evidence
  schemas;
- `crates/traverse-registry/`, `crates/traverse-cli/`, and
  `crates/traverse-mcp/` for validation, discovery, and evidence projection;
- `specs/525-tiered-registry-trust-lifecycle/` for this specification and its
  conformance fixtures; and
- focused registry/CLI/MCP conformance tests.

## Out of Scope

- Implementing signing, publisher identity verification, registry hosting, or
  a remote trust service.
- Cache preparation, activation, rollback, and cache storage policy (Spec 520
  successor work).
- Durable traces, DataStore migration, browser synchronization, and
  cross-platform certification execution.
- Automatically adding this draft to `approved-specs.json`.

## Independent Conformance Evidence

An independent suite must validate default discovery, every explicit opt-in
combination, missing Certified admission evidence, immutable state transitions,
90-day deprecation validation, normal-yank preparation rejection, and offline
security-yank enforcement from a lockfile fixture. Each fixture must assert
stable errors and structured resolution evidence.

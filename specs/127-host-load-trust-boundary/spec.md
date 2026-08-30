# Feature Specification: Host-Load Trust Boundary for Governed Public Bundles

**Feature Branch**: `claude/spec-127-host-load-trust-boundary`
**Created**: 2026-08-29
**Status**: Draft — architectural trust decision requiring maintainer approval before implementation.
**Input**: Traverse #1219; Specs 030, 118, 120, 124, 125, 126; `traverse-framework/registry` decision-log entries 74–79.

## Purpose

Define, once, what `traverse-cli serve` (and `load_registry_bundle`) validates
versus delegates when loading a registry bundle, keyed on the bundle's `scope`.
Specs 118 / 120 / 124 / 125 / 126 each defined one hop of the
`sync → prepare-public-bundle → materialize → serve` pipeline and were unit
tested against small hand-authored fixtures; each integration seam has since
failed when hit with the real published registry (Traverse #1203, #1211, #1215,
#1219). The common cause is that `serve`'s load path re-derives properties that
the source registry already established, permanently, at publish time.

## Trust Boundary

An immutable capability version published by a governed registry
(`traverse-framework/registry`) has already passed that registry's deterministic
publish gate: JSON-Schema validity, semver-bump-vs-contract-diff class,
digest format, dependency resolvability, artifact signature, and immutability.
Those properties are fixed for the lifetime of that version. A host that syncs
a **signed** public index from that registry and prepares a `scope: "public"`
bundle from it is loading content whose governance is already settled and
cryptographically attestable.

For such a bundle, `serve` MUST verify what it needs in order to execute safely,
and MUST NOT re-run the source registry's publish-time governance:

| `serve` verifies (executability + trust) | `serve` delegates to the source registry (publish-time governance) |
|---|---|
| Artifact digest (integrity) | Semver progression between versions |
| Ed25519 artifact signature, Spec 124 (authenticity) | Contract-diff compatibility classification |
| Host ABI version compatibility | Dependency-policy admissibility |
| Contract schema shape / parse | "Should this version have been published" |
| Bundle provenance (signed sync, known source) | |

`scope: "private"` and workspace bundles are locally authored, have no external
governance to delegate to, and retain today's full validation unchanged.

## Functional Requirements

- **FR-001**: For a `scope: "public"` registry bundle, bundle registration
  (`load_registry_bundle` / `serve` startup) MUST verify, per capability
  version: artifact digest, Ed25519 signature (Spec 124), host ABI
  compatibility, and contract schema parse. It MUST NOT run
  `validate_semver_progression`, contract-diff compatibility classification, or
  dependency-policy admissibility across the bundle's versions.
- **FR-002**: `scope: "private"` and workspace bundles MUST retain their
  current validation behavior in full. This spec changes nothing for locally
  authored content.
- **FR-003**: The reduced-validation path of FR-001 MUST apply only when the
  bundle's provenance is established as a governed public registry sync —
  every capability carries verifiable Spec-124 signature evidence and the
  bundle derives from a synced public index (Spec 055). A bundle missing that
  evidence, or of unknown provenance, MUST receive full validation.
- **FR-004**: Any compatibility metadata a consumer needs at runtime (for
  `registry_ref` semver-range resolution, deprecation-aware selection, etc.)
  MUST be read from the synced index / bundle, where the source registry
  recorded it at publish time, and MUST NOT be recomputed by the consumer.
  Producing that metadata in the public index (a verified `change_class` /
  `declared_bump` per version pointer) is `traverse-framework/registry`'s
  scope, tracked there; this FR only forbids the consumer-side recompute.
- **FR-005**: CI MUST run an end-to-end conformance test that executes
  `registry sync → registry prepare-public-bundle → registry materialize →
  serve --registry-state` against a fixture set representative of the real
  published registry — at minimum a capability id with three or more
  non-deprecated versions, one event-emitting capability with its referenced
  event, and a deprecated version present in the source index — and asserts
  that `serve` starts and executes one verified capability end to end.
- **FR-006**: No change to the source registry's publish-time validation, to
  `materialize`'s digest/signature verification, or to runtime execution-time
  signature enforcement (Spec 030). This spec narrows only what bundle
  *registration* re-derives.

## Acceptance Scenarios

1. Given a `scope: "public"` bundle prepared from a signed synced index whose
   capability set includes an id with four non-deprecated versions whose
   consecutive contract diff `classify_contract_change` reports as `Unknown`,
   `serve` registers all versions and starts — the progression check is not
   run.
2. Given the same bundle, every capability still fails registration if its
   artifact digest or Ed25519 signature does not verify.
3. Given a `scope: "private"` bundle with a semver-progression violation
   between two locally authored versions, registration still fails exactly as
   today.
4. Given a `scope: "public"` bundle in which one capability lacks Spec-124
   signature evidence, that bundle receives full validation (FR-003), not the
   reduced path.
5. The FR-005 conformance test runs in CI and fails if any pipeline hop
   regresses.

## Compatibility and Out of Scope

Additive and scope-gated. It does not weaken execution-time trust (Spec 030
signature enforcement is untouched), does not change private/workspace bundle
behavior, does not change what the source registry validates at publish, and
does not itself add the registry-side index metadata of FR-004 (tracked in
`traverse-framework/registry`). Workflows are out of scope until they are
carried through the public sync path.

## Approval Note

This draft MUST NOT be registered in `approved-specs.json` or used for a merged
implementation without explicit maintainer approval. Unlike Specs 125 / 126
(mechanical follow-ons from decisions already made), this defines an
architectural trust boundary — what a host is entitled to assume about content
from a governed registry — and should be reviewed as such.

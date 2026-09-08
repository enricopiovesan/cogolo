# Feature Specification: Mixed Local and Registry-Reference Activation

**Status**: Approved (2026-09-08)
**Canonical governing ID**: `130-mixed-registry-reference-activation`
**Version**: 1.0.0
**Extends**: `106-activation-artifact-resolution`,
`107-cross-host-embedded-registry-cache`
**Input**: #1258; ADR-0059 (`0059-mixed-registry-reference-activation`).

## Purpose

Allow an application containing local components and `registry_ref` components
to activate and execute offline, using only host-owned verified cache evidence
prepared before activation. This successor closes the activation boundary
explicitly excluded by Spec 107 without changing cache ownership or allowing
runtime re-resolution.

## Boundary and Ownership

| Concern | Owner |
| --- | --- |
| Syncing indexes and fetching artifacts | Host or deployment tooling before preparation |
| Selecting and verifying a `registry_ref` cache entry | Traverse resolver contract |
| Cache storage, paths, credentials, encryption, retention | Host |
| Combining local and verified registry candidates at activation | Traverse activation resolver |
| Activation evidence and drift checks | Traverse runtime/host integration |

## Requirements

- **FR-001**: Activation MUST accept mixed local and `registry_ref` components
  without rewriting the application manifest.
- **FR-002**: A `registry_ref` component MUST resolve only from a previously
  prepared verified cache entry. Activation, execution, and drift checking MUST
  perform no network request or fallback resolution.
- **FR-003**: Before activation succeeds, the resolver MUST verify namespace,
  capability id, requested version range, selected version, contract digest,
  artifact digest, signature/trust lifecycle, ABI, target, placement, and
  declared execution constraints.
- **FR-004**: Successful activation MUST persist deterministic immutable,
  non-secret evidence for each selected local or registry component, including
  identity, selected version, requested version range where applicable,
  contract and artifact digests, trust lifecycle, ABI, target, placement,
  resolver/cache evidence, connector evidence, and configuration-reference
  names only.
- **FR-005**: Evidence MUST NOT contain cache paths, configuration values,
  credentials, endpoints, or artifact bytes.
- **FR-006**: Missing, altered, inactive or yanked, trust-invalid,
  contract-mismatched, ABI-incompatible, target-incompatible, placement- or
  constraint-incompatible cache entries MUST fail closed with stable,
  secret-free errors.
- **FR-007**: Execution MUST consume activation evidence and MUST reject every
  selected-component drift without re-resolution.
- **FR-008**: Rust and Web implementations MUST expose equivalent observable
  evidence and error semantics. Native host conformance follows the Spec 107
  matrix.

## Acceptance Scenarios

1. A prepared signed registry component and a local component validate,
   register, activate, and execute offline from their recorded evidence.
2. A missing or modified cache entry fails closed and no alternate local or
   registry candidate is selected.
3. A yanked, inactive, trust-invalid, ABI-incompatible, wrong-target, or
   contract-digest-mismatched entry fails with deterministic non-secret
   evidence.
4. Execution after a post-activation mutation fails with activation drift and
   performs no network request.
5. Equivalent Rust and Web fixtures emit the same normalized result projection
   and omit all host-private values.

## Compatibility

This is additive. Existing local-only activation remains unchanged. Existing
Spec 107 cache APIs and error codes remain supported; this spec may add stable
activation-specific error codes but MUST NOT change their meanings. Any later
incompatible evidence-schema change requires a new major version and a
documented migration.

## Quality Gates

- Unit and integration tests cover every failure class and deterministic
  evidence projection.
- Offline fixtures prove no network request occurs after preparation.
- Cross-host conformance covers Rust, Web, and the applicable native matrix.
- Spec-alignment, contract validation, and targeted regression suites pass.

## Out of Scope

Runtime network fetching, manifest rewriting, a shared cache layout, cache
storage/secrets ownership, and silent artifact re-resolution.

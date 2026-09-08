# Feature Specification: Offline Activation from Verified Registry Cache Evidence

**Status**: Approved (2026-09-07)
**Canonical governing ID**: `1258-offline-cache-activation`
**Version**: 0.1.0
**Extends**: `106-activation-artifact-resolution` and `107-cross-host-embedded-registry-cache`.
**Decision evidence**: Traverse #1258 decision record (2026-09-07).

## Purpose

Define the explicit lifecycle by which a mixed local/Registry application
activates offline from already verified host-owned cache evidence, without
rewriting its manifest, fetching at activation, or re-resolving a substitute.

## Requirements

- **FR-001**: The only supported Registry lifecycle is explicit sync, verified
  preparation/registration, offline activation, then execution. Activation and
  execution MUST make no network request and MUST NOT silently re-resolve.
- **FR-002**: Activation MUST consume only a verified cache entry matching the
  `registry_ref` namespace/id, requested version range, selected version,
  contract digest, artifact digest, trust lifecycle, ABI, target, placement,
  and constraints. An equivalent host candidate without that evidence is not a
  substitute.
- **FR-003**: Successful activation MUST persist immutable, non-secret evidence
  for selected identity/range/version, digests, trust lifecycle, ABI, target,
  placement, resolver/cache evidence, and connector evidence. It MUST NOT
  record paths, values, credentials, endpoints, or artifact bytes.
- **FR-004**: Missing, altered, yanked/inactive, ABI-incompatible,
  target-incompatible, trust-invalid, contract-mismatched, or drifted entries
  MUST fail closed with stable secret-free errors.
- **FR-005**: Execution MUST consume activation evidence and re-check required
  immutable identifiers/digests without re-resolution. Any drift fails closed.
- **FR-006**: Rust and Web cache paths MUST expose equivalent evidence and
  errors; native coverage follows the Spec-107 conformance matrix.

## Acceptance scenarios

1. A signed Registry component and a local component validate, prepare,
   register, activate, and execute offline without manifest rewriting.
2. A missing, tampered, yanked, incompatible, untrusted, or contract-mismatched
   entry fails before execution with stable redacted evidence.
3. Modifying cache evidence or selected bytes after activation causes execution
   to fail closed without a network request or alternate candidate selection.

## Compatibility and non-goals

This successor is additive: Specs 106 and 107 remain immutable. It does not
define cache layout, host storage/secrets, runtime network fetches, shared
cache files, or manifest rewriting.

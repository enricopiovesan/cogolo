# Feature Specification: Materialized Artifact Signatures

**Status**: Draft
**Canonical governing ID**: `124-materialized-artifact-signatures`
**Extends**: `030-security-identity-model`, `120-host-owned-artifact-preparation`, and ADR-0051
**Tracks**: #1203; depends on `traverse-framework/registry` #333, #334, and #335.

## Purpose

Carry publisher-supplied Ed25519 verification metadata from the registry's
additive signature sibling files through the host-owned artifact preparation
manifest into the existing runtime artifact verifier. This makes the governed
signature requirement in Spec 030 reachable through `registry materialize`
and `serve --registry-state --artifact-state`, without granting `serve` any
network, key-discovery, or state-mutation responsibility.

## Ownership and boundary

The registry owns publishing immutable artifact bytes, a public verification
key, and an additive signature sibling file. The host-owned `materialize`
step fetches and validates those inputs before writing local state. The
artifact-state manifest is the only persisted handoff to `serve`; it remains
separate from Spec 118's registry-bundle manifest. `serve` only reads local
state, re-verifies the binary digest, and passes validated signature metadata
to the runtime's existing verification boundary.

## Requirements

- **FR-001**: For every executable capability, `registry materialize` MUST
  locate the registry's additive signature sibling file, `signature.json`, in
  the same directory as the capability's `contract.json`, and require a
  `scheme` of `ed25519`, a 64-hex-character `public_key_hex`, and a
  128-hex-character `signature_hex`. Only `ed25519` is supported by this
  slice; a missing sibling file, an unknown scheme, or any missing, empty, or
  malformed field MUST fail with a stable, capability-qualified error. The
  signature values are inline hex, not URLs -- this matches
  `traverse-framework/registry` Spec 007 v1.1.0, which publishes
  `signature.json` (fields `scheme`, `public_key_hex`, `signature_hex`,
  `sigstore_bundle_ref`, `signed_at`) as an immutable additive sibling because
  `contract.json` is frozen at merge time, before signing occurs.
- **FR-002**: After digest-verifying the artifact bytes, `materialize` MUST
  verify the Ed25519 `signature_hex` over those exact bytes using
  `public_key_hex`. The signature material is read inline from `signature.json`
  -- no network fetch for signature evidence. `materialize` MAY additionally
  pin `public_key_hex` against the registry's published well-known key
  (`signing-key.pub` on the catalog site) on first use. Invalid encodings, a
  byte-mismatched signature, or a public key that fails an enabled pin check
  MUST fail closed and MUST NOT create a state entry for that capability.
- **FR-003**: Each executable artifact-state entry MUST persist the additive
  signature evidence needed by `ArtifactSignature`: `scheme`,
  `public_key_hex`, and `signature_hex`. This spec defines artifact-state
  schema version `1.1.0`; loaders supporting `1.0.0` MUST preserve existing
  unsigned manifest behavior, while `1.1.0` requires complete signature
  evidence for every executable entry.
- **FR-004**: Before `serve` registers a capability from a `1.1.0`
  artifact-state manifest, it MUST reject malformed or incomplete signature
  evidence and must retain existing whole-manifest digest verification.
- **FR-005**: On successful validation, `serve` MUST construct the resolved
  executable binary with the artifact-state entry's `ArtifactSignature` so
  Spec 030 governs verification at execution. It MUST NOT bypass, duplicate,
  or weaken the runtime verifier.
- **FR-006**: `serve` and runtime execution MUST perform no network fetch for
  artifact signature evidence. Key rotation, Sigstore, detached provenance,
  and workflow-level artifacts are out of scope.

## Compatibility

This is an additive artifact-state schema successor. Existing `1.0.0`
manifests remain loadable with their established behavior. A `1.1.0` manifest
is rejected by an older loader rather than silently dropping required security
metadata. Registry `contract.json` files remain byte-for-byte unchanged: the
signature travels in the sibling `signature.json`, read only by
materialization.

## Acceptance scenarios

1. A registry capability with a valid Ed25519 artifact signature materializes
   into `1.1.0` state, starts under `serve`, and executes through the verified
   endpoint with `signature_verified` runtime evidence.
2. A missing sibling file, or a malformed or byte-mismatched signature, fails
   materialization with a stable capability-qualified error and creates no
   artifact-state entry for that capability.
3. An artifact-state entry with malformed or incomplete signature metadata
   causes `serve` startup to fail closed before listening.
4. An existing `1.0.0` artifact-state fixture continues to load under its
   documented behavior, and an unsupported schema version remains rejected.
5. Altering a materialized binary after preparation causes the existing
   startup digest check to fail before runtime signature verification.

## Quality gates

- Unit tests cover every accepted and rejected signature field combination,
  encoding failure, and signature mismatch.
- Integration tests cover materialize-to-serve-to-verified-execution with a
  real Ed25519 test key and no runtime network access.
- Tests preserve the `1.0.0` manifest compatibility path and fail closed for
  invalid `1.1.0` manifests.

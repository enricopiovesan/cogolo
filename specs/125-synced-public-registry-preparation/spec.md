# Feature Specification: Synced Public Registry Bundle Preparation

**Feature Branch**: `codex/issue-1211-synced-registry-bridge`
**Created**: 2026-08-29
**Status**: Draft — successor specification requiring maintainer approval before implementation.
**Input**: Traverse #1211; Specs 055, 118, 120, and 124.

## Purpose

Define the host-run bridge from the durable public index produced by `traverse-cli
registry sync` to the local Spec-118 registry-bundle manifest consumed by
`registry materialize` and `serve`. The bridge prepares verified local contract
and signature evidence without adding request-time network access to `serve` or
changing the public registry index schema.

## Capability Boundary

`registry prepare-public-bundle` is a host-owned preparation capability. Its
input is one explicit synced-public-index state path and its output is one
complete, local registry-bundle directory. It fetches and verifies only the
published contract and additive signature evidence needed to create that
bundle. Artifact download and verification remain owned by the existing
`registry materialize` capability; `serve` remains an offline read-only
consumer of the resulting prepared state.

## Functional Requirements

- **FR-001**: `traverse-cli registry prepare-public-bundle --synced-state
  <path> --out <dir>` MUST accept only a valid, versioned
  `SyncedPublicRegistryState` produced by Spec 055. It MUST reject an absent,
  malformed, unsupported, or unverified state with stable, secret-free errors.
- **FR-002**: For every non-deprecated capability pointer in the synced state,
  preparation MUST fetch `contract_url`, verify the downloaded bytes against
  `contract_digest`, and validate that the contract identity and version match
  that pointer. It MUST NOT fetch artifacts in this step.
- **FR-003**: Preparation MUST fetch the additive `signature.json` sibling in
  the same immutable directory as each `contract_url`. It MUST validate the
  Spec-124 `ed25519`, `public_key_hex`, and `signature_hex` evidence before
  writing it beside the corresponding local `contract.json`. A missing,
  malformed, or identity-mismatched sibling MUST fail closed with a
  capability-qualified error.
- **FR-004**: On success, preparation MUST atomically emit a Spec-118
  `bundle.json` with `scope: "public"` and one local, relative contract path
  for every prepared non-deprecated capability. The generated bundle MUST
  preserve exact capability identities and versions. Events and workflows are
  empty until separately prepared under their owning specs.
- **FR-005**: A failed fetch, digest mismatch, signature validation failure,
  invalid path segment, or output-write failure MUST leave no partial bundle
  usable at `--out` and MUST preserve any prior complete output generation.
- **FR-006**: The command MUST make all network access during explicit host
  preparation only. `registry materialize`, `serve`, and runtime execution
  MUST retain their existing boundaries: materialize fetches and verifies the
  artifact; serve and runtime make no network request for contracts, artifacts,
  or signature evidence.
- **FR-007**: The command MUST emit stable JSON evidence containing the input
  state path, source provenance already recorded by sync, output bundle path,
  and prepared capability count. Evidence MUST contain no credentials or raw
  artifact bytes.
- **FR-008**: Existing `registry sync`, `registry materialize`, and `serve`
  flags and behavior MUST remain compatible. Operators opt in by passing the
  generated `bundle.json` to `materialize`, then pass that bundle and its
  artifact state explicitly to `serve`.

## Command Shape

```text
traverse-cli registry sync --workspace proxy --json
traverse-cli registry prepare-public-bundle \
  --synced-state .traverse/workspaces/proxy/registry/public/index.json \
  --out .traverse/workspaces/proxy/registry/prepared
traverse-cli registry materialize \
  --registry-state .traverse/workspaces/proxy/registry/prepared/bundle.json \
  --out .traverse/workspaces/proxy/artifacts
traverse-cli serve \
  --registry-state .traverse/workspaces/proxy/registry/prepared/bundle.json \
  --artifact-state .traverse/workspaces/proxy/artifacts/artifact-state.json
```

## Acceptance Scenarios

1. Given a valid synced index with non-deprecated pointers, preparation writes
   a complete local bundle whose contracts and signature siblings verify; that
   bundle can be passed to `materialize` without format translation.
2. Given a pointer whose fetched contract digest or identity does not match the
   synced state, preparation fails closed and preserves the prior output.
3. Given a missing or malformed adjacent `signature.json`, preparation emits a
   stable capability-qualified error and writes no usable generation.
4. Given a complete prepared bundle, materialization and verified-entrypoint
   serving execute using explicit local paths and make no serve/runtime network
   request.
5. Given a deprecated pointer, preparation excludes it deterministically and
   reports the resulting prepared capability count.

## Compatibility and Out of Scope

This is additive to Specs 055, 118, 120, and 124. It does not change the
synced-index format, mutate published registry files, add a default path,
merge public and private registrations, prepare workflows, or introduce
request-time refresh. Registry-index signature-reference fields and a generic
artifact cache are deferred; deriving the immutable sibling from `contract_url`
is the only registry-side location rule in this slice.

## Approval Note

This draft MUST not be registered in `approved-specs.json` or used for a
merged implementation without explicit maintainer approval.

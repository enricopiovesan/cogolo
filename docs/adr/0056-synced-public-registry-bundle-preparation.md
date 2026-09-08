# ADR-0056: Bridge Synced Public Index State Through an Explicit Prepared Bundle

- Status: Accepted
- Date: 2026-08-29
- Governing spec: `125-synced-public-registry-preparation` (Approved)
- Extends: Specs 055, 118, 120, and 124; ADR-0051 and ADR-0055
- Related issues: #1211, #1168, #1158, and `traverse-framework/registry` #328

## Context

`registry sync` deliberately writes pointer-only public index state, while
Spec-118 registration and Spec-120 materialization deliberately consume a
local bundle containing contract paths. Treating either representation as the
other fails before artifact preparation and leaves the documented public
execution path unusable. Moving resolution into `serve` would violate the
host-owned, offline serving boundary.

## Decision

Introduce one explicit host-run `registry prepare-public-bundle` command. It
reads synced public state, verifies each non-deprecated contract against its
recorded digest, derives and verifies the immutable adjacent `signature.json`
sibling, and atomically writes a local Spec-118 `bundle.json` and relative
contract tree. Existing `registry materialize` then owns artifact fetching;
existing `serve` consumes only the explicit local bundle and artifact state.

## Consequences

The public execution path gains a single format transition with a clear trust
boundary and stable operator evidence. The registry index remains compatible,
and no server or runtime network capability is added. Hosts must run one more
explicit preparation command; partial preparation fails rather than exposing a
subset of capabilities.

## Alternatives Considered

- Make `materialize` infer a bundle directly from an index: rejected because it
  combines contract preparation and artifact materialization into one opaque
  operation and weakens diagnosability.
- Make `serve` fetch contracts or artifacts: rejected by Specs 118 and 120's
  offline, host-owned state boundary.
- Add signature URLs to the registry index first: deferred because immutable
  sibling derivation from the already indexed contract URL is sufficient and
  keeps this slice additive.

## Approval evidence

The maintainer approved Spec 125 on 2026-08-29 after review against the live
synced-index and published `signature.json` shape. The approved-spec registry
records `125-synced-public-registry-preparation` at version 0.1.0. This ADR
records that same approved decision; it does not alter the immutable spec.

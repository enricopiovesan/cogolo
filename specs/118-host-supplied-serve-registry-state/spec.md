# Host-Supplied Verified Registry State for `serve`

**Status**: Draft
**Canonical governing ID**: `118-host-supplied-serve-registry-state`
**Version**: 0.1.0
**Input**: Traverse #1142; unblocks #1105.

## Purpose

Define the host-owned, verified registry-state boundary used by `traverse-cli
serve`. It lets a server resolve exact public capability and workflow
entrypoints without using the canonical expedition bundle, caller-owned
filesystem input, or request-time synchronization.

## Ownership and boundary

The embedding or deployment host owns creation, storage, permissions, and
refresh of registry state. Before starting, `serve` receives one explicit path
to a host-authored, versioned JSON state manifest. Traverse validates the
entire manifest and all declared verification bindings before accepting
requests.

The server is only a read-only consumer of this prepared state. It MUST NOT
invent a default state location, fetch, sync, refresh, repair, or fall back to
the expedition bundle, private registrations, or an in-process development
registry.

## Requirements

- **FR-001**: `serve` MUST require one explicit path to a host-authored
  verified-registry state manifest before enabling verified entrypoint
  resolution.
- **FR-002**: The manifest MUST be a versioned JSON document. It MUST reject
  unknown or missing required fields, unsupported schema versions, and
  incompatible successor versions with stable, secret-free errors.
- **FR-003**: The manifest MUST describe complete verified state for both
  capabilities and workflows. Each entrypoint MUST bind exact kind, identity,
  version, public lifecycle/provenance, artifact identity, and the evidence
  required by the normal artifact-verification gate.
- **FR-004**: The server MUST validate the manifest atomically before
  listening. It MUST either expose one complete verified state generation or
  fail closed; partial, mixed, stale-unverified, or independently assembled
  state MUST never become executable.
- **FR-005**: Exact resolution MUST reject a request whose declared
  `entrypoint_kind`, id, or version does not exactly match a verified manifest
  entry. A capability MUST NOT resolve as a workflow, and vice versa.
- **FR-006**: An absent manifest, malformed manifest, unavailable declared
  state, invalid verification binding, or unprepared artifact MUST fail closed
  with stable actionable errors. None may be represented as an empty registry
  or satisfied by a fallback.
- **FR-007**: When verified state is present, `serve` MUST preserve the
  authentication modes, authorization outcomes, request-size limits, CORS
  behavior, existing routes, and response-redaction rules governed by Spec
  033 and Spec 115.
- **FR-008**: State-manifest migration MUST be explicit. A server that does
  not support a manifest version MUST reject it rather than apply a lossy
  conversion or infer omitted verification data.

## Acceptance scenarios

1. A host supplies one complete manifest containing verified capability and
   workflow entries; the server validates it before listening and can resolve
   each entry only by exact matching kind, id, and version.
2. A manifest that omits workflow state, has a mismatched artifact binding, or
   uses an unsupported version prevents verified entrypoint serving and emits
   a stable actionable error without network activity.
3. A request that names a valid id/version with the wrong kind is rejected
   without attempting cross-kind resolution.
4. A server whose manifest or artifact preparation is absent never substitutes
   the expedition bundle, a private registration, or an in-process registry.
5. Existing HTTP routes retain their established authentication, CORS, size,
   and response-redaction behavior.

## Validation

- Manifest fixtures cover valid capability and workflow state, every required
  missing field, unknown field, malformed JSON, unsupported version, partial
  generation, verification mismatch, and unavailable artifact.
- HTTP integration tests cover exact kind/id/version resolution, cross-kind
  rejection, absent/unprepared state, authorization, CORS, and no-fallback
  behavior.
- Offline tests prove server startup and entrypoint requests make no network
  request and do not mutate host state.

## Compatibility

This is additive to the existing `serve` API. Existing routes and their
contracts remain unchanged. Verified entrypoint execution is unavailable until
the host opts in with a valid manifest. Future manifest changes require a
successor versioned artifact and explicit migration rule.

## Non-goals

- An implicit `.traverse` or other default production state path.
- Caller-provided local artifact paths, request-time registry refresh, or
  fallback to expedition/private/development state.
- Hosted MCP, browser authentication changes, tenancy, or a new remote
  gateway.

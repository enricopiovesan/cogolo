# Browser-Reachable Verified Entrypoint Execution

**Status**: Approved (2026-08-24)
**Canonical governing ID**: `115-browser-verified-entrypoint-execution`
**Version**: 0.1.0
**Input**: Traverse #1124; successor work following ADR-0044 and Spec 033.

## Purpose

Define an additive `traverse-cli serve` HTTP endpoint that lets browser clients
execute an exact governed entrypoint without a filesystem request path, caller
pre-registration, a fixed expedition bundle, or a new transport/security
system.

## Requirements

- **FR-001**: The existing HTTP server exposes one additive versioned endpoint
  for entrypoint execution and preserves all existing routes unchanged.
- **FR-002**: Its JSON body requires `entrypoint_kind`, exact `id`, exact
  `version`, and inline `request` serialized as `RuntimeRequest`.
- **FR-003**: Missing/unknown kind, missing identity, non-exact resolution,
  malformed request, and verification/readiness failures return stable,
  actionable HTTP error envelopes.
- **FR-004**: Resolution uses only the server's already-synced,
  digest-verified registry state. It MUST NOT fetch, sync, refresh, or fall
  back to the expedition bundle during execution.
- **FR-005**: The resolved artifact MUST pass the normal verification gate
  before runtime handoff. An absent/unprepared cache fails closed.
- **FR-006**: Authentication modes, authorization outcomes, request-size
  limits, and CORS behavior are exactly those already governed by Spec 033;
  this endpoint adds no token or browser-specific fallback.
- **FR-007**: Responses contain runtime-owned results and redacted public
  trace/evidence only; requests, secrets, and unredacted diagnostics are not
  exposed by default.

## Compatibility and scope

This is an additive HTTP surface. `POST /v1/capabilities/execute` remains its
own pre-registration path. This spec does not define hosted MCP, WebSocket MCP,
caller-managed request files, or a browser credential protocol.

## Acceptance scenarios

1. A loopback browser request executes a verified public capability by exact
   identity with an inline request.
2. A workflow and a capability resolve only when their declared kind matches.
3. An absent, invalid, or unverified registry state fails closed without
   network access or expedition fallback.
4. A bearer-required listener rejects unauthenticated calls and CORS behavior
   matches existing routes.
5. Responses expose no raw request or secret material.

## Validation

- HTTP integration tests for all acceptance scenarios and stable error codes.
- Verification-gate and CORS/auth regression tests.
- `bash scripts/ci/spec_alignment_check.sh` with the PR body.

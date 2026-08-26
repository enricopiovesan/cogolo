# Host-Owned Artifact Preparation for `serve`

**Status**: Approved (2026-08-26)
**Canonical governing ID**: `120-host-owned-artifact-preparation`
**Version**: 0.1.0
**Input**: Traverse #1153; unblocks #1153.

## Purpose

Define a host-owned artifact-preparation manifest and the `traverse-cli
registry materialize` command that produces it, so `traverse-cli serve
--registry-state` (spec 118) can resolve and execute a capability's real
WASM artifact instead of a fabricated `bundled://` path — without `serve`
itself ever fetching, syncing, or mutating state during startup or
execution (spec 115 FR-004, spec 118 FR-004/FR-006).

## Ownership and boundary

Spec 118's registry-bundle manifest (identity/scope of what's registered)
is governed and untouched by this spec. This spec governs a separate,
cross-referenced artifact-preparation manifest (`artifact-state.json`)
describing where each capability's real, digest-verified WASM binary was
materialized. `serve` discovers it via a new, independent `--artifact-state
<path>` flag — never a default location, never embedded in spec 118's own
schema.

`traverse-cli registry materialize` is the sole sanctioned way to produce
this file: it fetches each capability's `contract.artifact.url`, verifies
the downloaded bytes against `contract.artifact.digest`, writes verified
bytes to a host-chosen output directory, and emits the artifact-state
manifest. `serve` never fetches; it only re-verifies and reads.

## Requirements

- **FR-001**: `traverse-cli registry materialize --registry-state <path>
  --out <dir>` MUST fetch, for every capability listed in the given
  spec-118 registry-bundle manifest, the artifact named by that
  capability's contract's `artifact.url`.
- **FR-002**: `materialize` MUST verify each fetched artifact's digest
  against the contract's declared `artifact.digest` before writing it to
  `--out`. A mismatch MUST abort materialization for that entry with a
  stable, actionable error; it MUST NOT write unverified bytes.
- **FR-003**: `materialize` MUST emit a versioned `artifact-state.json`
  binding, for each successfully materialized capability, its exact `id`,
  `version`, real local `path` (relative to the artifact-state manifest),
  verified `digest`, `source_url`, and `materialized_at` timestamp.
- **FR-004**: `serve` MUST accept an explicit, independent `--artifact-state
  <path>` flag. It MUST NOT invent a default location, and this spec's
  manifest MUST NOT be embedded into or discovered via spec 118's
  registry-bundle manifest.
- **FR-005**: Before listening, `serve` MUST re-verify every referenced
  artifact's on-disk digest against `artifact-state.json`'s declared
  digest. It MUST NOT trust `materialize`'s prior verification as
  sufficient on its own.
- **FR-006**: If any capability declared in the `--registry-state` manifest
  has no corresponding, verifying entry in `--artifact-state`, `serve` MUST
  fail closed for the entire startup — no subset of verified capabilities
  may be served while others are silently excluded.
- **FR-007**: This spec applies only to `capabilities[]` entries. Workflow
  entrypoints compose already-covered capability artifacts and carry no
  artifact of their own.
- **FR-008**: `artifact-state.json` MUST be a versioned JSON document;
  migration MUST be explicit, matching spec 118's existing FR-008
  convention.

## Acceptance scenarios

1. Given a spec-118 registry-bundle manifest naming 5 real, published
   capabilities, `materialize --out ./artifacts` fetches and
   digest-verifies all 5 and emits an `artifact-state.json` with 5 entries.
2. Given one capability's downloaded bytes don't match its declared digest,
   `materialize` aborts with a stable error naming that capability and does
   not write partial/tampered bytes for it.
3. Given `serve --registry-state <m> --artifact-state <a>` where `<a>`
   covers every capability in `<m>` and all on-disk digests re-verify,
   `serve` starts and can execute any of them via the verified-entrypoint
   endpoint (spec 115).
4. Given `<a>` is missing an entry for one capability declared in `<m>`,
   `serve` refuses to start at all — not a partial server.
5. Given an on-disk artifact's bytes have changed since `materialize` ran
   (tampering, stale reuse, filesystem corruption), `serve`'s startup
   re-verification catches the mismatch and refuses to start.

## Validation

- `materialize` fixtures: successful fetch+verify, digest mismatch abort,
  network failure, malformed contract artifact fields.
- `serve` startup fixtures: complete matching state (success), missing
  artifact-state entry (fail closed), tampered on-disk binary (fail
  closed), unsupported artifact-state schema version (fail closed).
- Integration: real end-to-end run against a live, non-deprecated registry
  capability, executed through the verified-entrypoint HTTP endpoint.

## Compatibility

Additive to spec 118 and spec 115: no existing field, flag, or behavior of
either changes. `serve` without `--artifact-state` behaves exactly as it
does today (verified-entrypoint execution unavailable, per spec 118's
existing "unavailable until the host opts in" framing) — this spec adds a
second, independent opt-in, not a replacement.

## Non-goals

- Modifying spec 118's registry-bundle manifest schema.
- A generic/live artifact cache with request-time refresh — materialization
  is a discrete, host-run, pre-serve step, not a `serve`-owned or
  per-request mechanism.
- Workflow-level artifacts.
- Automatic re-materialization on a schedule; when/how often to re-run
  `materialize` is an operational decision for whoever hosts `serve`, out
  of scope here.

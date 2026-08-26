# ADR-0051: Host-Owned Artifact Preparation as a Separate, Cross-Referenced Manifest

- Status: Accepted
- Governing specs: `120-host-owned-artifact-preparation`; related `115-browser-verified-entrypoint-execution`, `118-host-supplied-serve-registry-state`
- Related issues: #1153

## Context

`serve --registry-state` (spec 118) always fabricates a synthetic
`bundled://{id}/{version}/module.wasm` artifact location for every
registered capability, since the registry-bundle manifest carries only a
`contract.json` path, never a real WASM location. `#1153` proposed fixing
this inside `serve` itself (fetch the contract's `artifact.url`, verify the
digest, at load or request time), but that directly violates spec 115
FR-004 and spec 118 FR-004/FR-006, which require `serve` to consume only
already-prepared, already-verified host state and forbid any fetch, sync,
or state mutation during startup or execution.

## Decision

Real artifact preparation becomes a separate, host-run step, governed by a
new spec (`120`) rather than a modification to the already-Approved,
immutable spec 118:

1. A new, first-party `traverse-cli registry materialize` subcommand
   fetches each capability's real WASM artifact, verifies it against the
   contract's declared digest, and writes a new `artifact-state.json`
   manifest — kept entirely separate from spec 118's registry-bundle
   manifest, cross-referenced only via a new, independent `serve
   --artifact-state <path>` flag (never embedded in or auto-discovered
   from spec 118's own schema, and never a default/conventional location).
2. `serve` re-verifies every artifact's on-disk digest against the
   declared digest at startup, rather than trusting `materialize`'s prior
   verification alone — defense in depth, consistent with the
   artifact-verification-gate posture the rest of this lineage already
   holds.
3. Failure is whole-manifest fail-closed: a missing or mismatched artifact
   for any declared capability refuses the entire `serve` startup, never a
   silently reduced subset — matching spec 118 FR-004's existing
   "partial... state MUST never become executable."
4. Scope is capabilities only; workflows compose already-covered
   capability artifacts and carry no artifact of their own.

## Consequences

`#1153` is unblocked once spec 120 lands: it becomes "implement `registry
materialize` and `serve --artifact-state` per spec 120" rather than a
design question. Hosting a real, publicly-reachable `serve` instance
against the live registry (the motivating discover.html use case) still
requires someone to actually run `materialize` and operate the resulting
artifacts directory, and re-run it on whatever cadence they choose to pick
up new registry publishes — that operational question is explicitly out of
this spec's scope.

## Alternatives considered

- Extend spec 118's manifest schema directly with artifact-location/digest
  fields: rejected — spec 118 is Approved and immutable; this org's own
  precedent (ADR-0043 adding a new phase to spec 108 rather than modifying
  spec 109) favors a new governing artifact over amending an approved one,
  and spec 118 FR-002's strict unknown-field rejection means even one
  additive field requires a formal successor version regardless.
- Fixed-name sibling file discovered by convention next to
  `--registry-state`, no new flag: rejected — spec 118's own Ownership
  section already explicitly forbids `serve` inventing a default state
  location; a convention-based second file repeats exactly the pattern
  this lineage has consistently rejected.
- Keep artifact fetch+verification an external, host-authored script
  (matching how the registry-state manifest generator itself stayed
  external): rejected — unlike manifest generation (filtering/reshaping
  already-public metadata), digest verification is the actual security
  boundary stopping a tampered or wrong binary from executing; leaving it
  to per-host reimplementation risks a subtly wrong verification nobody
  audits, versus shipping it once as tested, first-party code.
- Trust `materialize`'s verification without startup re-verification in
  `serve`: rejected — no protection against anything mutating the
  artifacts directory in the window between `materialize` running and
  `serve` starting, which is a real assumption for a public-facing host to
  carry unchecked.

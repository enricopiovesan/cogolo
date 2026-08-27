# Cross-host fixture comparison contract

Tracks issue #1161 and supplies the comparison boundary for the cross-host
conformance work in #1162 through #1165. It applies to one pinned WASM
capability artifact and its pinned contract run by a browser host, a CLI/Node
host, and one native host. It is an evidence contract only: it does not create
a platform certification program, require a five-target matrix, or require
byte-for-byte trace equality.

## Fixture identity

Every fixture release has an immutable `fixture_version` and identifies one
capability by all of the following:

- `capability_id` and `capability_version`
- `artifact_digest` (`sha256:<lowercase-hex>`)
- `contract_id`, `contract_version`, and `contract_digest`
- `input_id` and `expected_projection_version`

The fixture input and expected projection are repository-controlled test data.
They are not copied into evidence records. A host must reject an artifact whose
digest differs from `artifact_digest` before attempting execution. That outcome
is `artifact_identity_failure`.

## Evidence record

Each run emits one JSON evidence record with this public shape. Fields not
listed here are not part of comparison.

```json
{
  "fixture_version": "1.0.0",
  "capability_id": "example.capability",
  "capability_version": "1.0.0",
  "artifact_digest": "sha256:...",
  "contract_id": "example.contract",
  "contract_version": "1.0.0",
  "contract_digest": "sha256:...",
  "host": { "package": "@traverse/web", "version": "1.0.0" },
  "engine": { "name": "wasmtime", "version": "44.0.3" },
  "platform": { "os": "macos", "architecture": "aarch64" },
  "outcome": "success",
  "output_projection": { "status": "success", "reason_code": "ok" },
  "trace_projection": { "terminal_state": "completed", "event_kinds": ["started", "completed"] },
  "comparison": { "result": "equal", "projection_version": "1.0.0" }
}
```

`host.package` and `host.version` identify the package under test, not an app
or device name. `engine` identifies the runtime engine used for the run.
`platform` records the executing OS and architecture. A result is incomplete,
and therefore fails comparison, if any required identity, environment, outcome,
or comparison field is absent.

## Canonical projections

The fixture's expected projection defines the comparison target.

- `output_projection` is a recursively key-sorted JSON object containing only
  contract-observable result fields selected by the fixture, including the
  terminal `status` and stable `reason_code` when the contract defines them.
  Numbers, strings, booleans, arrays, and `null` retain their JSON semantics;
  array order is significant unless the fixture explicitly defines a sorted
  array projection.
- `trace_projection` is a recursively key-sorted JSON object containing only
  safe lifecycle evidence selected by the fixture, such as terminal state,
  stable event kinds, policy decision codes, and failure category. It must not
  contain request bodies, output bodies, payload fragments, credentials, or
  local paths.

Hosts compare both projections structurally after key sorting. An absent key,
additional key, type difference, scalar difference, or significant array-order
difference is unequal. Hosts may render human-readable diagnostics separately;
those diagnostics are not comparable evidence.

## Permitted host variance

The following values may be retained as local diagnostics or environment
metadata but are excluded from equality checks: execution and wall-clock
timestamps, generated execution/request/trace IDs, host identity, device name,
OS and architecture, host-package version, engine name/version, performance
measurements, and documented engine metadata. Their presence must never mask a
projection mismatch.

## Outcomes and comparison semantics

The fixture specifies one of three expected outcome classes:

| Expected class | Required observed outcome | Comparison result |
| --- | --- | --- |
| `success` | Verified artifact executes and both projections equal the expected projections. | `equal` / pass |
| `invalid_input` | Contract validation rejects the pinned invalid input with its expected stable failure code, without successful capability execution. | `equal` / pass |
| `artifact_identity_failure` | Artifact or contract identity verification fails before execution with the expected stable identity failure code. | `equal` / pass |

Any other outcome, an execution after an expected rejection, missing evidence,
or unequal projection is `not_equal` / fail. A host implementation failure that
prevents a record from being emitted is also fail; it is not silently skipped.

## Native profile evidence

The selected native host is `traverse-swift-host` using Wasmi 1.1.0 under
ADR-0014 and approved Spec 121. `scripts/ci/cross_host_native_fixture.sh`
executes the pinned artifact with the bounded WASI command profile, verifies
the artifact and contract digests, validates the two rejection fixtures, and
emits the required safe success record. CI runs it on macOS; it deliberately
does not claim or imply conformance for additional native platforms.

## Redaction requirements

Evidence is safe by construction. It must exclude raw request and output
payloads; credentials, tokens, headers, and secret-like values; device-local
or cache paths; network addresses; user identifiers; and opaque trace payloads.
If a value is needed to demonstrate behavior, the fixture must project a stable
category, boolean, count, digest, or contract-defined reason code instead of
the original value. Redaction is fail-closed: a host that cannot produce the
required safe projection must emit no successful comparison result.

## Producer and verifier responsibilities

Each host producer writes one evidence record for every fixture run. The shared
verifier first validates the identity fields and safe shape, then evaluates the
expected outcome class, then compares the canonical projections. A conformance
report lists each host record and its `comparison.result`; it must not aggregate
away an individual host failure.

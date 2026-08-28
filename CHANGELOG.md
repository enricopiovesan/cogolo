# Changelog

## Unreleased

## v0.10.0 — 2026-08-28

- Added governed runtime workflow proposals: lifecycle operations, bounded
  deterministic parallel scheduling, declarative planning, and promotion
  through the MCP host.
- Added a governed browser proposal journey, verified browser capability
  execution with receipts, and the server-side verified entrypoint endpoint.
- Added standalone capability-package discovery and activation-time executable
  artifact resolution, with stricter artifact-metadata validation.
- Added universal local connector contracts, mediated connector invocation,
  application connector bindings, and native verified-registry metadata caches.
- Added durable orchestration controls and a durable trace journal, alongside
  cross-host fixture evidence and browser/CLI comparison coverage.
- Added host-owned, privacy-preserving authoring telemetry configuration and
  aggregation, disabled by default.
- Updated the registry dependency to 0.15.2 and refreshed selected Rust
  dependencies.

## v0.9.1 — 2026-08-09

- Capability packages can be scaffolded, inspected, validated, and published through
  the CLI. Publishing now records executable artifacts with registry records and
  rejects unresolved persona references, including during dry runs.
- The runtime now carries real events emitted by executable capabilities through the
  `LocalExecutor`; the retired output-JSON convention is no longer the execution path.
- Added governed, end-to-end-smoked examples for `core.authorize`,
  `core.process-comment`, `core.transition-action-status`,
  `core.assign-ownership`, and `core.validate-action-item`.
- Added contract-surface coverage enforcement and corrected the
  `core.process-comment` published surface with an honesty bump.
- Fixed the CLI release package so its generated protobuf source is included in the
  v0.9.1 crate artifact.

- Kotlin and Swift embedder SDKs: marshal native bridge requests and map
  typed runtime bridge results, with a CI-enforced embedder conformance
  suite for both platforms (specs 071, 072).
- Kotlin embedder SDK: bound runtime execution against the Chicory Wasm
  runtime.
- Hardened durable event revocation to await completion instead of firing
  and forgetting, with test coverage for revocation failure paths.
- Recorded the decision to retain the durable event journal after
  operational evaluation (see `docs/decision-log.md` and
  `docs/adr/durable-journal-after-operational-evaluation.md`).

## v0.8.0 — 2026-07-16

See [docs/releases/v0.8.0.md](docs/releases/v0.8.0.md) for full release notes.
Highlights: public `traverse-embedder` Rust SDK and `traverse-embedder-web`
TypeScript SDK with real bundle execution (spec 068); the durable event
journal, including journal-backed replay through `subscribe`/`poll`; the
deterministic `doc-approval.recommend` capability and canonical
`doc-approval.pipeline` workflow (spec 069); HTTP API connection timeout
hardening, Sigstore placeholder-evidence rejection, and approved-specs-based
governed-artifact classification; and idempotency fixes across the event and
capability registries.

# Changelog

## Unreleased

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

# ADR-0060: Portable Audio and Model-Runtime Authority Boundaries

- Status: Proposed
- Date: 2026-09-07
- Governing spec: `1259-portable-authority-contracts` (Draft)
- Related issue: #1259

## Context

Applications need reusable boundaries for physical audio capture and model
activation without turning microphone, provider, or product workflow choices
into public portable contracts. The existing connector architecture already
separates abstract requirements, application bindings, and host-owned private
configuration.

## Decision

Add native-only `traverse.audio-input` and vendor-neutral
`traverse.model-runtime` connector contracts under that architecture. Keep
portable audio/DSP algorithms and request/policy preparation in WASM
capabilities. Defer an advisory-review connector; compose through
`traverse.http` until a reusable authority is evidenced. Permit a future
media-generic acceleration connector only for unavoidable host/hardware work.

## Consequences

Contracts gain typed, bounded authority envelopes and target-aware redacted
evidence without vendor, device, endpoint, credential, or UI leakage. Native
authorities fail before invocation on browsers. Future connector proposals
need conformance fixtures and a separately justified host boundary.

## Alternatives considered

- Put capture/model activation inside WASM capabilities: rejected because it
  hides host authority and breaks portability.
- Standardize a review connector now: rejected because it would likely encode
  application workflow rather than a stable authority.
- Make codec/DSP connector-first: rejected because portable algorithms should
  remain portable capabilities.

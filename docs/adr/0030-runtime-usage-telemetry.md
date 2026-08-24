# ADR-0030: Opt-In Runtime Usage Telemetry Behind a Provider-Neutral Port

- Status: Accepted
- Date: 2026-08-04
- Governing spec: `536-runtime-usage-telemetry` / `088-runtime-usage-telemetry`
- Decision evidence: `docs/decision-log.md` Decision 42, originating in
  `traverse-framework/registry`'s Decision 47 (registry#134)
- Extends: ADR-0049 (provider-neutral port precedent; renumbered from
  ADR-0029 on 2026-08-24)

## Context

There is no signal today for whether a published capability is actually
resolved or executed by real consumers, only whether it exists in the
registry index. `traverse-framework/registry` brainstormed the policy for a
maintainer-facing adoption signal (registry decision 47) and handed off the
architecture to this repo, since the only components that can observe
resolve/execute events — `traverse-cli` and the shared `traverse-contracts`
port surface — live here. `crates/traverse-registry`, where resolution
itself happens, was extracted to `traverse-framework/registry` (Spec 051)
and cannot take on a concrete network/telemetry dependency without breaking
its own portability and that repo's inherited-governance boundary
(`013-inherited-registry-governance` FR-002).

## Decision

Add a `UsageTelemetrySink` port trait to `traverse-contracts`, with a no-op
default — the same provider-neutral-port pattern ADR-0049 established for
hosted DataStore transport. `traverse-cli` owns the only real
implementation: a persistent, prompt-free opt-in config command, a locally
generated anonymous install ID, and an HTTPS client to a purpose-built
hosted product-analytics collector (e.g. PostHog). `crates/traverse-registry`
calls the port at its resolution call site (contract defined here as FR-008;
the actual call site is `traverse-framework/registry`'s own governed change,
tracked as that repo's Spec 015) but never links against the concrete
adapter, network code, or opt-in state — it only ever sees the trait, and by
default the no-op implementation.

Consent is opt-in and off by default, with no interactive prompts anywhere.
Every event carries exactly four fields (event type, `namespace/id@version`,
timestamp, anonymous install ID) — no CLI version, OS, hostname, or IP.
Sends are fire-and-forget with a short timeout; any failure is swallowed
completely so telemetry can never add latency or a new failure mode to a
real command.

## Consequences

- Capability adoption/orphan visibility becomes possible for the first time,
  without any component gaining a hard telemetry dependency.
- `crates/traverse-registry` stays portable and fully testable offline; the
  no-op default means today's behavior is unchanged until `traverse-cli`
  wires the real adapter and a user opts in.
- Two repos must coordinate a release sequence: this repo publishes the
  trait in `traverse-contracts` first, `traverse-framework/registry` bumps
  its pinned dependency and adds the resolve-side call, then this repo bumps
  its own `traverse-registry` dependency to pick it up.
- Adoption of the telemetry itself will likely be low precisely because it
  is opt-in with no prompts — accepted deliberately, matching this
  ecosystem's existing no-consent-UI posture (registry decision 45) rather
  than trading it away for higher data volume.

## Alternatives Considered

- Opt-out by default: rejected — a materially bigger trust decision than
  anything else in either repo's history, requiring real disclosure/consent
  design neither repo has built.
- Reuse registry's existing Plausible website-analytics account: rejected —
  Plausible's event/property model is website-pageview-shaped, a weaker fit
  for arbitrary CLI event properties than a tool built for anonymous
  distinct-ID-plus-custom-event tracking.
- Self-hosted collector (e.g. a Cloudflare Worker): rejected — first piece
  of maintained backend infrastructure either repo would own, a bigger
  operational shift than this feature's value justifies today.
- Instrumenting `crates/traverse-registry` directly with a concrete HTTP
  client: rejected — would break that crate's portability/offline
  testability and cross the inherited-governance boundary from
  `013-inherited-registry-governance` FR-002 without a dedicated spec in its
  own repo.

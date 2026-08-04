# Feature Specification: Opt-In Runtime Usage Telemetry for Published Capabilities

**Status**: Approved
**Canonical governing ID**: `088-runtime-usage-telemetry`
**Extends**: `087-hosted-datastore-transport` (provider-neutral port precedent), `029-integrated-observability` (distinguished from, not superseded by — see Scope)
**Decision evidence**: `docs/decision-log.md` Decision 42, itself handed off from `traverse-framework/registry`'s `docs/decision-log.md` Decision 47 (registry `/brainstorm`, closing registry#134)

## Purpose

Define an opt-in, anonymous signal for how often a published capability is
actually **resolved** (a registry version lookup) and **executed** (a real
WASM invocation via `capability execute`/`serve`), reported to the Traverse
maintainers so capability adoption and unused/orphaned capabilities become
visible.

This is deliberately not the same concern as Spec 029 (Integrated
Observability): Spec 029 is an operator-facing OTel signal scoped to a single
deployment, always under that deployment's own control and export
configuration. This spec defines a maintainer-facing, cross-deployment
adoption signal, sent only when a user explicitly opts in, to a fixed
maintainer-owned collector, not a configurable OTLP endpoint. The two systems
share no code path and neither depends on the other.

## Scope

In scope:

- a provider-neutral `UsageTelemetrySink` port in `traverse-contracts`, with a
  no-op default
- a `traverse-cli` adapter: persistent opt-in/opt-out config command, a
  locally generated anonymous install ID, and a hosted product-analytics
  collector client (e.g. PostHog)
- an `execute` event emitted from `traverse-cli`'s `capability execute`/
  `serve` command path
- the contract that `crates/traverse-registry`'s resolution path calls the
  port for a `resolve` event on every successful resolution

Out of scope (tracked elsewhere or deliberately deferred):

- the actual `resolve`-event call site inside `crates/traverse-registry` —
  that crate was extracted to, and is now governed by,
  `traverse-framework/registry` (Spec 051 extraction; `013-inherited-registry-governance`
  FR-002 requires any new behavior there to go through a new spec in that
  repo, not this one). See `traverse-framework/registry`'s Spec 015.
- any public-facing display of usage counts (registry decision 47: admin-only
  dashboard for now)
- richer diagnostic payloads (CLI version, OS, etc.) beyond the fields in
  FR-004
- a self-hosted collector or new backend infrastructure of any kind

## Requirements

### Functional Requirements

- **FR-001**: A `UsageTelemetrySink` trait MUST be added to `traverse-contracts`,
  exposing a method to record a usage event (event type `resolve` or
  `execute`, a capability reference `namespace/id@version`, and a timestamp).
  A no-op implementation MUST be the default so that no caller (including
  `crates/traverse-registry`) takes on a network or configuration dependency
  merely by calling the port.
- **FR-002**: `traverse-cli` MUST provide a persistent local opt-in mechanism
  (`traverse-cli telemetry enable` / `telemetry disable`), off by default.
  Enabling telemetry MUST NOT display any interactive prompt at any other
  time; there MUST be no env-var-only or first-run-prompt path to enabling
  it.
- **FR-003**: On first successful `telemetry enable`, `traverse-cli` MUST
  generate a random UUID once and persist it in local CLI config as the
  install ID. This ID MUST NOT be derived from or combined with any
  machine-identifying, network-identifying, or personally-identifying value.
- **FR-004**: Every event `traverse-cli`'s real `UsageTelemetrySink`
  implementation sends MUST contain exactly: event type (`resolve` or
  `execute`), `namespace/id@version`, an event timestamp, and the install ID
  from FR-003. It MUST NOT contain CLI version, OS, hostname, IP address, or
  any other field.
- **FR-005**: `traverse-cli`'s real sink implementation MUST send each event
  to a purpose-built hosted product-analytics collector (e.g. PostHog) over
  HTTPS, fire-and-forget, with a short send timeout (target: 1-2 seconds).
  Any failure (timeout, DNS failure, non-2xx response, or collector
  unavailability) MUST be swallowed completely: it MUST NOT delay, retry
  synchronously, fail, or log — even in verbose/debug output — the CLI
  command the event is attached to.
- **FR-006**: `traverse-cli` MUST emit an `execute` event through the sink
  each time `capability execute`/`serve` completes a real WASM invocation,
  only when telemetry is enabled (FR-002).
- **FR-007**: When telemetry is disabled (the default), `traverse-cli` MUST
  wire the no-op `UsageTelemetrySink` (FR-001) so that no network call of any
  kind related to this spec ever occurs.
- **FR-008**: `crates/traverse-registry`'s resolution path MUST accept an
  optional, caller-supplied `UsageTelemetrySink` and, on a successful
  version resolution, MUST invoke it with a `resolve` event carrying the
  resolved `namespace/id@version`. A failed resolution MUST NOT emit an
  event. This requirement defines the contract `traverse-cli` and
  `crates/traverse-registry` (as of `traverse-framework/registry` Spec 015)
  must both satisfy; it does not itself change `crates/traverse-registry`'s
  code (out of this repo's governance, see Scope).

## Acceptance Scenarios

1. Given a fresh `traverse-cli` install with telemetry never enabled, when
   `registry sync` or `capability execute` runs, then no network call to any
   telemetry collector occurs.
2. Given a user runs `traverse-cli telemetry enable`, when they run it again
   on the same machine, then the same install ID persists (it is not
   regenerated) and no interactive prompt is ever shown.
3. Given telemetry is enabled and a capability executes successfully via
   `capability execute`, when the event is sent, then it contains exactly
   the four fields in FR-004 and no others.
4. Given telemetry is enabled and the collector is unreachable, when
   `capability execute` runs, then the command completes with its normal
   exit code and output, unaffected by the telemetry failure.
5. Given telemetry is disabled, when any command that could emit a usage
   event runs, then the no-op sink is used and no collector code path
   executes at all.

## Quality Gates

- **QG-001**: A unit test MUST prove the no-op `UsageTelemetrySink` is wired
  whenever telemetry is disabled, with no HTTP client constructed.
- **QG-002**: A unit test MUST prove the real sink's payload contains exactly
  the FR-004 field set (no accidental extra fields) for both `resolve` and
  `execute` event types.
- **QG-003**: A test MUST simulate collector timeout/failure and assert the
  invoking command's exit code and output are unaffected.
- **QG-004**: A test MUST prove the install ID persists across repeated CLI
  invocations after `telemetry enable` and is a v4 UUID with no embedded
  machine-identifying data.

## Implementation Tickets

- Traverse #925 — add `UsageTelemetrySink` port trait to `traverse-contracts`
- Traverse #926 — `traverse-cli`: `telemetry enable`/`disable` command,
  install ID, PostHog adapter
- Traverse #927 — `traverse-cli`: emit `execute` event from `capability
  execute`/`serve`
- Traverse #928 — `traverse-cli`: bump `traverse-registry` dependency once
  registry's resolve-hook (registry Spec 015) ships
- `traverse-framework/registry`#144 — `crates/traverse-registry`: emit
  `resolve` event via the port (registry Spec 015, that repo's Project 3)

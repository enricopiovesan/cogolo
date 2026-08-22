# ADR-0043: Plan Workflows as a Deterministic, Data-Dependency-Only Reader, Never an Auto-Chooser

- Status: Accepted
- Governing specs: `108-governed-runtime-workflow-composition`, `113-declarative-workflow-planning`
- Related issues: #865, #1089, #1098

## Context

`109-runtime-workflow-proposals` (P1) deliberately excluded "planner
implementation" from its scope, leaving open how a concrete, submittable
proposal actually gets generated in the first place. A live website demo
(traverse-framework.com/discover.html) exposed this gap concretely: without
real declared data-dependency metadata or a real planner, composing a
plausible workflow from live registry data required a client-side
namespace/verb-name heuristic — a guess, not something grounded in the
contracts themselves.

## Decision

The planner reads only declared `consumes`/`emits` contract metadata; it
never infers a chain from capability names, namespaces, or any other
signal, and never operates on a capability with no declared linkage. When
more than one capability could satisfy a need, the planner enumerates every
resulting complete candidate plan rather than choosing one — ambiguity
resolution is a caller/reviewer decision, never a runtime one, consistent
with ADR-0041 treating the planner-adjacent surface as untrusted and this
runtime's everywhere-else stance against silent inference. Search is
bounded by fixed, small, non-configurable limits in v1. Field-level
JSON-path mappings are proposed but always marked unconfirmed until
reviewed — the planner produces a draft, not an authority. The planner is a
new phase (P0) of the `108` north star rather than a standalone spec, since
`109` already named this exact gap as its own boundary.

## Consequences

A planner call against today's registry (per registry#305: `consumes`/
`emits` populated on roughly 1 of 116 published capability versions) will
mostly return zero candidates. That is treated as correct, honest behavior
— not a bug to route around with a heuristic — and makes this spec's real
unblock condition explicit: registry#305's backfill, not more runtime code.
`traverse#1098`'s original framing (which implied the review/execution/
trust architecture was still undecided) is superseded by this narrower
reading; `108`–`112` already own that.

## Alternatives considered

- Fall back to a namespace/verb heuristic when `consumes`/`emits` is empty
  (matching the discover.html demo): rejected — mixing a real, data-grounded
  candidate with a guessed one on the same governed proposal surface blurs
  exactly the honesty line this org has held elsewhere (registry
  decision-log entry 61).
- Auto-resolve ambiguity via a scoring heuristic (coverage %, version
  recency): rejected — a silent automatic pick is the kind of unreviewed
  runtime decision `109`'s explicit-mapping requirement exists to prevent.
- Accept natural-language goals and interpret them in-runtime: rejected —
  reintroduces exactly the hosted-model dependency `109` FR-001 and
  ADR-0041 explicitly excluded.

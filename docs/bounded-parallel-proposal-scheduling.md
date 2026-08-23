# Bounded Parallel Proposal Scheduling (P2)

Governed by spec [`110-bounded-parallel-workflow-scheduling`](../specs/110-bounded-parallel-workflow-scheduling/spec.md)
and [ADR-0042](adr/0042-phased-dynamic-orchestration-evolution.md). Tracks
issue `#1092`.

P2 extends the [P1 sequential proposal lifecycle](workflow-proposal-lifecycle.md)
with deterministic, resource-bounded **concurrent** execution of independent
branches in the same proposal DAG. It reuses the exact same
`WorkflowProposal` wire format, canonicalization, and cross-validation as
P1 — P2 is a smarter, bounded *execution strategy* over an unchanged
document, not a new schema.

## Where the code lives

| Layer | Crate | What it owns |
|---|---|---|
| Wave levelization, fan-out/join-width/queue-depth bounds | `traverse-contracts::proposal` (`compute_parallel_schedule`) | Pure graph analysis over an already-canonicalized `CanonicalProposal` — no manifest/registry access, same portability guarantee as P1's structural validation. |
| `pure_read`-only authorization (FR-004a), concurrent execution, wall-time/payload bounds | `traverse-runtime::parallel_proposal` | Needs resolved capability contracts (for `effect_class`) and the `Runtime` execution engine, same split rationale as P1's `traverse-runtime::proposal`. |
| Public MCP tool surface | `traverse-mcp::tools::parallel_proposals` | Mirrors `tools::proposals`' plain-function pattern; reuses its authorization/quota helper (`authorize_and_reserve_quota`) so P1 and P2 share one FR-006/FR-006a/FR-007b implementation. |

## Why the proposal document doesn't change

P1's DAG model (`nodes`/`edges`/`mappings`) already supports arbitrary
fan-out and fan-in — `canonicalize_proposal`'s topological sort already
handles diamonds and wide fan-out deterministically. What P1's executor
does *not* do is run independent branches concurrently; it walks the total
order one node at a time. P2 adds a second analysis pass —
`compute_parallel_schedule` — that levelizes the same validated graph into
**waves**: each wave is the set of node ids whose dependencies are fully
satisfied by earlier waves, sorted lexicographically for determinism. A
linear proposal produces one node per wave (no observable behavior change);
a diamond (`a → {b, c} → d`) produces three waves, with `b` and `c`
eligible to run concurrently in the middle one.

## Bounds (FR-001, FR-005: reject before doing any work)

`ParallelScheduleLimits` (`traverse-contracts`), checked structurally
before any execution:

| Bound | Meaning |
|---|---|
| `max_fan_out` | Max node ids ready to run concurrently in any single wave. |
| `max_join_width` | Max direct predecessors (in-degree) converging into any one node. |
| `max_queue_depth` | Max total node ids across the whole schedule. |
| `max_concurrent_nodes` | Max node executions the runtime dispatches at once within a wave (also the execution-time throttle — see below). |

`ParallelExecutionLimits` (`traverse-runtime::parallel_proposal`), checked
during execution because they aren't expressible from graph shape alone:

| Bound | Meaning |
|---|---|
| `max_wall_time` | Total wall-clock budget for the whole execution, checked **before starting each wave** — not preemptively mid-wave (see below). |
| `max_wave_payload_bytes` | Max total serialized JSON byte size of one wave's assembled node inputs — a bounded, honest proxy for a per-wave memory budget. |
| `max_concurrent_nodes` | Mirrors the schedule-level bound; batches a wave into chunks of this size before dispatch. |

A structural bound violation is reported as `ParallelScheduleFailure` with a
stable `fan_out_exceeded` / `join_width_exceeded` / `queue_depth_exceeded`
code (FR-010 style, matching P1). An execution-time bound violation reports
`ProposalTerminalState::Cancelled`, with every un-dispatched node marked
`skipped_after_earlier_failure`.

## `pure_read`-only concurrency (FR-004a)

The first P2 implementation permits a wave with more than one member only
when **every** member's declared `effect_class` is `pure_read`.
`enforce_pure_read_only_parallelism` checks this after cross-validation
resolves each node's capability contract, denying with
`ConcurrentSideEffectDenied` otherwise. A non-`pure_read` node is still
allowed to execute — just never concurrently with a sibling; a singleton
wave (no concurrent sibling) is unaffected regardless of effect class. State
writes and external effects stay entirely sequential until a successor spec
proves declared independence, idempotency, cancellation, and budget
semantics (ADR-0042).

## Execution and determinism (FR-002)

`execute_parallel_proposal` dispatches each wave's nodes on real OS threads
via `std::thread::scope`, bounded to `max_concurrent_nodes` per batch. A
join may only consume declared, completed predecessor outputs — guaranteed
structurally, since a mapping's source can only be `initial_input` or a
node in the *same or earlier* wave by construction of the levelization
(nodes in the same wave have no edge between them, so no mapping between
them is structurally valid either).

Real thread completion order is not deterministic, but the **observable
trace is**: outcomes are folded back in the wave's lexicographic order
(the same tie-break rule P1 uses for its sequential execution order), never
completion order. A test proposal where one branch sleeps and the other
doesn't demonstrates this: the trace always lists the fast branch before
the slow one, matching lexicographic order.

## Cancellation semantics

Rust has no safe way to preemptively interrupt an in-flight OS thread
without `unsafe`, and FR-004a already restricts concurrent work to
side-effect-free local reads. So a wave that is already dispatched is
always allowed to finish — including every node within it, even after a
sibling in the same batch fails. A wall-time or payload budget that is
already exhausted simply refuses to **start** the next wave; it never
attempts to interrupt a running one. This is checked and tested explicitly
(a slow first wave plus a tight time budget results in the first wave's
outcomes present and `Cancelled`, with the second wave's nodes marked
skipped).

A panicking host executor is surfaced as a `Failed` outcome — never
silently dropped — matching this crate's fail-closed convention for
host-side faults (e.g. mutex-poisoning handling in `events/broker.rs`).

## MCP tool surface

`tools::parallel_proposals` exposes two functions, mirroring
`tools::proposals`' plain-function pattern (not wired into
`stdio_server.rs`, matching spec 015's precedent that pattern already set):

- `compute_schedule_for_proposal` — validates a proposal (P1, unchanged)
  and, if valid, computes and returns its concurrency waves plus automatic
  eligibility. A structural, cross-validation, schedule-bound, or
  concurrent-side-effect denial is a normal structured response.
- `execute_parallel_proposal_via_mcp` — the full P2 pipeline: validate,
  compute schedule, authorize concurrency (FR-004a), decide or verify
  authorization (FR-006/FR-006a, shared with P1 via
  `authorize_and_reserve_quota`), reserve a quota slot (FR-007b), execute,
  and return the trace. Every denial reason has its own stable code:
  `invalid_proposal`, `invalid_parallel_schedule`,
  `concurrent_side_effect_denied`, `approval_token_required`, a token error
  code, or `quota_exhausted_<scope>`.

`observe_proposal` from `tools::proposals` is reused as-is for P2 traces —
the trace shape is identical, so no new observation function exists.

## Non-goals (spec 110's own "Out of scope")

Dynamic expansion, loops, durable waits, retry/saga semantics, and implicit
parallelism inferred by a planner. These remain out of scope for every P2
implementation, not just this first one.

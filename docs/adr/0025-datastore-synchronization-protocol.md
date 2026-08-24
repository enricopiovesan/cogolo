# ADR-0025: Keep DataStore Synchronization Explicit and Deterministic

- Status: Superseded by [ADR-0027](0027-datastore-synchronization.md)
- Approved: 2026-08-24, then immediately identified as superseded the same day
- Governing spec: `531-datastore-synchronization-protocol` (superseded; removed
  from `specs/governance/approved-specs.json` — see decision log entry 64's
  amendment)
- Extends: ADR-0018 and ADR-0019

**Superseded note (2026-08-24)**: this ADR was briefly approved (Decision 64)
without checking whether the DataStore synchronization question had already
been answered elsewhere. It had: issue `#883` (closed, completed) shows this
exact protocol — mutation envelopes, Lamport-clock-then-writer-ID conflict
resolution, opaque per-peer cursors, in-process test-double transport — already
shipped under `089-datastore-synchronization` / the pre-existing
`docs/adr/0027-datastore-synchronization.md` (unrelated to the renumbered
ADR-0045). Kept here for historical record of the design that was independently
re-derived, not as an active decision. See ADR-0027 (datastore-synchronization)
for the current model.

## Decision (historical — not in effect)

Synchronization is an explicit host-requested protocol, not a background
runtime service. The v1 conformance target is a local-peer or test-double
transport. Conflicts resolve by higher Lamport clock and then lexicographically
greater writer identifier. Attempts are idempotent, interruptible, scoped by
the host, and emit secret-free decision evidence.

## Consequences

No provider, hosted transport, offline queue, CRDT, or multi-process policy is
introduced. Those capabilities require successor decisions.

# ADR-0026: Use OS-Backed Local Exclusive Locks for DataStore Roots

- Status: Superseded by [ADR-0033](0033-datastore-multiprocess-coordination.md)
- Approved: 2026-08-24, then immediately identified as superseded the same day
- Governing spec: `532-multiprocess-datastore-coordination` (superseded; removed
  from `specs/governance/approved-specs.json` — see decision log entry 65's
  amendment)
- Extends: ADR-0018 and ADR-0019

**Superseded note (2026-08-24)**: this ADR was briefly approved (Decision 65)
without checking whether the multi-process coordination question had already
been answered differently elsewhere. It had: issue `#879`'s own history shows
the team explicitly pivoted away from this plain-advisory-lock model on
2026-08-05, to the host-owned-coordinator-with-lease-fencing model in
`093-datastore-multiprocess-coordination` / ADR-0033 — which is what actually
shipped. Kept here for historical record of the design that was considered
and abandoned, not as an active decision. See ADR-0033 for the current model.

## Decision (historical — not in effect)

Single-process locking remains the default. The local multi-process model uses
one OS-backed exclusive advisory lock per root. Contenders return `store_busy`
and retry only on host instruction. Crash recovery relies on OS lock release;
leases, heartbeats, fencing, takeover timers, and coordinator daemons are not
used. Coordination is not synchronization and does not merge concurrent writes.

## Consequences

The implementation must prove contention, crash recovery, explicit retry,
reader snapshot visibility, and unsupported-filesystem behavior without
exposing roots or process identity. Distributed providers remain separate
decisions.

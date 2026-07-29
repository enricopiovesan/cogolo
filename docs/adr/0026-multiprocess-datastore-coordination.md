# ADR-0026: Use OS-Backed Local Exclusive Locks for DataStore Roots

- Status: Proposed
- Governing draft: `532-multiprocess-datastore-coordination`
- Extends: ADR-0018 and ADR-0019

## Decision

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

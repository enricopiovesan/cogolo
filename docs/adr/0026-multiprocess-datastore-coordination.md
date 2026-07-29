# ADR-0026: Preserve Fenced Single-Writer Coordination for DataStore Roots

- Status: Proposed
- Governing draft: `532-multiprocess-datastore-coordination`
- Extends: ADR-0018 and ADR-0019

## Decision

Single-process locking remains the default. A future host-selected coordination
authority may grant a bounded exclusive lease with a monotonic fencing epoch.
Expired or fenced owners fail closed before durable mutation. Coordination is
not synchronization and does not merge concurrent writes.

## Consequences

The implementation must prove contention, crash recovery, expiry, fencing, and
fairness without exposing roots or process identity. Provider selection and
lock-service SDKs remain separate decisions.

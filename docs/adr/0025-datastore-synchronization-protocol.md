# ADR-0025: Keep DataStore Synchronization Explicit and Deterministic

- Status: Proposed
- Governing draft: `531-datastore-synchronization-protocol`
- Extends: ADR-0018 and ADR-0019

## Decision

Synchronization is an explicit host-requested protocol, not a background
runtime service. The v1 conformance target is a local-peer or test-double
transport. Conflicts resolve by higher Lamport clock and then lexicographically
greater writer identifier. Attempts are idempotent, interruptible, scoped by
the host, and emit secret-free decision evidence.

## Consequences

No provider, hosted transport, offline queue, CRDT, or multi-process policy is
introduced. Those capabilities require successor decisions.

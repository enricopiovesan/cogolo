# ADR-0033: Keep Multi-Process DataStore Coordination Host-Owned

- Status: Accepted
- Governing spec: `093-datastore-multiprocess-coordination`

## Decision

A host-owned coordinator exclusively holds the write lease for a DataStore root.
Other processes access it through an explicit host-provided IPC port. Fenced
generations prevent stale coordinators from committing after crash takeover.

## Consequences

Traverse preserves file integrity without adopting an IPC daemon or transport.
Hosts retain lifecycle, authorization, and transport ownership.

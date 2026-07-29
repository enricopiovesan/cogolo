# ADR-0024: Keep Remote DataStore Provider-Neutral and Host-Owned

- Status: Proposed
- Governing draft: `530-remote-key-value-datastore`
- Extends: ADR-0018 and ADR-0019

## Decision

Traverse will expose a provider-neutral remote key-value DataStore contract
only through explicit host injection. The host owns provider choice,
credentials, tenancy, endpoint, retry policy, cost, and availability. An
acknowledged write has read-your-write consistency for its owning host;
ambiguous transport outcomes fail closed and are never silently retried as
successful writes.

## Consequences

No provider SDK or remote synchronization behavior is introduced. A separate
approved protocol is required for replication, cursors, offline queues, and
conflict resolution.

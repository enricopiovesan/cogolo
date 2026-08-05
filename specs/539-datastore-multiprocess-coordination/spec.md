# Feature Specification: Host-Owned Multi-Process DataStore Coordination

**Status**: Approved
**Canonical governing ID**: `093-datastore-multiprocess-coordination`
**Extends**: `518-durable-local-datastore`, `519-embedder-owned-datastore-integration`

## Purpose

Define the host-owned coordinator required when multiple local processes use one
DataStore root. Traverse does not own an IPC server, root, tenant, or credential.

## Requirements

- **FR-001**: One host-owned coordinator holds the exclusive writer lease.
- **FR-002**: Non-coordinator processes use a host-provided IPC port; they never write store files directly.
- **FR-003**: Each lease has a monotonically increasing fenced generation. A stale generation MUST fail before commit.
- **FR-004**: After coordinator crash, a new coordinator acquires the released OS lock and a newer generation before serving writes.
- **FR-005**: Coordinator serialization defines deterministic request order.
- **FR-006**: Stable errors include `datastore_owner_locked` and `datastore_coordinator_unavailable`; evidence contains no root, payload, tenant, or credential.
- **FR-007**: Cross-process conformance covers contention, crash takeover, stale-writer fencing, fairness, and deterministic failures.

## Out of Scope

Traverse-owned daemons, direct concurrent file writes, network/distributed coordination, peer discovery, and host IPC transport selection.

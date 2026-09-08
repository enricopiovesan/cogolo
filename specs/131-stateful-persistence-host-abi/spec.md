# Feature Specification: Capability-Side WASM Host ABI for Stateful Persistence

**Feature Branch**: `governance/stateful-abi-registry-spec-approvals`
**Created**: 2026-09-08
**Status**: Approved (2026-09-08)
**Canonical governing ID**: `131-stateful-persistence-host-abi`
**Version**: 0.1.0
**Extends**: `002-capability-contracts`, `208-service-type-taxonomy` (canonical `014`), `518-durable-local-datastore`
**Input**: Issue #1285; ADR-0063; `/brainstorm` session recorded as Decision 68 in `docs/decision-log.md`. Precedent: `098-capability-event-host-abi` / ADR-0035 / Decisions 48-49.

## Purpose

Define a WASM host-function ABI that lets a `Stateful` capability persist and
retrieve named state during execution, and the host-side contract that services
it. Today `service_type: Stateful` exists on `CapabilityContract` (spec `014`)
and the placement evaluator already excludes `Browser` for it, but the runtime
exposes no managed-persistence import in `host_abi_v1.json` — only WASI stdio,
`traverse_host` environment/metadata queries, `emit_event`, and
`connector_invoke`. Without this surface, no `Stateful` capability can actually
run, and the mapped Registry roster (cart, ticket workspace, approval packet
store, challenge session, pricing config, and more) cannot be published.

This spec governs the host-function ABI surface (import signatures, calling
convention, synchronous validation, scoping, and failure semantics) and the
abstract host store contract it is serviced by. It does not include the
implementation, fixture authoring, or any change to `service_type` taxonomy or
the `Stateful` + `Browser` placement rule.

## Capability Boundary

| Concern | Owner |
| --- | --- |
| The three `state_*` import signatures, calling convention, and validation | This spec / Traverse runtime |
| Composing the real storage key from `capability_id` + partition + key | Traverse runtime (guest never sees raw paths) |
| Per-`(capability_id, partition)` serialization of state operations | Traverse runtime |
| Recording mutation metadata to the trace journal | Traverse runtime |
| Durable byte storage, integrity, atomicity, encryption, retention, backup | Host (DataStore v2 by default; substitutable) |
| Partition-wide teardown, quota, and lifecycle scheduling | Host / embedder (retention spec `526`) |

## User Scenarios & Testing

### User Story 1 — A Stateful capability persists and reads its own state (Priority: P1)

A `Stateful` capability writes a value under a partition and key during one
execution and reads the same value back in a later execution, without the
guest ever handling a storage path.

**Why this priority**: This is the entire reason the ABI exists; every roster
capability depends on it.

**Independent Test**: Execute a `Stateful` fixture that calls `state_put(p, k,
v)`; execute it again with the same partition and key calling `state_get(p, k)`;
assert the returned bytes equal `v`.

**Acceptance Scenarios**:

1. **Given** a `Stateful` capability, **when** it calls `state_put` then
   `state_get` for the same partition and key within one execution, **then**
   `state_get` returns the just-written value (read-your-writes).
2. **Given** a value written in a prior execution, **when** a later execution of
   the same capability calls `state_get` with the same partition and key,
   **then** it returns that value after a host restart.
3. **Given** a key that was never written, **when** `state_get` is called,
   **then** it returns an explicit absent result, not an error.
4. **Given** a key holding a value, **when** `state_delete` is called for it and
   then `state_get`, **then** `state_get` returns an absent result.

---

### User Story 2 — Cross-capability and cross-partition isolation (Priority: P1)

State written by one capability under one partition is never readable by another
capability, or under another partition, through this ABI.

**Why this priority**: One deployed capability instance serves many end users;
isolation must be host-guaranteed, not guest-cooperative.

**Independent Test**: Write a value as capability A under partition `x`; from
capability B (any partition) and from capability A under partition `y`, call
`state_get` for the same key; assert both see an absent result.

**Acceptance Scenarios**:

1. **Given** capability A wrote `(x, k) -> v`, **when** capability B calls
   `state_get(x, k)`, **then** the result is absent — B cannot address A's
   namespace.
2. **Given** capability A wrote `(x, k) -> v`, **when** capability A calls
   `state_get(y, k)` with a different partition `y`, **then** the result is
   absent.
3. **Given** any `state_*` call, **when** the runtime composes the storage key,
   **then** the calling `capability_id` is the enforced prefix and the guest
   cannot supply, override, or observe it.

---

### User Story 3 — Only `Stateful` capabilities may call the ABI (Priority: P1)

A capability whose `service_type` is not `Stateful` is rejected at call time if
it invokes any `state_*` import.

**Why this priority**: Mirrors `098` FR-003 (only `Subscribable` may emit) and
keeps the persistence surface bound to the placement constraints written against
`Stateful`.

**Acceptance Scenarios**:

1. **Given** a `Stateless` or `Subscribable` capability, **when** it calls
   `state_get`, `state_put`, or `state_delete`, **then** the host rejects the
   call synchronously with a stable error code and performs no storage
   operation.

---

### User Story 4 — Bounds and malformed input fail closed (Priority: P1)

Oversized or out-of-bounds guest input is rejected with a stable error; the host
never traps, panics, or reads past guest memory.

**Independent Test**: Call `state_put` with a value length exceeding the maximum,
and separately with a pointer/length pair outside guest linear memory; assert a
stable error code each time and that the store is unchanged.

**Acceptance Scenarios**:

1. **Given** a `state_put` whose value exceeds the maximum value size, **when**
   the call is made, **then** it is rejected with `state_value_too_large` and no
   write occurs.
2. **Given** a `state_*` call whose key or partition exceeds its maximum length,
   **when** the call is made, **then** it is rejected with
   `state_key_invalid` and no operation occurs.
3. **Given** a `state_*` call whose payload pointer/length falls outside guest
   linear memory, **when** the call is made, **then** the host returns
   `state_payload_invalid` without trapping or reading out of bounds.

---

### User Story 5 — Concurrent executions on one partition are serialized (Priority: P2)

Two concurrent executions of the same capability targeting the same partition
see a sequential, non-interleaved view of state.

**Acceptance Scenarios**:

1. **Given** two concurrent executions of capability A both writing
   `(x, k)`, **when** both complete, **then** one write is fully applied, the
   other is fully applied after it, and no partial or interleaved value is
   observable.
2. **Given** executions targeting different partitions or different
   capabilities, **when** they run concurrently, **then** they are not
   serialized against each other.

---

### User Story 6 — Mutations are auditable without leaking payloads (Priority: P2)

Every `state_put` and `state_delete` leaves a trace-journal record identifying
what changed, without the value itself entering the journal.

**Acceptance Scenarios**:

1. **Given** a `state_put`, **when** it succeeds, **then** the trace journal
   gains an entry with `capability_id`, a digest of the partition, the key, a
   digest of the value, and the `execution_id` — and not the value bytes.
2. **Given** a `state_get`, **when** it runs, **then** no trace-journal entry is
   produced (reads are not mutations).

### Edge Cases

- A `state_get` for an absent key is an absent result, never
  `integrity_check_failed`.
- A value that fails the host store's integrity check on read surfaces the
  store's stable integrity failure to the guest, not a partial value.
- A `state_put` that would exceed a host-configured quota (when a host enforces
  one) fails closed with a stable code; v0.1.0 defines no quota of its own.
- `Browser` remains excluded for `Stateful` by spec `014` FR-005; this ABI adds
  no path that circumvents that rule.

## Requirements

### Functional Requirements

- **FR-001**: The runtime MUST expose exactly three new `traverse_host` imports —
  `state_get`, `state_put`, and `state_delete` — versioned and whitelisted the
  same way as the existing `traverse_*` bridge functions
  (`host_abi_whitelist` / `HOST_ABI_V1_WHITELIST`), and documented in the same
  location as the rest of the ABI-version import set.
- **FR-002**: Each import MUST take a caller-supplied opaque `partition` byte
  string and a `key` byte string. `state_put` additionally takes a `value` byte
  string. `state_get` MUST return either the stored value or an explicit absent
  indicator. The guest MUST NOT supply, override, or observe the storage
  namespace prefix.
- **FR-003**: The host MUST compose the real storage key as
  `capability_id / partition / key` using the calling capability's authenticated
  `capability_id`. State written by one `capability_id` MUST NOT be readable or
  deletable through this ABI by any other `capability_id`, and state under one
  partition MUST NOT be addressable under another.
- **FR-004**: The host MUST reject any `state_*` call from a capability whose
  `service_type` is not `Stateful`, synchronously at call time, returning a
  stable error to the guest and performing no storage operation.
- **FR-005**: The host MUST validate that the guest-supplied pointer/length
  pairs lie within the guest's linear memory and MUST enforce fixed maximums for
  partition length, key length, and value size before any storage operation. A
  malformed or oversized input MUST be rejected with a stable, secret-free error
  code; the host MUST NOT trap, panic, or read outside guest memory.
- **FR-006**: `state_get` for a key that holds no value MUST return the absent
  indicator, not an error. A value that fails the backing store's integrity
  check MUST surface that store's stable integrity failure and no partial value.
- **FR-007**: The host MUST serialize `state_get` / `state_put` / `state_delete`
  per `(capability_id, partition)` so that a guest observes a sequential,
  non-interleaved view including read-your-writes within a partition.
  Operations on distinct partitions or distinct capabilities MUST NOT be
  serialized against each other. This ABI version defines no compare-and-swap or
  version-token operation.
- **FR-008**: Every successful `state_put` and `state_delete` MUST append a
  trace-journal entry containing `capability_id`, a digest of the partition, the
  key, a digest of the resulting value (or a tombstone marker for delete), and
  the `execution_id`. The value bytes MUST NOT be written to the trace journal.
  `state_get` MUST NOT produce a trace-journal entry.
- **FR-009**: The three imports MUST be serviced through an abstract host store
  interface with a documented guarantee floor: integrity-checked reads, atomic
  writes, read-your-writes within a partition, and durability across host
  restart. The embedded host MUST bind this interface to the spec `518`
  DataStore v2 by default; a host MAY substitute another implementation that
  meets the guarantee floor.
- **FR-010**: Guest-visible outcomes and the trace-journal entries MUST NOT
  contain storage paths, credentials, endpoints, encryption material, or
  host-private configuration values.
- **FR-011**: The ABI MUST fold into host ABI version 1 with no parallel or
  deprecated compatibility path, consistent with Decision 48 (no back-compat tax
  before production use).
- **FR-012**: Existing `Stateless` and `Subscribable` capability behavior, the
  `emit_event` / `connector_invoke` imports, and the `service_type` taxonomy
  MUST remain unchanged.

### Key Entities

- **Partition**: an opaque caller-supplied byte string (for example an end-user
  or session identifier) that subdivides a capability's namespace. The host
  treats it as opaque and digests it for trace records.
- **StatefulStore interface**: the abstract host contract servicing the three
  imports, with the FR-009 guarantee floor; DataStore v2 is the default binding.
- **State mutation record**: the trace-journal entry produced by `state_put` /
  `state_delete` — identity, digests, and `execution_id`, never the value.

## Success Criteria

### Measurable Outcomes

- **SC-001**: A `Stateful` fixture writes and reads back a value across two
  executions and a simulated restart, with zero raw storage paths visible to the
  guest.
- **SC-002**: Isolation tests prove no cross-`capability_id` and no
  cross-partition read or delete is possible through the ABI.
- **SC-003**: A non-`Stateful` capability calling any `state_*` import is
  rejected synchronously in an automated test, with no storage side effect.
- **SC-004**: Oversized value, oversized key/partition, and out-of-bounds
  pointer inputs each produce their distinct stable error code and leave the
  store unchanged; no test induces a host trap or panic.
- **SC-005**: A concurrency test shows two same-partition executions apply
  fully and sequentially with no interleaved or partial value, while
  different-partition executions are observed to run without mutual
  serialization.
- **SC-006**: Trace-journal assertions confirm a metadata-plus-digest entry for
  every `state_put` / `state_delete`, no entry for `state_get`, and no value
  bytes or host-private values in any entry.

## Compatibility and Scope

This specification is additive to host ABI version 1. It does not modify the
`service_type` enum, the `Stateful` + `Browser` placement rule in spec `014`
FR-005, the `emit_event` ABI (`098`), `connector_invoke` (`104`), or the
embedder-owned DataStore surface (`518` / `519` / `528`). It does not add a
partition-wide clear import, a per-capability quota, a compare-and-swap
operation, a key-enumeration import, or any Stateful + event multi-role
mechanism — each is recorded as deferred follow-up work in Decision 68.

## Assumptions

- Hosts own durable storage location, integrity mechanism, encryption,
  retention, backup, and any quota; none are surfaced to the guest or in trace
  records.
- The default binding is DataStore v2; substitutes are the host's
  responsibility to keep within the FR-009 guarantee floor.
- Partition-wide teardown and lifecycle scheduling remain an embedder/retention
  concern under spec `526`.
- Traverse has no production users or production capabilities today
  (`docs/decision-log.md` Decision 48), so the ABI is introduced without a
  compatibility shim.

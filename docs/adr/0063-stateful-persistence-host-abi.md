# ADR-0063: Key-Value WASM Host ABI for Stateful Capability Persistence

- Status: Accepted
- Date: 2026-09-08
- Governing spec: `131-stateful-persistence-host-abi` (Approved)
- Extends: `002-capability-contracts`, `208-service-type-taxonomy` (`014`), `518-durable-local-datastore`
- Related issue: #1285
- Related: ADR-0035 (`098` event host ABI, the structural precedent); `docs/decision-log.md` Decision 68

## Context

`service_type: Stateful` has existed on `CapabilityContract` since spec `014`,
and the placement evaluator already refuses to place a `Stateful` capability on
`Browser`. But `crates/traverse-runtime/src/executor/host_abi_v1.json` exposes
no persistence import — only WASI stdio, `traverse_host` environment/metadata
queries, `emit_event`, and `connector_invoke`. A `Stateful` capability
therefore cannot actually retain anything, and the Registry's mapped Wave 2
roster (cart, ticket workspace, approval packet store, challenge session,
pricing config, and more — registry decision-log entry 92) is blocked from
publication. Registry decision 92 explicitly rejected "affinity-only /
`prior_state` + `next_state` theater," so the fix has to be real managed
persistence, not state threaded through capability I/O.

The closest precedent is the event-publish ABI (spec `098` / ADR-0035): one
whitelisted `traverse_host` import, a call-time `service_type` gate,
synchronous validation, guest-memory bounds safety, and no back-compat tax
(Decision 48). Traverse has no production users or capabilities today.

## Decision

Introduce a key-value host ABI of three flat `traverse_host` imports —
`state_get`, `state_put`, `state_delete` — callable only by `Stateful`
capabilities, and fold it into host ABI version 1. The shape follows Decision
68:

- **Three imports, not a handle table or a multiplexed `state_op`.** A flat KV
  trio maps 1:1 onto DataStore v2 and spec `094` remote-KV, keeps each import
  independently whitelisted and bounds-checkable exactly like `emit_event`, and
  stays idiomatic with the existing `traverse_*` imports. No `list`, no batch,
  no compare-and-swap in this version.
- **Host-enforced `capability_id` namespace plus a caller-supplied opaque
  `partition`.** The runtime composes the real key as
  `capability_id / partition / key`; the guest never supplies or sees the
  prefix. One deployed `commerce.cart` serves every shopper because the
  per-user split is an opaque `partition` argument, while cross-capability and
  cross-partition isolation is guaranteed by the host, not by guest
  cooperation.
- **Per-key `state_delete` only.** Partition-wide teardown stays with the
  embedder and retention (spec `526`), which is accountable for retention and
  backup. No `state_clear` import in this version.
- **Serviced through an abstract host store interface with a written guarantee
  floor** — integrity-checked reads, atomic writes, read-your-writes within a
  partition, durability across restart — bound to DataStore v2 by default and
  substitutable by any implementation meeting the floor. This matches
  `connector_invoke` mediation and spec `519`'s embedder-owned framing.
- **Fixed conservative bounds, no quota.** The spec fixes maximum partition
  length, key length, and value size; the host validates guest pointer/length
  against linear memory and rejects oversized or malformed input with stable
  codes without trapping. No per-capability key-count or byte quota in this
  version.
- **Per-`(capability_id, partition)` serialization.** The host takes a
  short-lived lock around each operation so the guest sees a plain sequential
  model; different partitions and different capabilities are not serialized
  against each other. This is DataStore's single-writer behavior (spec `518`
  User Story 4) surfaced as the ABI's concurrency contract.
- **Trace-journal metadata on mutation.** Each `state_put` / `state_delete`
  appends an entry with `capability_id`, digested partition, key, value digest,
  and `execution_id` — never the value. `state_get` is not traced. This gives
  the audit-sensitive roster (`doc-approval.packet-store`,
  `doc-approval.policy-store`) a real record without pulling business data into
  the append-only journal or under spec `527` retention/encryption scope.
- **`Stateful` + events multi-role is out of scope.** `service_type` is a
  single enum and `098` FR-003 gates `emit_event` to `Subscribable`; the
  intersection is a taxonomy question deferred to its own decision.

## Consequences

Host ABI version 1 gains three imports that must be whitelisted and documented
alongside the existing `traverse_*` set, plus a `StatefulStore`-shaped host
trait with a stated guarantee floor and a DataStore v2 default binding. The
runtime gains a per-partition serialization point and a new trace-journal
entry type. `Stateful` capabilities become executable and the Registry Wave 2
roster becomes publishable. No migration or deprecation window is provided,
consistent with Decision 48.

Deferred, each to its own follow-up: a `state_clear(partition)` import, an
embedder-set quota, `Stateful` on `Browser` via an IndexedDB-backed store,
cross-execution compare-and-swap, and the `Stateful` + events multi-role
taxonomy change.

## Alternatives Considered

- **Handle-based ABI** (`state_open` / `read` / `write` / `close` /
  `dispose`) — rejected: five imports, a per-execution handle table for the
  host to manage, and more failure modes (stale/double-close), heavier than any
  existing capability ABI for no roster need.
- **Single multiplexed `state_op(request_json)` import** — rejected: opaque to
  static ABI inspection, which defeats the per-function whitelist that
  `host_abi_v1.json` exists to provide.
- **`capability_id` namespace only, no partition** — rejected: no structural
  per-user boundary; a capability that forgets to namespace keys silently
  shares one user's cart with all, and per-user retention has nothing to
  target.
- **Host-pinned scope from `runtime_config`** — rejected: one activation = one
  scope, so N concurrent users need N activations or per-request rebinding;
  does not fit a long-lived embedded host.
- **Mandate DataStore v2 as the backing store** — rejected: couples the ABI to
  the `518` + `528` + `092` surface, leaves no seam for tests or alternative
  stores, and contradicts the mediation pattern used elsewhere in the runtime.
- **Leave persistence entirely unspecified** — rejected: no durability /
  integrity floor means "Stateful" promises nothing portable and registry
  decision 92's "real managed persistence" bar goes unenforced.
- **Expose contention / add compare-and-swap now** — deferred: every `Stateful`
  capability would have to implement retry/backoff, and CAS widens `get`/`put`
  with version tokens for a case the roster does not yet need.
- **Full value in the trace journal** — rejected: doubles storage, brings
  business data under spec `527` retention/encryption scope, and cuts against
  the journal's metadata-not-payload intent.
- **Relax the `Stateful` + `Browser` ban in this slice** — deferred: widens the
  slice into an amendment of approved spec `014` for a capability class nothing
  in the roster needs.

## Approval evidence

The maintainer directed on 2026-09-08, immediately after the Decision 68
`/brainstorm`, that the spec and ADR be authored and approved rather than left
as drafts. Spec `131-stateful-persistence-host-abi` is recorded in
`specs/governance/approved-specs.json` at version 0.1.0. This ADR records that
approved decision; it does not alter the immutable spec.

# Feature Specification: Capability-Side WASM Host ABI for Event Publish/Subscribe

**Status**: Approved
**Canonical governing ID**: `098-capability-event-host-abi`
**Extends**: `002-capability-contracts`, `003-event-contracts`, `207-event-broker`
**Input**: Issue #969; ADR-0035; `/brainstorm` session recorded as Decisions 48–49 in `docs/decision-log.md`.

## Purpose

Define a WASM host-function ABI that lets a capability imperatively publish
an event during execution, and remove the output-JSON `emitted_events`
convention it replaces. Today a capability declares emitted events inside
its own JSON output, checked post-execution by `PlacementRouter` Step 3.5
against the contract's `emits` list. This spec moves that check to the
point of emission and gives capability authors a real, imperative
publish/subscribe surface, matching UMA's reference model (§5.1.2.2,
`eventDispatcher.dispatch(...)`).

## Capability Boundary

This spec governs the host-function ABI surface (import signatures, calling
convention, synchronous validation behavior) and the removal of the
output-JSON convention it replaces. It is unrelated to and does not modify
`071-native-runtime-wasm-bridge`'s guest-exported `traverse_next_event`
function, which is part of the native embedder-bridge lifecycle ABI, not a
business-event mechanism. It does not change `EventBroker`'s internal
delivery or catalog semantics (`207`), and it does not include the actual
implementation or fixture migration (tracked separately as issue #970).

## Requirements

- **FR-001**: A new host-function import (versioned and whitelisted the same
  way as existing `traverse_*` bridge functions in
  `host_abi_whitelist`/`HOST_ABI_V1_WHITELIST`) MUST let a WASM capability
  publish a `TraverseEvent`-shaped payload during its own execution.
- **FR-002**: The host implementation of this function MUST validate the
  emitted event's `event_type`/`version` against the calling capability's
  contract `emits` list **synchronously, at call time**. An undeclared
  emission MUST be rejected immediately (returning an error to the guest),
  not discovered after execution completes.
- **FR-003**: A capability whose `service_type` is not `Subscribable` MUST
  be rejected by this host function at call time — only `Subscribable`
  capabilities may emit events, matching the existing constraint enforced
  by `PlacementRouter` Step 3.5 today.
- **FR-004**: The output-JSON `emitted_events` convention (a capability
  declaring events inside its JSON output, validated post-execution) MUST
  be removed once this ABI exists — not kept as a second supported path
  (ADR-0035; Decision 48, no back-compat tax pre-production).
- **FR-005**: `PlacementRouter` Step 3.5's post-hoc `undeclared_event_emission`
  violation check MUST be removed once the host-side synchronous check
  (FR-002) supersedes it. `PlacementRouter` Step 5 (publish to `EventBroker`)
  continues to run for events accepted via this ABI.
- **FR-006**: All existing `Subscribable` capability fixtures and tests
  relying on the output-JSON convention MUST be migrated to call this ABI
  instead, with no fixture left on the old convention once this spec's
  implementation lands.
- **FR-007**: The host function signature and its whitelist entry MUST be
  documented in the same location as the existing native-bridge ABI
  whitelist, so the full set of host-callable functions for a given ABI
  version remains discoverable in one place.

## Acceptance Scenarios

1. Given a `Subscribable` capability whose contract declares `emits:
   [{event_id: "x", version: "1.0.0"}]`, when it calls the new host function
   with a matching event, then the event is accepted and later published to
   `EventBroker` by `PlacementRouter` Step 5.
2. Given the same capability calls the host function with an undeclared
   event type, when the call is made, then it is rejected synchronously,
   before execution completes — not discovered afterward as a trace
   violation.
3. Given a capability whose `service_type` is `Stateless`, when it calls the
   event-emit host function, then the call is rejected regardless of
   payload.
4. Given the migration is complete, when the codebase is inspected, then no
   capability fixture or test relies on an `emitted_events` field inside a
   capability's JSON output, and `PlacementRouter`'s post-hoc violation
   check for undeclared emissions no longer exists.

## Out of Scope

- Any change to `071-native-runtime-wasm-bridge`'s `traverse_next_event` or
  other native-bridge lifecycle functions.
- A subscribe-side host function for a capability to receive events
  pushed to it mid-execution — this spec covers publish only; a
  subscribe-side ABI, if needed, is separate future work.
- `EventBroker`'s internal delivery/durability semantics (governed by
  `207`).
- Workflow event-driven edges consuming from this ABI's output (governed
  separately by `099-workflow-event-broker-unification`, issue #971).

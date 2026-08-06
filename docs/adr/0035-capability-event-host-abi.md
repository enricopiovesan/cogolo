# ADR-0035: Replace the Output-JSON Event Convention With an Imperative WASM Host ABI

- Status: Accepted
- Governing spec: `098-capability-event-host-abi`

## Context

No WASM host-function ABI exists for a capability to imperatively publish or
subscribe to events during execution — confirmed by searching
`traverse-native-bridge`, `crates/traverse-runtime/src/executor/wasm.rs`, and
`crates/traverse-runtime/src/executor/native.rs` for any such import (zero
hits). The unrelated guest-exported `traverse_next_event` function
(`071-native-runtime-wasm-bridge`) is part of the native embedder-bridge
lifecycle ABI for hosting the runtime orchestrator in Swift/Kotlin/.NET
shells; it is not a mechanism for WASM business capabilities to emit domain
events and this ADR does not touch it.

Instead, a capability declares an `emitted_events` array inside its own JSON
output. `PlacementRouter` Step 3.5 validates this array *after* execution
completes, checking each declared event against the contract's `emits` list
and rejecting undeclared emissions as a `ContractViolation`
(`undeclared_event_emission`). This is the piece of the runtime furthest
from UMA's reference model, where a microservice calls the runtime's
abstraction layer directly to dispatch an event
(`this.eventDispatcher.dispatch(...)`, UMA white paper §5.1.2.2) rather than
describing events as a side channel in its return value.

Traverse has no production users or production capabilities today
(`docs/decision-log.md` Decision 48).

## Decision

Introduce a new WASM host-function import capabilities call imperatively to
emit an event during execution, and **breaking-replace** the output-JSON
`emitted_events` convention with it rather than keeping both. The host
function validates the emitted event against the contract's `emits` list
synchronously, at call time — an undeclared emission is rejected as it
happens, not discovered after the capability has already finished running.
This follows directly from Decision 48: with no production capabilities
depending on the current convention, there is no cost to replacing it
outright, and a single canonical path avoids permanently maintaining two
ways to emit an event plus the post-hoc violation-detection complexity in
`PlacementRouter` Step 3.5.

## Consequences

Every existing `Subscribable` capability fixture and test that relies on the
output-JSON convention must be migrated to call the new host function
instead (tracked as issue #970). `PlacementRouter`'s Step 3.5 post-hoc
violation check is removed once the host-side synchronous check exists,
since it becomes dead code. The capability host ABI gains a new versioned
surface that must be documented and whitelisted the same way the existing
`traverse_*` bridge functions are (`host_abi_whitelist` in
`crates/traverse-runtime/src/executor/wasm.rs`). No migration or
deprecation window is provided, consistent with Decision 48 — this is a
point-in-time replacement, not a phased rollout.

## Alternatives Considered

- Additive/optional host function alongside the existing output-JSON
  convention — rejected. The user explicitly chose breaking replacement per
  Decision 48, to avoid two permanent paths to the same outcome.
- Leave the output-JSON convention as-is and only add host-side synchronous
  validation on top of it — rejected as a half-measure: it would keep the
  declarative convention that diverges furthest from UMA's imperative model
  while adding complexity, without capturing the actual benefit (a real
  imperative ABI) of doing this work at all.

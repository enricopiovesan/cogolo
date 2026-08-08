# Feature Specification: `LocalExecutor` Event Emission

**Status**: Approved
**Canonical governing ID**: `101-local-executor-event-emission`
**Version**: 1.0.0
**Extends**: `098-capability-event-host-abi`, `207-event-broker`
**Input**: Issue #995; ADR-0037; `/brainstorm` session recorded as Decision 53 in `docs/decision-log.md`.

## Purpose

Close a gap left by `098-capability-event-host-abi`'s implementation
(issue #970): the new `traverse_host::emit_event` WASM ABI only reaches
capabilities executed through `CapabilityExecutor`. A separate,
older trait, `LocalExecutor` (`crates/traverse-runtime/src/lib.rs`), backs
two real production paths — `BoundLocalExecutor` (the live
`Runtime::execute()` single-capability path) and `ArtifactRouter`
(workflow-internal node execution in `workflows.rs`) — and has no channel
to carry emitted events at all. This spec extends `LocalExecutor`'s return
type to carry emitted events, threads them through both paths to
`EventBroker`, and completes `098` FR-004's removal of the output-JSON
`emitted_events` convention, which is still alive in `workflows.rs` because
it was, until now, the only event-emission mechanism available to native
`LocalExecutor` capabilities.

## Capability Boundary

This spec governs `crates/traverse-runtime/src/lib.rs` (the `LocalExecutor`
trait and `BoundLocalExecutor`), `crates/traverse-runtime/src/artifact_router.rs`
(`ArtifactRouter`'s `LocalExecutor` implementation), and
`crates/traverse-runtime/src/workflows.rs` (workflow-internal node
execution, event-driven edge matching, and the new workflow-node event
publish step). It does not change `traverse_host::emit_event`'s WASM ABI
signature or synchronous validation behavior (`098`, unchanged), does not
change `CapabilityExecutor`/`ExecutorOutput` or `PlacementRouter` Step 5's
existing publish logic (both already correct per `098`/#970 and reused
as-is once `BoundLocalExecutor` threads real events through them), and does
not change how a waiting workflow edge subscribes to or consumes
`EventBroker` events (`099-workflow-event-broker-unification`'s domain,
unchanged).

## Requirements

- **FR-001**: `LocalExecutor::execute()` MUST return a value carrying both
  the capability's JSON output and any events it emitted (e.g. a
  `LocalExecutionOutput { value: Value, emitted_events: Vec<TraverseEvent> }`
  struct), replacing the current bare `Result<Value, LocalExecutionFailure>`.
- **FR-002**: `BoundLocalExecutor` MUST thread `LocalExecutionOutput.emitted_events`
  into `ExecutorOutput.emitted_events` when bridging a `LocalExecutor` into
  `CapabilityExecutor`, so `PlacementRouter` Step 5 publishes them for
  `Subscribable` capabilities on the live `Runtime::execute()` path with no
  change to Step 5 itself.
- **FR-003**: `ArtifactRouter::execute` MUST return the real,
  ABI-validated `emitted_events` produced by `WasmExecutor::execute` for
  WASM capabilities, instead of discarding them.
- **FR-004**: `ArtifactRouter` MUST NOT hold an `EventBroker` reference or
  publish events internally. It is used both directly by `workflows.rs`
  (bypassing `PlacementRouter`) and, wrapped in `BoundLocalExecutor`, by
  `PlacementRouter` Step 5 for the live path; publishing from within
  `ArtifactRouter` would double-publish on the live path.
- **FR-005**: `workflows.rs::execute_workflow_capability` MUST publish a
  workflow node's `emitted_events` to `EventBroker` when the executing
  capability's `service_type == Subscribable`, structurally analogous to
  `PlacementRouter` Step 5 (same gate, same best-effort semantics: a
  publish error is recorded but does not fail the workflow step). This
  closes the gap where workflow-node-emitted events previously satisfied
  only same-execution waiting edges and never reached `EventBroker`.
- **FR-006**: The output-JSON `emitted_events` convention in `workflows.rs`
  (`emitted_events(output: &Value)`, parsing an `emitted_events` array out
  of a capability's own JSON output) MUST be removed. Workflow-internal
  Pass-1 event-driven edge matching MUST read from the structured
  `LocalExecutionOutput.emitted_events` field instead.
- **FR-007**: An event populated directly by a native `LocalExecutor`
  implementor (a host closure, or one of `ArtifactRouter`'s registered
  native handlers) MUST be validated against the executing capability
  contract's `emits` list (`event_id` + `version`) and
  `service_type == Subscribable` before publish, mirroring `098`
  FR-002/FR-003's synchronous WASM-boundary validation. This check
  necessarily runs after the native closure has already returned (there is
  no host-function call boundary to reject mid-call, unlike WASM), but MUST
  still run before any publish to `EventBroker`.
- **FR-008**: A native-emitted event that fails FR-007's validation MUST
  fail the whole capability/node execution (the same severity as a WASM ABI
  synchronous rejection under `098` FR-002), not be silently dropped with
  only a warning.
- **FR-009**: All existing `LocalExecutor` implementors and call sites in
  this workspace (test doubles in `workflows.rs` and router tests,
  `traverse-cli/src/main.rs`, `traverse-cli/src/http_api.rs`,
  `traverse-mcp/src/lib.rs`, `traverse-mcp/src/stdio_server.rs`,
  `examples/load_workspace_app_state.rs`) MUST be migrated to the new
  `LocalExecutionOutput` return type.
- **FR-010**: Tests that currently document the gap this spec closes as
  expected behavior — `lib.rs`'s
  `bound_local_executor_never_publishes_events_through_placement_router`
  and `tests/placement_router_live_wiring.rs`'s
  `live_native_execution_completes_and_writes_trace_without_publishing_events`
  — MUST be rewritten to assert the corrected behavior (events do publish).

## Acceptance Scenarios

1. Given a `Subscribable` WASM capability invoked via `Runtime::execute()`'s
   live path (backed by `ArtifactRouter` wrapped in `BoundLocalExecutor`)
   that calls `traverse_host::emit_event` with a declared event, when
   execution completes, then the event is published to `EventBroker`.
2. Given a `Subscribable` WASM capability inside a workflow node that calls
   `traverse_host::emit_event` with a declared event, when the node
   completes, then the event both satisfies same-execution waiting edges
   (if a matching edge exists) and is published to `EventBroker` for
   external consumers.
3. Given a native `LocalExecutor` capability (a host closure, or an
   `ArtifactRouter` native handler) whose `service_type` is `Subscribable`
   and that populates `LocalExecutionOutput.emitted_events` with an event
   declared in its contract's `emits` list, when execution completes, then
   the event is published to `EventBroker`.
4. Given a native `LocalExecutor` capability that populates
   `LocalExecutionOutput.emitted_events` with an event NOT declared in its
   contract's `emits` list, when execution completes, then the capability's
   execution result is a failure and no event reaches `EventBroker`.
5. Given a native `LocalExecutor` capability whose `service_type` is not
   `Subscribable` that populates `LocalExecutionOutput.emitted_events` with
   any event, when execution completes, then the capability's execution
   result is a failure and no event reaches `EventBroker`.
6. Given a workflow node capability with no emitted events, when the node
   completes, then no publish call is made and existing traversal behavior
   is unchanged.
7. Given `EventBroker` is unreachable when `workflows.rs` attempts to
   publish a workflow node's emitted event, when the publish is attempted,
   then the error is recorded (matching `PlacementRouter` Step 5's
   best-effort semantics) and the workflow step still completes
   successfully — a broker outage does not fail workflow traversal.

## Out of Scope

- Changes to `traverse_host::emit_event`'s WASM ABI signature or
  synchronous validation behavior (`098-capability-event-host-abi`).
- Changes to `CapabilityExecutor`, `ExecutorOutput`, or `PlacementRouter`
  Step 5's existing publish logic.
- Changes to how a waiting workflow edge subscribes to or consumes
  `EventBroker` events (`099-workflow-event-broker-unification`).
- Collapsing `LocalExecutor` into `CapabilityExecutor` (considered and
  rejected in ADR-0037 as disproportionate to this gap).
- A native-side host-callback API giving `BoundLocalExecutor`'s
  host-provided closures a richer, ABI-validated event-emission mechanism
  beyond directly populating `LocalExecutionOutput.emitted_events` —
  deferred as a distinct future problem if ever needed.

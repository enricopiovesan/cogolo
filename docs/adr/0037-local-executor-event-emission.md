# ADR-0037: Extend `LocalExecutor` to Carry Emitted Events

- Status: Accepted
- Governing spec: `101-local-executor-event-emission`

## Context

`098-capability-event-host-abi` (issue #969) defined `traverse_host::emit_event`,
a WASM host function letting a capability imperatively publish an event
during execution, validated synchronously against the capability's contract
`emits` list and `service_type`. Issue #970 implemented it end to end for
`CapabilityExecutor::execute() -> Result<ExecutorOutput, ExecutorError>`:
`WasmExecutor` populates `ExecutorOutput.emitted_events` from validated host
calls, and `PlacementRouter` Step 5 publishes them to `EventBroker` for
`Subscribable` capabilities.

That implementation only reaches `CapabilityExecutor`. A separate, older
trait, `LocalExecutor::execute() -> Result<Value, LocalExecutionFailure>`
(`crates/traverse-runtime/src/lib.rs`), has no event channel at all, and two
real production paths use it:

- `BoundLocalExecutor` bridges a host-provided native `LocalExecutor`
  closure into `CapabilityExecutor` for `Runtime::execute()`'s live
  single-capability path. It always returns an empty `emitted_events`,
  regardless of what the underlying closure does — there is structurally no
  ABI for a native closure to call, since `traverse_host::emit_event` is a
  WASM guest/host import.
- `ArtifactRouter` implements `LocalExecutor` directly and is the executor
  used for workflow-internal node execution (`workflows.rs`). For WASM
  capabilities it calls `WasmExecutor::execute` internally — which *does*
  produce real, ABI-validated `ExecutorOutput.emitted_events` — but discards
  them (`.map(|output| output.value)`) before returning.

`traverse-cli`'s production wiring (`main.rs`) constructs
`Runtime::new(registry, ArtifactRouter::new()?)`, so `ArtifactRouter` is the
same executor instance backing both paths: workflow-internal node execution
(direct call, bypassing `PlacementRouter`) and the live single-capability
path (wrapped in `BoundLocalExecutor`, routed through `PlacementRouter`).

A second, related gap: `098` FR-004 required removing the old output-JSON
`emitted_events` convention (a capability embedding an `emitted_events`
array in its own JSON output) "once this ABI exists — not kept as a second
supported path." `workflows.rs`'s `emitted_events(&output: &Value)`
JSON-parsing helper is still in use, because it was — until now — the only
event-emission mechanism available to native (non-WASM) capabilities inside
a workflow. It is also narrower than `EventBroker`-backed emission: it only
ever satisfies waiting edges within the *same* workflow execution, never
reaching `EventBroker` for other workflows, capabilities, or external
subscribers.

Traverse has no production capabilities depending on `LocalExecutor`'s
current signature today (`docs/decision-log.md` Decision 48).

## Decision

Extend `LocalExecutor::execute()`'s return type to carry emitted events
alongside its value output, mirroring `ExecutorOutput`:

```rust
pub struct LocalExecutionOutput {
    pub value: Value,
    pub emitted_events: Vec<TraverseEvent>,
}

pub trait LocalExecutor: Send + Sync {
    fn execute(
        &self,
        capability: &ResolvedCapability,
        input: &Value,
    ) -> Result<LocalExecutionOutput, LocalExecutionFailure>;
}
```

This is a breaking change to a public, embedder-facing trait (roughly a
dozen implementors/call sites across `traverse-runtime`, `traverse-cli`, and
`traverse-mcp`), accepted because it is the only shape that gives native
`LocalExecutor` implementors — host-provided closures, `ArtifactRouter`'s
native handlers — an actual, structural channel to emit events at all. A
narrower fix scoped only to `ArtifactRouter`'s WASM path would leave native
capabilities with no channel; collapsing `LocalExecutor` into
`CapabilityExecutor` entirely was considered and rejected as disproportionate
to this gap (see Alternatives).

`BoundLocalExecutor` threads `LocalExecutionOutput.emitted_events` into
`ExecutorOutput.emitted_events`, so `PlacementRouter` Step 5 publishes them
for the live `Runtime::execute()` path with no further changes to `Step 5`
itself. `ArtifactRouter` returns real `WasmExecutor`-sourced events for WASM
capabilities instead of discarding them, and native handlers may populate
`emitted_events` directly.

`ArtifactRouter` itself does not hold an `EventBroker` reference and does
not publish internally — it is used both directly by `workflows.rs` and,
via `BoundLocalExecutor`, by `PlacementRouter` Step 5, and publishing from
within `ArtifactRouter` would double-publish on the live path.
`workflows.rs::execute_workflow_capability` gains its own publish step,
structurally analogous to `PlacementRouter` Step 5 (same `Subscribable`
gate, same best-effort publish semantics), since workflow-internal node
execution bypasses `PlacementRouter` entirely.

The old output-JSON `emitted_events` convention in `workflows.rs` is
removed. Pass-1 event-driven edge matching reads from the new structured
`LocalExecutionOutput.emitted_events` field instead of parsing the node's
JSON output — completing `098` FR-004 across the codebase, not just the
`executor`/`router` slice it originally covered.

Because native `LocalExecutor` implementors are unsandboxed host code (unlike
WASM, which is validated synchronously inside the host function at call
time), natively-populated events are validated against the capability
contract's `emits` list and `service_type == Subscribable` before publish —
after the closure has already returned, since there is no way to reject
mid-call the way the WASM host function can. An undeclared or invalid native
event fails the whole capability/node execution, matching the severity of a
WASM ABI rejection, so "emitted events are always declared" remains a real
guarantee on the native path too.

## Consequences

`LocalExecutor` implementors across the workspace (test doubles in
`workflows.rs` and router tests, `traverse-cli/src/main.rs`,
`traverse-cli/src/http_api.rs`, `traverse-mcp/src/lib.rs`,
`traverse-mcp/src/stdio_server.rs`,
`examples/load_workspace_app_state.rs`) must migrate to the new return
type. Existing tests that document the gap as expected behavior
(`bound_local_executor_never_publishes_events_through_placement_router` in
`lib.rs`, `live_native_execution_completes_and_writes_trace_without_publishing_events`
in `tests/placement_router_live_wiring.rs`) must be rewritten to assert the
corrected behavior. `workflows.rs` gains a new validation-and-publish
responsibility it did not previously have, and its trace evidence
(`WorkflowTraversalEvidence.emitted_events`) is sourced from the structured
field rather than JSON parsing. Downstream embedders implementing
`LocalExecutor` directly against this crate must update their
implementations at the next version bump — acceptable per Decision 48 given
no production capabilities exist yet.

## Alternatives Considered

- Narrow fix scoped to `ArtifactRouter` only (inject an `EventBroker`
  reference directly, publish from within `ArtifactRouter`, no trait
  signature change) — smaller and non-breaking, and was the initially
  recommended option. Rejected because it leaves native `LocalExecutor`
  implementors with no event-emission channel at all, and because
  publishing from within `ArtifactRouter` itself would double-publish on
  the live `Runtime::execute()` path once `ArtifactRouter` is also wrapped
  by `BoundLocalExecutor`/`PlacementRouter` Step 5.
- Collapse `LocalExecutor` into `CapabilityExecutor` entirely, removing the
  dual-trait split at its root — rejected as disproportionate: it would
  rewrite `BoundLocalExecutor`'s role, `ArtifactRouter`'s trait impl, every
  workflow test double, and both embedders' integration simultaneously, for
  a gap this ADR closes with a narrower, additive-field change.
- Keep the output-JSON `emitted_events` convention in `workflows.rs` as a
  documented fallback alongside the new structured field — rejected: keeps
  a second supported path indefinitely, directly contradicting `098` FR-004
  and Decision 48's no-back-compat-tax principle.
- Drop invalid native-emitted events with a warning instead of failing
  execution — rejected: weakens "emitted events are always declared" to
  something easy to silently miss, inconsistent with the WASM ABI's
  synchronous-rejection guarantee.

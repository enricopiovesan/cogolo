# Expedition Example Smoke Validation

Run the expedition artifact smoke path with:

```bash
bash scripts/ci/expedition_artifact_smoke.sh
```

What it validates:

- the six expected expedition capability contracts exist
- the five expected expedition event contracts exist
- the canonical `plan-expedition` workflow artifact exists
- the governed capability, event, and workflow ids match the approved expedition example specs

To verify the failure path without touching the checked-in artifacts:

```bash
tmpdir="$(mktemp -d)"
cp -R contracts workflows "$tmpdir/"
rm "$tmpdir/contracts/examples/expedition/events/expedition-plan-assembled/contract.json"
TRAVERSE_REPO_ROOT="$tmpdir" bash scripts/ci/expedition_artifact_smoke.sh
```

The command should fail with a missing-artifact message.

Run the expedition execution smoke path with:

```bash
bash scripts/ci/expedition_execution_smoke.sh
```

What it validates:

- the canonical expedition runtime request can execute `expedition.planning.plan-expedition`
- the output includes a completed workflow-backed planning result
- invalid expedition execution input fails deterministically

Run the expedition trace smoke path with:

```bash
bash scripts/ci/expedition_trace_smoke.sh
```

What it validates:

- the expedition execution path can persist a governed runtime trace artifact
- the trace inspection command renders deterministic trace metadata for the canonical request
- malformed trace input fails deterministically

Run the expedition golden-path validation with:

```bash
bash scripts/ci/expedition_golden_path.sh
```

What it validates:

- the canonical expedition bundle registers successfully
- the canonical expedition request executes successfully and produces a trace artifact
- the trace inspection command renders the generated trace deterministically
- the validation fails deterministically when the bundle is incomplete

Run the local browser adapter smoke path with:

```bash
bash scripts/ci/browser_adapter_smoke.sh
```

What it validates:

- the local browser adapter serves the governed browser subscription transport on a local HTTP port
- subscription creation returns the governed `local_browser_subscription_created` response for the canonical expedition request
- the SSE stream emits the governed lifecycle, state, trace, and terminal messages for the canonical execution
- invalid create requests and missing stream requests fail deterministically

Run the event-driven workflow smoke path with:

```bash
bash scripts/ci/event_driven_workflow_smoke.sh
```

What it validates:

- event-driven workflow traversal still completes deterministically through governed event edges
- the runtime records deterministic wake ordering and per-edge exact-once event consumption
- simple event payload predicates reject mismatches deterministically
- invalid event links are rejected deterministically by workflow registration validation

Run the governed WASM AI agent smoke path with:

```bash
bash scripts/ci/wasm_agent_example_smoke.sh
```

What it validates:

- the first governed WASM AI agent package can build its deterministic local WASM fixture
- the package inspection command validates the approved capability, workflow linkage, and binary digest
- the agent executes through the approved Traverse runtime request path without ad hoc private routes

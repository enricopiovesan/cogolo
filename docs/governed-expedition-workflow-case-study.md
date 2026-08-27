# Governed expedition workflow case study

The `expedition.planning.plan-expedition@1.0.0` reference workflow is a
reproducible example of composition through Traverse rather than application
code copying capability business logic. It is a deterministic, local reference
case only; it is not evidence of autonomous planning, universal portability,
or production performance.

## Goal and authority boundary

The workflow turns a supplied expedition objective, intent, and team profile
into a bounded plan. The application owns the request, presentation, and any
human decision following the result. Traverse owns contract validation,
workflow ordering, capability execution, and the redacted runtime trace.

The governed workflow artifact is
[`plan-expedition`](../workflows/examples/expedition/plan-expedition/workflow.json).
It composes five pinned `1.0.0` capabilities: capture objective, interpret
intent, assess conditions, validate readiness, and assemble plan. Their
contracts and the workflow are registered through the
[`expedition bundle`](../examples/expedition/registry-bundle/manifest.json).

## Shared composition versus application glue

| Shared, governed | Application-specific glue |
| --- | --- |
| Five capability contracts, WASM artifacts, versions, execution order, and trace model | The expedition request values and caller-owned rendering of the plan |
| Schema validation and deterministic failure result | Choosing whether a human follows the recommendation |
| Registry bundle and `expedition execute` CLI path | No UI, provider, prompt, credential, or model code is part of this case |

## Reproduce the evidence

```bash
bash scripts/ci/expedition_execution_smoke.sh
bash scripts/ci/expedition_trace_smoke.sh
bash scripts/ci/expedition_golden_path.sh
```

The first command proves a real successful execution and a rejection when the
required `planning_intent` input is removed. The failure is explicit; recovery
means submitting a corrected request as a new execution. It is not an automatic
retry, compensation, or mutable workflow update.

The trace command writes a runtime trace and inspects its redacted metadata.
It identifies the execution, selected workflow capability, terminal outcome,
and state transitions without publishing the request payload.

## Local performance profile

Measured on 2026-08-27 in this repository's local development environment,
the following cold CLI command completed in **2.28 s real**, **0.67 s user**,
and **0.18 s system** time:

```bash
/usr/bin/time -p cargo run -q -p traverse-cli-rs -- expedition execute \
  examples/expedition/runtime-requests/plan-expedition.json \
  --trace-out /private/tmp/plan-expedition-trace.json
```

This is a single-process developer-machine sample, including CLI and Cargo
overhead. It is versioned evidence for this fixture, not a latency SLO or a
claim about another host, workload, or concurrency level.

## Remaining manual boundaries

The input is prepared by a caller and the output is reviewed by a human. This
case does not execute an LLM, external connector, browser animation, automatic
approval, retry, or compensation path.

# Expedition Example Authoring Guide

Traverse keeps the canonical expedition example artifacts in three governed locations:

```text
contracts/examples/expedition/
workflows/examples/expedition/
examples/expedition/registry-bundle/
```

## Artifact Categories

Atomic capability contracts:

- `contracts/examples/expedition/capabilities/capture-expedition-objective/contract.json`
- `contracts/examples/expedition/capabilities/interpret-expedition-intent/contract.json`
- `contracts/examples/expedition/capabilities/assess-conditions-summary/contract.json`
- `contracts/examples/expedition/capabilities/validate-team-readiness/contract.json`
- `contracts/examples/expedition/capabilities/assemble-expedition-plan/contract.json`

Workflow-backed composed capability contract:

- `contracts/examples/expedition/capabilities/plan-expedition/contract.json`

Event contracts:

- `contracts/examples/expedition/events/expedition-objective-captured/contract.json`
- `contracts/examples/expedition/events/expedition-intent-interpreted/contract.json`
- `contracts/examples/expedition/events/conditions-summary-assessed/contract.json`
- `contracts/examples/expedition/events/team-readiness-validated/contract.json`
- `contracts/examples/expedition/events/expedition-plan-assembled/contract.json`

Workflow artifact:

- `workflows/examples/expedition/plan-expedition/workflow.json`

Registry bundle manifest:

- `examples/expedition/registry-bundle/manifest.json`

## Authoring Rules

- Keep ids and versions aligned with the approved expedition specs.
- Do not invent alternate names for the canonical expedition capabilities, events, or workflow.
- Treat the contracts and workflow artifacts under `contracts/examples/` and `workflows/examples/` as the source of truth.
- Treat the registry bundle manifest as a projection over those governed artifacts, not a replacement for them.

## Validation Commands

Artifact smoke validation:

```bash
bash scripts/ci/expedition_artifact_smoke.sh
```

Registry bundle inspection:

```bash
cargo run -p traverse-cli -- bundle inspect examples/expedition/registry-bundle/manifest.json
```

Event contract inspection:

```bash
cargo run -p traverse-cli -- event inspect contracts/examples/expedition/events/expedition-objective-captured/contract.json
```

Workflow artifact inspection:

```bash
cargo run -p traverse-cli -- workflow inspect workflows/examples/expedition/plan-expedition/workflow.json
```

Repository checks:

```bash
bash scripts/ci/repository_checks.sh
```

## What Good Output Looks Like

The bundle inspection output must include:

- `expedition.planning.capture-expedition-objective`
- `expedition.planning.interpret-expedition-intent`
- `expedition.planning.assess-conditions-summary`
- `expedition.planning.validate-team-readiness`
- `expedition.planning.assemble-expedition-plan`
- `expedition.planning.plan-expedition`

And the workflow section must include:

- `expedition.planning.plan-expedition@1.0.0`

The event inspection output must include:

- `id: expedition.planning.expedition-objective-captured`
- `event_type: domain`
- `publisher_ids:`

The workflow inspection output must include:

- `id: expedition.planning.plan-expedition`
- `start_node: capture_objective`
- `node_capabilities:`

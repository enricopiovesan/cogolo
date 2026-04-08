# Traverse WASM Agent Authoring Guide

This guide shows how to create a new governed WASM agent in Traverse without inventing a separate packaging model.

Use the checked-in examples as the source of truth:

- [`examples/templates/executable-capability-package/manifest.template.json`](/Users/piovese/Documents/cogolo/examples/templates/executable-capability-package/manifest.template.json)
- [`examples/agents/expedition-intent-agent/manifest.json`](/Users/piovese/Documents/cogolo/examples/agents/expedition-intent-agent/manifest.json)
- [`examples/agents/team-readiness-agent/manifest.json`](/Users/piovese/Documents/cogolo/examples/agents/team-readiness-agent/manifest.json)

## Start From a Governed Package

Begin with the executable capability package template, then specialize it for the new agent:

- choose one governed `package_id`
- bind exactly one approved capability contract
- bind the agent to the workflow it participates in
- keep the source entry point explicit
- keep the binary path and digest explicit
- keep host API, network, and filesystem access governed and narrow
- declare model dependencies as abstract interfaces, not direct implementation hooks

## Minimal Package Shape

A new agent package should make these fields obvious:

- `package_id`
- `version`
- `summary`
- `capability_ref`
- `workflow_refs`
- `source`
- `binary`
- `constraints`
- `model_dependencies`

The package must remain a portable WASM-backed artifact bundle, not a generic host-bound executable.

## Authoring Steps

1. Copy the template manifest into a new agent directory.
2. Replace the placeholder capability and workflow references with approved Traverse ids.
3. Point `source.path` at the agent implementation file.
4. Build the deterministic local fixture for the agent package.
5. Update the expected digest after the fixture is built.
6. Verify the package with `traverse-cli agent inspect`.
7. Verify the runtime path with `traverse-cli agent execute`.
8. Run the example smoke script before opening a PR.

## Validation

Run the agent authoring smoke path with:

```bash
bash scripts/ci/wasm_agent_authoring_guide_smoke.sh
```

That smoke path confirms the guide points at the governed template, the approved example packages, and the deterministic Traverse CLI validation flow.

## Common Mistakes

- skipping the governed manifest and improvising a package shape
- declaring host access that is broader than the approved capability contract allows
- treating the example as a general microservice instead of a governed agent package
- forgetting to link the agent package to a workflow reference
- changing the binary digest without rebuilding the fixture

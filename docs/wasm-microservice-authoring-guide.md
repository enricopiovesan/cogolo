# Traverse WASM Microservice Authoring Guide

This guide shows how to create a new governed WASM microservice in Traverse without turning it into a host-coupled service deployment.

Traverse does not currently treat a microservice as a separate runtime category from the governed package model. Instead, a WASM microservice should be authored as a governed executable package that is explicit about its boundaries, dependencies, and validation path.

Use the checked-in examples and references as the source of truth:

- [`examples/templates/executable-capability-package/manifest.template.json`](../examples/templates/executable-capability-package/manifest.template.json)
- [`docs/adapter-boundaries.md`](adapter-boundaries.md)
- [`docs/compatibility-policy.md`](compatibility-policy.md)
- [`docs/oss-pattern-extraction.md`](oss-pattern-extraction.md)

## Start From a Governed Package

Begin with the executable capability package template, then adapt it for the new microservice:

- choose one governed `package_id`
- bind exactly one approved capability contract
- keep the source entry point explicit
- keep the binary path and digest explicit
- keep host API, network, and filesystem access governed and narrow
- declare model dependencies as abstract interfaces, not direct implementation hooks
- document the app-facing or integration-facing boundary the microservice is meant to serve

## Minimal Package Shape

A new WASM microservice package should make these fields obvious:

- `package_id`
- `version`
- `summary`
- `capability_ref`
- `workflow_refs`
- `source`
- `binary`
- `constraints`
- `model_dependencies`

The package must remain portable and governed. It should not depend on a hidden host process or a separately managed deployment topology to make sense.

## Authoring Steps

1. Copy the template manifest into a new package directory.
2. Replace the placeholder capability and workflow references with approved Traverse ids.
3. Point `source.path` at the microservice implementation file.
4. Build the deterministic local fixture for the package.
5. Update the expected digest after the fixture is built.
6. Document how the microservice interacts with Traverse adapters or client surfaces.
7. Validate the package with the same repository-level smoke and repo-check flow used for other governed package examples.
8. Open the PR only after the docs and validation references are in sync.

## Validation

Run the microservice authoring smoke path with:

```bash
bash scripts/ci/wasm_microservice_authoring_guide_smoke.sh
```

That smoke path confirms the guide points at the governed template, the adapter boundary docs, and the deterministic Traverse validation flow.

## Common Mistakes

- treating the microservice as a host-bound deployment instead of a governed package
- omitting the digest or the workflow/capability linkage
- broadening host or network access beyond what the package explicitly documents
- skipping adapter-boundary documentation
- changing the binary digest without rebuilding the fixture

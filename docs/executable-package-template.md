# Executable Capability Package Template

Traverse keeps executable capability packages deterministic and colocated.

Use the template under:

- `examples/templates/executable-capability-package/`

It provides:

- `manifest.template.json`
- `build-fixture.sh`
- `src/implementation.rs`
- `artifacts/.gitignore`

## What the template standardizes

Every executable capability package should declare:

- one package id and semver package version
- one governed capability contract reference
- any approved workflow references the package participates in
- one colocated source path and entry symbol
- one colocated binary path and declared digest
- explicit execution constraints
- explicit model dependency declarations
- one deterministic local build command

## Expected manifest shape

The manifest template follows this pattern:

- `kind: agent_package`
- `schema_version: 1.0.0`
- `package_id`
- `version`
- `summary`
- `capability_ref`
- `workflow_refs`
- `source`
- `binary`
- `constraints`
- `model_dependencies`

## Expected build pattern

The colocated build command should:

- write the package binary into `./artifacts/`
- be deterministic
- avoid hidden external state
- print the artifact path it built

The checked-in template uses a tiny deterministic WASM fixture so packaging, inspection, and smoke validation can stay runnable even when a full external WASM toolchain is not available.

## How to use the template

1. Copy `examples/templates/executable-capability-package/` into a new package folder.
2. Replace the placeholder package and capability ids.
3. Point `capability_ref.contract_path` at the governed capability contract.
4. Replace the placeholder workflow refs with the approved workflow links you need.
5. Replace the placeholder model interface with the real abstract dependency.
6. Run the colocated `build-fixture.sh` script to produce the deterministic binary.

## Validation

Verify the template path stays complete with:

```bash
bash scripts/ci/executable_package_template_smoke.sh
```

That smoke path checks:

- the template manifest exists
- the template build script exists
- the template source stub exists
- the template artifact ignore file exists
- the manifest still contains the governed package fields the repo expects

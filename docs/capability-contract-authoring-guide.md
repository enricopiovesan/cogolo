# Capability Contract Authoring Guide

This guide is for writing a new Traverse **capability contract** from scratch.

Use it when you are a human developer or coding agent and you want a contract you can:

- validate locally
- include in a registry bundle
- register and execute through the runtime

If you are instead trying to package a real WASM implementation, start with:

- [docs/wasm-microservice-authoring-guide.md](./wasm-microservice-authoring-guide.md)
- [docs/wasm-agent-authoring-guide.md](./wasm-agent-authoring-guide.md)

## What A Capability Contract Is

A capability contract is the governed source of truth for a business action:

- identity (`id`, `version`)
- inputs and outputs (JSON Schemas)
- declared side effects and event edges (`side_effects`, `emits`, `consumes`)
- execution constraints (network/filesystem/host API rules)

Traverse registries and runtime behavior must conform to the contract. The contract is not a CLI convenience layer.

## Minimal Working Contract Template

Start from this template and replace only the values marked `demo.*`.

```json
{
  "kind": "capability_contract",
  "schema_version": "1.0.0",
  "id": "demo.echo",
  "namespace": "demo",
  "name": "echo",
  "version": "1.0.0",
  "lifecycle": "draft",
  "owner": {
    "team": "your-team",
    "contact": "you@example.com"
  },
  "summary": "Echo the request payload.",
  "description": "A minimal capability contract used to validate a local authoring workflow.",
  "inputs": {
    "schema": {
      "type": "object",
      "required": ["message"],
      "properties": {
        "message": { "type": "string" }
      },
      "additionalProperties": false
    }
  },
  "outputs": {
    "schema": {
      "type": "object",
      "required": ["message"],
      "properties": {
        "message": { "type": "string" }
      },
      "additionalProperties": false
    }
  },
  "preconditions": [
    {
      "id": "input-provided",
      "description": "The message field is provided."
    }
  ],
  "postconditions": [
    {
      "id": "echo-produced",
      "description": "The output contains the same message."
    }
  ],
  "side_effects": [
    {
      "kind": "none",
      "description": "No side effects."
    }
  ],
  "emits": [],
  "consumes": [],
  "permissions": [
    { "id": "demo.echo.execute" }
  ],
  "execution": {
    "binary_format": "wasm",
    "entrypoint": {
      "kind": "wasi-command",
      "command": "run"
    },
    "preferred_targets": ["local"],
    "constraints": {
      "host_api_access": "none",
      "network_access": "forbidden",
      "filesystem_access": "none"
    }
  },
  "policies": [
    { "id": "manual-approval-required" }
  ],
  "dependencies": [],
  "provenance": {
    "source": "greenfield",
    "author": "your-handle",
    "created_at": "2026-04-18T00:00:00Z",
    "spec_ref": "002-capability-contracts@1.0.0",
    "adr_refs": [],
    "exception_refs": []
  },
  "evidence": [],
  "service_type": "stateless",
  "permitted_targets": ["local", "cloud", "edge", "device"]
}
```

## Field-By-Field Notes

These notes are intentionally practical. For the governing spec, start from the capability contracts spec in `specs/`.

- `kind`: Must be `capability_contract`.
- `schema_version`: Must be `1.0.0`.
- `id`: Globally unique string for the capability, typically `namespace.name`.
- `namespace`: A stable grouping prefix for related capabilities.
- `name`: Short name within the namespace.
- `version`: SemVer string. Increment when the contract changes.
- `lifecycle`: Use `draft` while iterating; publishable flows require `active`.
- `owner`: Human accountability. Keep it stable so downstream consumers know who to ask.
- `summary` / `description`: Used for discovery surfaces and documentation.
- `inputs.schema` / `outputs.schema`: JSON Schema objects. Keep them strict enough for deterministic validation.
- `preconditions` / `postconditions`: Human-readable intent. These are not a runtime policy language.
- `side_effects`: Declare any effect beyond returning data. If you truly have none, declare `kind: none`.
- `emits`: List of event contracts this capability publishes.
- `consumes`: List of event contracts this capability subscribes to (for event-driven execution models).
- `permissions`: Access control identifiers. Keep them stable; treat them as part of the app-facing contract.
- `execution`: How the runtime may execute this capability. Even if you are not packaging the WASM yet, set constraints now.
- `execution.constraints`: The safety envelope. Defaults are not implied; make your intent explicit.
- `policies`: Governance policy identifiers (for example, manual approval).
- `dependencies`: Contract-time dependencies (events, workflows, or other artifacts) required for validation/registration.
- `provenance`: Traceability. `spec_ref` should point at the governing spec that justifies the contract shape.
- `evidence`: Validation evidence records (often empty for a new draft).
- `service_type`: UMA service type; influences placement and routing. Default is `stateless` but set it explicitly.
- `permitted_targets`: Allowed runtime targets. If you only support local initially, restrict this list.

## Where “Artifact Type” Lives

The capability contract describes governed behavior and constraints. The runtime execution **artifact type** (for example, whether the implementation artifact is native or WASM) is carried by the **packaged executable capability** metadata, not by the capability contract itself.

If you are authoring a real executable package next, use the template:

- `examples/templates/executable-capability-package/manifest.template.json`

## Step-By-Step: Create → Validate → Register

This guide assumes you are following the existing “first capability” path and reusing the checked-in bundle wiring rather than inventing a brand new bundle layout.

1. Create your contract JSON under `contracts/` (pick a new directory that matches your domain).
2. Validate it locally by running the contract validation suite:

```bash
cargo test -p traverse-contracts
```

3. Include it in a registry bundle manifest (see [docs/getting-started.md](./getting-started.md) for how the expedition example is wired).
4. Inspect the bundle (this should fail fast if the contract is malformed):

```bash
cargo run -p traverse-cli -- bundle inspect <path-to-manifest.json>
```

5. Register the bundle:

```bash
cargo run -p traverse-cli -- bundle register <path-to-manifest.json>
```

## Common Mistakes

- Missing a required top-level field. The contract schema is intentionally explicit.
- Using permissive schemas (`additionalProperties: true`) without realizing it reduces determinism.
- Declaring side effects implicitly (for example, “it emits an event”) but forgetting to declare `side_effects` and `emits`.
- Forgetting to restrict `execution.constraints` early, then discovering you accidentally depended on host/network access.


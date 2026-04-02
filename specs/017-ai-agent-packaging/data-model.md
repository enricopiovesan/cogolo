# Data Model: AI Agent Execution and WASM Agent Packaging

## Purpose

This document defines the implementation-tight artifacts for the `017-ai-agent-packaging` slice.

It governs packaged-agent manifests, package files, declared Traverse surface bindings, runtime constraints, validation evidence, and execution evidence.

## 1. AI Agent Package Manifest

Represents the authoritative machine-readable artifact for one governed packaged agent.

### Required Fields

- `kind`
- `schema_version`
- `governing_spec`
- `id`
- `version`
- `summary`
- `owner`
- `lifecycle`
- `agent_kind`
- `entrypoint`
- `module`
- `surface_bindings`
- `runtime_constraints`

### Optional Fields

- `description`
- `tags`
- `instruction_files`
- `provenance`
- `mcp_exposure`

### Shape

```json
{
  "kind": "ai_agent_package",
  "schema_version": "1.0.0",
  "governing_spec": "017-ai-agent-packaging",
  "id": "expedition.planning.interpret-expedition-intent-agent",
  "version": "1.0.0",
  "summary": "Interprets expedition intent into structured Traverse planning input.",
  "owner": {
    "team": "traverse.examples",
    "contact": "team@traverse.local"
  },
  "lifecycle": "active",
  "agent_kind": "planner_assistant",
  "entrypoint": {
    "export": "run_agent",
    "request_schema": "runtime_request",
    "result_schema": "runtime_result"
  },
  "module": {
    "path": "artifacts/interpret-expedition-intent-agent.wasm",
    "digest": "sha256:agent-module...",
    "abi": "wasm32-wasip1",
    "runtime": "wasmtime"
  },
  "surface_bindings": [
    {
      "binding_kind": "capability",
      "id": "expedition.planning.interpret-expedition-intent",
      "version": "1.0.0",
      "mode": "invoke"
    },
    {
      "binding_kind": "workflow",
      "id": "expedition.planning.plan-expedition",
      "version": "1.0.0",
      "mode": "invoke"
    }
  ],
  "runtime_constraints": {
    "placement": "local",
    "required_host_features": [
      "traverse_runtime_v1"
    ],
    "max_input_bytes": 65536,
    "network_access": "forbidden"
  },
  "instruction_files": [
    {
      "path": "instructions/system.md",
      "digest": "sha256:instructions..."
    }
  ],
  "provenance": {
    "source": "examples",
    "built_at": "2026-04-01T00:00:00Z"
  },
  "mcp_exposure": {
    "intent": "future_optional",
    "surface_name": "expedition-intent-agent"
  }
}
```

### Rules

- `kind` must be `ai_agent_package`.
- `governing_spec` must be `017-ai-agent-packaging`.
- `module` defines the single primary executable WASM artifact in this slice.
- `surface_bindings` must contain at least one approved Traverse capability or workflow reference.
- `mcp_exposure` is declarative only in this slice; it does not define MCP protocol behavior.

## 2. Agent Kind Enum

Represents the governed high-level role for one packaged agent.

### Enum Values

- `planner_assistant`
- `workflow_operator`
- `capability_specialist`

### Rules

- `agent_kind` describes the role of the packaged agent, not its implementation mechanism.
- Additional values require explicit governed change.

## 3. Agent Entrypoint

Represents the governed callable boundary inside the WASM module.

### Required Fields

- `export`
- `request_schema`
- `result_schema`

### Shape

```json
{
  "export": "run_agent",
  "request_schema": "runtime_request",
  "result_schema": "runtime_result"
}
```

### Rules

- `export` identifies the primary exported function name.
- `request_schema` and `result_schema` describe the governed Traverse boundary, not an ad hoc agent-private shape.

## 4. Agent Module Record

Represents the primary WASM module artifact for one packaged agent.

### Required Fields

- `path`
- `digest`
- `abi`
- `runtime`

### Shape

```json
{
  "path": "artifacts/interpret-expedition-intent-agent.wasm",
  "digest": "sha256:agent-module...",
  "abi": "wasm32-wasip1",
  "runtime": "wasmtime"
}
```

### Rules

- `path` must be relative to the package root.
- `digest` is part of the governed-content immutability check.
- `runtime` identifies the expected compatible Wasm host family for this slice.

## 5. Agent Surface Binding

Represents one approved Traverse surface the agent may invoke or expose.

### Required Fields

- `binding_kind`
- `id`
- `version`
- `mode`

### Enum Values

`binding_kind`:

- `capability`
- `workflow`
- `mcp_surface`

`mode`:

- `invoke`
- `expose`

### Shape

```json
{
  "binding_kind": "capability",
  "id": "expedition.planning.interpret-expedition-intent",
  "version": "1.0.0",
  "mode": "invoke"
}
```

### Rules

- `capability` and `workflow` bindings must resolve to approved governed artifacts.
- `mcp_surface` may be declared only as future-facing exposure intent in this slice; it does not authorize a standalone transport contract.
- No undeclared surface may be used during execution.

## 6. Agent Runtime Constraint

Represents the declared execution eligibility requirements for the packaged agent.

### Required Fields

- `placement`
- `required_host_features`
- `max_input_bytes`
- `network_access`

### Enum Values

`placement`:

- `local`

`network_access`:

- `forbidden`
- `governed_host_only`

### Shape

```json
{
  "placement": "local",
  "required_host_features": [
    "traverse_runtime_v1"
  ],
  "max_input_bytes": 65536,
  "network_access": "forbidden"
}
```

### Rules

- `placement` is limited to `local` in this slice.
- `required_host_features` must remain explicit and machine-readable.
- `network_access` constrains host capability assumptions; it does not replace surface binding declarations.

## 7. Agent Package Validation Evidence

Represents machine-readable validation output for one packaged agent.

### Required Fields

- `kind`
- `schema_version`
- `governing_spec`
- `status`
- `id`
- `version`
- `validated_at`
- `checks`
- `violations`

### Shape

```json
{
  "kind": "ai_agent_package_validation",
  "schema_version": "1.0.0",
  "governing_spec": "017-ai-agent-packaging",
  "status": "passed",
  "id": "expedition.planning.interpret-expedition-intent-agent",
  "version": "1.0.0",
  "validated_at": "2026-04-01T00:00:00Z",
  "checks": [
    "manifest_shape_valid",
    "module_present",
    "module_digest_matches",
    "surface_bindings_declared",
    "runtime_constraints_valid"
  ],
  "violations": []
}
```

### Rules

- `status` values:
  - `passed`
  - `failed`
- `violations` entries must identify the failed rule and enough detail to explain package rejection.

## 8. Agent Execution Evidence

Represents machine-readable evidence connecting one runtime execution to one packaged agent.

### Required Fields

- `kind`
- `schema_version`
- `governing_spec`
- `agent_id`
- `agent_version`
- `request_id`
- `execution_id`
- `module_digest`
- `invoked_bindings`
- `status`
- `recorded_at`

### Shape

```json
{
  "kind": "ai_agent_execution_evidence",
  "schema_version": "1.0.0",
  "governing_spec": "017-ai-agent-packaging",
  "agent_id": "expedition.planning.interpret-expedition-intent-agent",
  "agent_version": "1.0.0",
  "request_id": "req_20260401_0001",
  "execution_id": "exec_20260401_0001",
  "module_digest": "sha256:agent-module...",
  "invoked_bindings": [
    {
      "binding_kind": "capability",
      "id": "expedition.planning.interpret-expedition-intent",
      "version": "1.0.0"
    }
  ],
  "status": "completed",
  "recorded_at": "2026-04-01T00:00:00Z"
}
```

### Rules

- `status` values:
  - `completed`
  - `error`
- `invoked_bindings` must list only declared `surface_bindings` actually used during the execution.
- This record complements runtime traces; it does not replace them.

## 9. Package Inspection Result

Represents a stable machine-readable inspection output for CLI and future UI/MCP consumers.

### Required Fields

- `status`
- `manifest`
- `validation`

### Shape

```json
{
  "status": "valid",
  "manifest": {
    "id": "expedition.planning.interpret-expedition-intent-agent",
    "version": "1.0.0",
    "agent_kind": "planner_assistant"
  },
  "validation": {
    "status": "passed"
  }
}
```

### Rules

- `status` values:
  - `valid`
  - `invalid`
- Inspection output must not depend on executing the agent.

## 10. Implementation Notes

- This slice intentionally keeps agent identity separate from capability and workflow identity.
- Package manifests may live under `examples/agents/` in early examples, but the manifest model must remain portable beyond that path.
- Future MCP, browser, and remote-placement slices may extend discovery and execution surfaces, but they must not break the governed package manifest defined here.

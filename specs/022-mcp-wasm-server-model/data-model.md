# Data Model: MCP WASM Server Model

## Purpose

This document defines the implementation-tight artifacts for the `022-mcp-wasm-server-model` slice.

It governs the first dedicated Traverse MCP server package, its supported host model, its core and convenience operations, and its runtime-authoritative execution mapping.

## 1. MCP Server Package Record

Represents one dedicated Traverse MCP server package.

### Required Fields

- `server_id`
- `package_kind`
- `supported_host_modes`
- `runtime_authority`
- `exposed_operation_sets`
- `supported_artifact_classes`

### Shape

```json
{
  "server_id": "traverse.mcp.server.v0",
  "package_kind": "dedicated_traverse_mcp_server",
  "supported_host_modes": [
    "stdio"
  ],
  "runtime_authority": "traverse_runtime",
  "exposed_operation_sets": [
    "core",
    "convenience"
  ],
  "supported_artifact_classes": [
    "capability_contract",
    "workflow_artifact",
    "wasm_agent_package"
  ]
}
```

### Rules

- `supported_host_modes` for the first slice must contain `stdio`.
- `runtime_authority` must resolve to Traverse runtime authority; no alternate execution authority is valid in v0.1.

## 2. MCP Core Operation Record

Represents one Traverse-native MCP operation.

### Required Fields

- `operation_id`
- `surface_kind`
- `subject_kind`
- `runtime_mapping`
- `returns`

### Shape

```json
{
  "operation_id": "run_workflow",
  "surface_kind": "core",
  "subject_kind": "workflow_backed_capability",
  "runtime_mapping": "translate_to_runtime_request_and_execute",
  "returns": "machine_readable_terminal_result"
}
```

### Rules

- `surface_kind` must be `core`.
- `subject_kind` must be one of:
  - `capability`
  - `workflow`
  - `workflow_backed_capability`
  - `trace`
  - `terminal_result`
- `runtime_mapping` must remain explicit; direct business-logic execution is invalid.

## 3. MCP Convenience Operation Record

Represents one generic workflow-oriented convenience operation.

### Required Fields

- `operation_id`
- `surface_kind`
- `entrypoint_subject`
- `delegates_to`
- `returns`

### Shape

```json
{
  "operation_id": "run_entrypoint",
  "surface_kind": "convenience",
  "entrypoint_subject": "governed_entrypoint_record",
  "delegates_to": [
    "describe_workflow",
    "run_workflow"
  ],
  "returns": "machine_readable_terminal_result"
}
```

### Rules

- `surface_kind` must be `convenience`.
- Convenience operations must delegate to one or more core operations.
- Convenience operations must remain generic and must not encode domain-specific scenario ids or chapter-specific terminology.

## 4. MCP Entrypoint Record

Represents one invocable governed entrypoint exposed through the dedicated server.

### Required Fields

- `entrypoint_id`
- `entrypoint_kind`
- `target_id`
- `target_version`
- `input_contract_ref`
- `output_contract_ref`

### Shape

```json
{
  "entrypoint_id": "expedition.planning.plan-expedition",
  "entrypoint_kind": "workflow_backed_capability",
  "target_id": "expedition.planning.plan-expedition",
  "target_version": "1.0.0",
  "input_contract_ref": "contracts/examples/expedition/capabilities/plan-expedition/contract.json#input",
  "output_contract_ref": "contracts/examples/expedition/capabilities/plan-expedition/contract.json#output"
}
```

### Rules

- `entrypoint_kind` must be one of:
  - `capability`
  - `workflow_backed_capability`
- Entry points must resolve to governed registered artifacts, not ad hoc demo handlers.

## 5. Runtime Delegation Record

Represents how one MCP operation delegates to Traverse runtime authority.

### Required Fields

- `delegation_id`
- `mcp_operation_id`
- `request_translation_rule`
- `validation_phase`
- `terminal_artifacts`

### Shape

```json
{
  "delegation_id": "run_workflow_to_runtime_request",
  "mcp_operation_id": "run_workflow",
  "request_translation_rule": "map_mcp_input_to_governed_runtime_request",
  "validation_phase": "pre_execution_runtime_validation",
  "terminal_artifacts": [
    "terminal_result",
    "trace_ref"
  ]
}
```

### Rules

- `validation_phase` must occur before execution and must remain runtime-owned.
- `terminal_artifacts` may include rendered forms, but the underlying artifacts must remain runtime-derived.

## 6. MCP Rendered Artifact Record

Represents one structured artifact returned by a rendering-oriented MCP operation.

### Required Fields

- `artifact_id`
- `source_kind`
- `render_kind`
- `machine_readable`

### Shape

```json
{
  "artifact_id": "execution_report",
  "source_kind": "trace_or_terminal_result",
  "render_kind": "structured_report",
  "machine_readable": true
}
```

### Rules

- `source_kind` must derive from governed runtime outputs, not private server-only state.
- `machine_readable` must be `true` for the first dedicated server slice.

## 7. WASM Exposure Record

Represents one governed WASM-hosted capability or agent exposed through the server.

### Required Fields

- `exposure_id`
- `manifest_ref`
- `governed_target_id`
- `binary_format`
- `exception_policy`

### Shape

```json
{
  "exposure_id": "expedition_intent_agent_stdio_exposure",
  "manifest_ref": "examples/agents/expedition-intent-agent/manifest.json",
  "governed_target_id": "expedition.planning.interpret-expedition-intent",
  "binary_format": "wasm",
  "exception_policy": "no_host_api_network_or_filesystem_exceptions"
}
```

### Rules

- `binary_format` for this slice must be `wasm`.
- `governed_target_id` must resolve to an approved governed capability or workflow-backed target.
- The server must not expose a WASM package whose manifest violates approved exception constraints.

# Data Model: Workflow Registry and Deterministic Traversal

## Purpose

This document defines the implementation-tight workflow artifacts for the `007-workflow-registry-traversal` slice.

It focuses on deterministic workflow registration, workflow discovery metadata, and workflow traversal evidence for `Foundation v0.1`.

## 1. Workflow Definition

Represents one authoritative workflow artifact.

### Required Fields

- `kind`
- `schema_version`
- `id`
- `name`
- `version`
- `lifecycle`
- `owner`
- `summary`
- `inputs`
- `outputs`
- `nodes`
- `edges`
- `start_node`
- `terminal_nodes`
- `tags`
- `governing_spec`

### Shape

```json
{
  "kind": "workflow_definition",
  "schema_version": "1.0.0",
  "id": "content.comments.publish-comment",
  "name": "publish-comment",
  "version": "1.0.0",
  "lifecycle": "active",
  "owner": {
    "team": "comments",
    "contact": "comments@example.com"
  },
  "summary": "Create, validate, and publish a comment deterministically.",
  "inputs": {
    "schema": {
      "type": "object"
    }
  },
  "outputs": {
    "schema": {
      "type": "object"
    }
  },
  "nodes": [],
  "edges": [],
  "start_node": "create_draft",
  "terminal_nodes": [
    "persist_comment"
  ],
  "tags": [
    "comments",
    "foundation"
  ],
  "governing_spec": "007-workflow-registry-traversal"
}
```

### Rules

- `kind` must equal `workflow_definition`
- `schema_version` must equal `1.0.0`
- `version` must be valid semver
- `start_node` must identify exactly one declared node
- `terminal_nodes` must not be empty

## 2. Workflow Owner

Represents workflow ownership metadata.

### Required Fields

- `team`
- `contact`

### Rules

- both fields must be non-empty

## 3. Workflow Node

Represents one workflow step backed by a capability contract.

### Required Fields

- `node_id`
- `capability_id`
- `capability_version`
- `input`
- `output`

### Shape

```json
{
  "node_id": "create_draft",
  "capability_id": "content.comments.create-comment-draft",
  "capability_version": "1.0.0",
  "input": {
    "from_workflow_input": [
      "comment_text",
      "resource_id"
    ]
  },
  "output": {
    "to_workflow_state": [
      "draft_id"
    ]
  }
}
```

### Rules

- `node_id` values must be unique within one workflow definition
- `capability_id` + `capability_version` must resolve to one registered capability

## 4. Workflow Edge

Represents one deterministic transition between nodes.

### Required Fields

- `edge_id`
- `from`
- `to`
- `trigger`

### Optional Fields

- `event`

### Shape

```json
{
  "edge_id": "draft_to_validate",
  "from": "create_draft",
  "to": "validate_comment",
  "trigger": "direct"
}
```

### Event Shape

```json
{
  "event_id": "content.comments.draft-created",
  "version": "1.0.0"
}
```

### Rules

- `trigger` values:
  - `direct`
  - `event`
- `event` is required only when `trigger = event`
- `from` and `to` must reference declared node ids

## 5. Deterministic Traversal Rules

Traversal order for this slice is:

1. enter `start_node`
2. execute the current node
3. collect all edges whose `from` matches the current node
4. if there is one valid direct edge, take it
5. if there is one valid event edge whose required event was emitted, take it
6. if there is no valid next edge and the current node is terminal, complete successfully
7. otherwise fail traversal explicitly

### Rules

- multiple valid next edges are not allowed in this slice
- deterministic traversal must fail rather than guess
- cycles are invalid for `v0.1`

## 6. Workflow Registry Record

Represents stored workflow registration metadata.

### Required Fields

- `scope`
- `id`
- `version`
- `lifecycle`
- `owner`
- `workflow_path`
- `workflow_digest`
- `registered_at`
- `governing_spec`
- `validator_version`
- `evidence`

### Rules

- uniqueness is per `(scope, id, version)`
- published workflow versions are immutable within their scope
- `scope` values:
  - `public`
  - `private`

## 7. Workflow Discovery Index Entry

Represents workflow metadata exposed for discovery.

### Required Fields

- `scope`
- `id`
- `version`
- `lifecycle`
- `owner`
- `summary`
- `tags`
- `participating_capabilities`
- `events_used`
- `start_node`
- `terminal_nodes`
- `registered_at`

### Rules

- `participating_capabilities` must be a deduplicated list of capability ids
- `events_used` must be a deduplicated list of `event_id@version`

## 8. Workflow Registration Evidence

Represents workflow registration validation output.

### Required Fields

- `evidence_id`
- `workflow_id`
- `workflow_version`
- `scope`
- `governing_spec`
- `validator_version`
- `produced_at`
- `result`

### Enum Values

`result`:

- `passed`

## 9. Workflow Validation Error

Represents one workflow registration or traversal validation failure.

### Required Fields

- `code`
- `message`
- `path`
- `severity`

### Error Codes

- `missing_required_field`
- `invalid_literal`
- `invalid_semver`
- `duplicate_item`
- `missing_reference`
- `invalid_start_node`
- `invalid_terminal_node`
- `invalid_edge_reference`
- `invalid_event_edge`
- `deterministic_cycle_not_allowed`
- `immutable_version_conflict`

### Severity Values

- `error`

## 10. Workflow Traversal Request

Represents one deterministic workflow invocation artifact.

### Required Fields

- `kind`
- `schema_version`
- `request_id`
- `workflow_id`
- `workflow_version`
- `scope`
- `input`
- `governing_spec`

### Rules

- `kind` must equal `workflow_execution_request`
- `scope` values:
  - `public_only`
  - `prefer_private`

## 11. Workflow Traversal Step Record

Represents one visited node during traversal.

### Required Fields

- `step_index`
- `node_id`
- `capability_id`
- `capability_version`
- `status`

### Enum Values

`status`:

- `entered`
- `completed`
- `failed`

## 12. Workflow Traversal Edge Record

Represents one traversed edge.

### Required Fields

- `edge_id`
- `from`
- `to`
- `trigger`

### Optional Fields

- `event`

## 13. Workflow Traversal Evidence

Represents the structured traversal artifact for one workflow run.

### Required Fields

- `kind`
- `schema_version`
- `trace_id`
- `request_id`
- `workflow_id`
- `workflow_version`
- `governing_spec`
- `visited_nodes`
- `traversed_edges`
- `emitted_events`
- `result`

### Result Shape

- `status`
- `failure_reason`

### Result Status Values

- `completed`
- `error`

### Failure Reasons

- `workflow_not_found`
- `workflow_invalid`
- `ambiguous_next_edge`
- `missing_required_event`
- `terminal_node_not_reached`
- `step_execution_failed`

## 14. Workflow-backed Capability Linkage

Represents the relationship between a composed capability and its workflow implementation.

### Required Fields

- `capability_id`
- `capability_version`
- `workflow_id`
- `workflow_version`

### Rules

- a workflow-backed capability must point to one registered workflow definition
- capability semver and workflow semver remain separate artifacts
- compatibility checks may consider both the capability contract and the linked workflow definition

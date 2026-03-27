# Cogolo Planning Board

This document is the local planning view for active work and mirrors the current backlog in GitHub Project 1.

Status meanings:

- `Ready`: can be implemented now under an approved governing spec
- `Needs Spec`: should not be implemented yet because the governing slice is not implementation-tight enough
- `Needs Enrico`: blocked on product, governance, or prioritization input from Enrico

## In Progress

### `Ready`

- `002-capability-contracts`
  - scope: capability contract parsing, normalization, validation, immutability, and evidence
  - status: merged in PR [#5](https://github.com/enricopiovesan/cogolo/pull/5)
  - notes: protected at `100%` line coverage

## Next Core Tasks

### `Ready`

- Event contract spec slice
  - issue: [#6](https://github.com/enricopiovesan/cogolo/issues/6)
  - suggested id: `003-event-contracts`
  - outcome: event artifact shape, lifecycle, ownership, versioning, publisher/subscriber metadata, validation rules

- Spec-alignment CI gate design
  - issue: [#7](https://github.com/enricopiovesan/cogolo/issues/7)
  - outcome: first deterministic check that maps implementation slices to governing spec ids and fails when required spec artifacts are missing or unapproved

### `Needs Spec`

- Capability registry implementation
  - issue: [#8](https://github.com/enricopiovesan/cogolo/issues/8)
  - target area: `crates/cogolo-registry`
  - missing first: dedicated registry slice for file layout, duplicate rules, indexing behavior, and evidence handling

- Runtime request and execution model
  - issue: [#9](https://github.com/enricopiovesan/cogolo/issues/9)
  - target area: `crates/cogolo-runtime`
  - missing first: dedicated runtime execution slice covering request schema, local WASM execution boundary, ambiguity behavior, and trace shape

- Workflow registry and deterministic traversal
  - issue: [#10](https://github.com/enricopiovesan/cogolo/issues/10)
  - target area: `crates/cogolo-registry`, `crates/cogolo-runtime`
  - missing first: dedicated workflow spec slice

- Event-driven composition
  - target area: `crates/cogolo-runtime`
  - missing first: event contract slice plus runtime event-flow slice

## Product and Architecture Backlog

### `Needs Spec`

- Metadata graph model
- Placement abstraction contract model beyond local execution
- Runtime state machine implementation slice
- Trace artifact implementation slice
- MCP surface spec
- Browser runtime subscription surface

### `Needs Enrico`

- Prioritized first five real capabilities for `v0.1`
  - issue: [#11](https://github.com/enricopiovesan/cogolo/issues/11)
  - we have examples and direction, but not a final approved set

- First canonical demo workflow
  - issue: [#12](https://github.com/enricopiovesan/cogolo/issues/12)
  - recommended target: one comment-flow style workflow using at least three capabilities

- Contract lifecycle publication policy
  - issue: [#13](https://github.com/enricopiovesan/cogolo/issues/13)
  - we have lifecycle states, but not yet a full approval/publication workflow decision for when `draft` becomes publishable

- Project-management policy
  - issue: [#14](https://github.com/enricopiovesan/cogolo/issues/14)
  - decide whether every spec slice becomes:
    - a GitHub issue
    - a project item only
    - both

## Recommended Sequence

1. Review and merge `003-event-contracts`
2. Design the spec-alignment CI gate
3. Write registry slice for capability and event registration
4. Implement `cogolo-registry`
5. Write runtime request/execution slice
6. Implement local runtime execution and trace skeleton

## Project 1 Sync

This planning board is mirrored into:

- [GitHub Project 1](https://github.com/users/enricopiovesan/projects/1/)

Recommended project fields:

- `Status`
- `Spec Status`
- `Owner`
- `Area`
- `Needs Enrico`

Recommended starter items:

- `002-capability-contracts`
- `003-event-contracts`
- `capability-registry`
- `runtime-execution-model`
- `workflow-definition-slice`
- `spec-alignment-ci-gate`

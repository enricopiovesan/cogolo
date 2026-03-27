# Traverse Planning Board

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

- `006-runtime-request-execution`
  - scope: runtime request parsing, deterministic local execution, ambiguity handling, state events, and runtime trace output
  - status: merged in PR [#20](https://github.com/enricopiovesan/cogolo/pull/20)
  - notes: protected at `100%` line coverage

- `005-capability-registry`
  - scope: capability registry storage, immutable publication, overlay lookup, compatibility checks, and discovery index
  - status: merged in PR [#18](https://github.com/enricopiovesan/cogolo/pull/18)
  - notes: protected at `100%` line coverage

## Next Core Tasks

### `Ready`

- Event contract spec slice
  - issue: [#6](https://github.com/enricopiovesan/cogolo/issues/6)
  - suggested id: `003-event-contracts`
  - outcome: event artifact shape, lifecycle, ownership, versioning, publisher/subscriber metadata, validation rules
  - status: merged in PR [#15](https://github.com/enricopiovesan/cogolo/pull/15)

- Spec-alignment CI gate design
  - issue: [#7](https://github.com/enricopiovesan/cogolo/issues/7)
  - outcome: first deterministic check that maps implementation slices to governing spec ids and fails when required spec artifacts are missing or unapproved
  - status: merged in PR [#16](https://github.com/enricopiovesan/cogolo/pull/16)

- Workflow registry and deterministic traversal
  - issue: [#10](https://github.com/enricopiovesan/cogolo/issues/10)
  - target area: `crates/traverse-registry`, `crates/traverse-runtime`
  - status: spec drafting in progress under `007-workflow-registry-traversal`
  - outcome: deterministic workflow artifact shape, workflow registry metadata, traversal rules, workflow-backed composed capability semantics

### `Needs Spec`

- Event-driven composition
  - target area: `crates/traverse-runtime`
  - missing first: event contract slice plus runtime event-flow slice

## Product and Architecture Backlog

### `Ready`

- Expedition example capability set and canonical workflow spec
  - issue: [#30](https://github.com/enricopiovesan/Traverse/issues/30)
  - target area: `specs/008-expedition-example-domain`
  - outcome: implementation-governing example domain for the first five capabilities, canonical workflow, and event/output shapes

### `Needs Spec`

- Metadata graph model
- Placement abstraction contract model beyond local execution
- Runtime state machine implementation slice
- Trace artifact implementation slice
- MCP surface spec
- Browser runtime subscription surface

### `Resolved`

- Prioritized first five real capabilities for `v0.1`
  - issue: [#11](https://github.com/enricopiovesan/Traverse/issues/11)
  - resolved direction: expedition-planning example domain

- First canonical demo workflow
  - issue: [#12](https://github.com/enricopiovesan/Traverse/issues/12)
  - resolved direction: `plan-expedition`

- Contract lifecycle publication policy
  - issue: [#13](https://github.com/enricopiovesan/Traverse/issues/13)
  - resolved direction: CI-gated plus manual approval before publication

- Project-management policy
  - issue: [#14](https://github.com/enricopiovesan/Traverse/issues/14)
  - resolved direction: issue + project item + PR for all meaningful slices

## Recommended Sequence

1. Review and merge `007-workflow-registry-traversal`
2. Implement workflow registry storage and deterministic traversal
3. Write the event-driven composition slice for runtime event flow
4. Implement event-driven workflow execution on top of the workflow slice
5. Lock the first five capabilities and the first canonical workflow
6. Move into browser-facing runtime subscription work

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

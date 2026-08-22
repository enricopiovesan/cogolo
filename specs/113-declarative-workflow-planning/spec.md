# Declarative Workflow Planning — P0 of Governed Runtime Workflow Composition

**Status**: Approved
**Canonical governing ID**: `113-declarative-workflow-planning`
**Version**: 0.1.0
**Implements phase**: P0 of `108-governed-runtime-workflow-composition` (precedes the P1 proposal-submission surface)
**Input**: Decision 59 (`docs/decision-log.md`); traverse #1098; registry #305.

## Purpose

Define a deterministic planner that, given a structured target and the
declared `consumes`/`emits` metadata of published capabilities, derives one
or more complete candidate workflow proposals — each already shaped to
satisfy `109-runtime-workflow-proposals` FR-002/FR-004/FR-011 (explicit
capability IDs/versions, explicit edges, explicit field-level JSON-path
mappings) — for a caller to review and submit through the existing P1 MCP
surface. This spec governs what generates a proposal's *content*; it does
not change how proposals are validated, authorized, or executed, which
remain entirely owned by `109`–`112`.

## Requirements

- **FR-001**: The planner MUST accept a structured target (a declared event
  type or capability ID the caller needs produced) and a starting fact set.
  It MUST NOT accept or interpret open-ended natural-language goals, and
  MUST NOT require or embed a hosted language model, matching FR-001 of
  spec `109`.
- **FR-002**: The planner MUST derive candidate chains only from capabilities
  whose contract declares non-empty `consumes`/`emits` linkage to the target
  and to each other. A capability with no declared linkage MUST NOT be
  selected, even if its name or namespace suggests a plausible fit.
- **FR-003**: When more than one published capability's declared `emits`
  could satisfy the same downstream `consumes` need, the planner MUST NOT
  pick one automatically. It MUST enumerate each resulting complete
  candidate plan separately and return the full set to the caller.
- **FR-004**: The planner MUST bound its search: at most 5 complete
  candidate plans, each at most 8 nodes deep, per planning call. A call
  that would exceed either bound MUST return the bounded subset it found
  plus a stable `plan_search_truncated` indicator, never fail silently or
  search unbounded.
- **FR-005**: Each returned candidate plan MUST include a best-effort,
  per-edge field-level JSON-path mapping derived from the source
  capability's `outputs.schema` and the target capability's `inputs.schema`.
  A candidate plan MUST be flagged `mapping_unconfirmed` until a human or
  caller-side reviewer approves it; this flag MUST NOT be cleared by the
  planner itself.
- **FR-006**: A candidate plan MUST be structurally submittable as-is to the
  `109-runtime-workflow-proposals` MCP surface (identical capability-id/
  version, edge, and mapping shape) once its `mapping_unconfirmed` flag is
  cleared by review. The planner MUST NOT submit or execute a plan itself.
- **FR-007**: The planner MUST be exposed as a public MCP tool alongside the
  existing P1 proposal tools (e.g. `workflow.plan`), discoverable without
  special credentials, per FR-001 of spec `109`.
- **FR-008**: Planning MUST be a pure, read-only operation over the current
  registry/manifest snapshot. It MUST NOT mutate the registry, the workflow
  catalog, or any manifest, and MUST NOT persist a candidate plan as a
  reusable workflow.

## Acceptance scenarios

1. A caller requests a plan targeting an event only one published capability
   chain can produce; the planner returns exactly one candidate plan with
   field mappings flagged `mapping_unconfirmed`.
2. A caller requests a plan where two published capabilities both emit a
   compatible event for the same downstream need; the planner returns two
   complete candidate plans, one per choice, rather than picking one.
3. A caller requests a plan for a target no published capability declares
   `emits` linkage to; the planner returns zero candidates rather than
   falling back to a name/namespace guess.
4. A candidate plan's field mapping is reviewed and corrected by a human
   before submission; the corrected mapping, not the planner's original
   guess, is what gets submitted to and validated by the P1 surface.
5. A goal whose valid chain space exceeds the bound returns the bounded
   subset plus `plan_search_truncated: true`, never an unbounded result or
   a silent failure.

## Out of scope

Natural-language goal interpretation, embedding a hosted planner model,
automatic proposal submission or execution, ambiguity auto-resolution,
unbounded plan search, registry or workflow-catalog mutation, and any
change to how `109`–`112` validate, authorize, or execute a submitted
proposal.

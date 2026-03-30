# Data Model: Traverse Expedition Example Artifacts

## Purpose

This data model defines the concrete artifact identities and example payload relationships for the first expedition example contracts and workflow.

It is intentionally implementation-tight so future example contracts and workflow artifacts can be authored directly from it.

## Namespace

All first expedition example artifacts use:

```json
"expedition.planning"
```

## Example Capability Contract Identities

```json
[
  "expedition.planning.capture-expedition-objective",
  "expedition.planning.interpret-expedition-intent",
  "expedition.planning.assess-conditions-summary",
  "expedition.planning.validate-team-readiness",
  "expedition.planning.assemble-expedition-plan"
]
```

## Example Event Contract Identities

```json
[
  "expedition.planning.expedition-objective-captured",
  "expedition.planning.expedition-intent-interpreted",
  "expedition.planning.conditions-summary-assessed",
  "expedition.planning.team-readiness-validated",
  "expedition.planning.expedition-plan-assembled"
]
```

## Capability-to-Event Mapping

```json
{
  "expedition.planning.capture-expedition-objective": "expedition.planning.expedition-objective-captured",
  "expedition.planning.interpret-expedition-intent": "expedition.planning.expedition-intent-interpreted",
  "expedition.planning.assess-conditions-summary": "expedition.planning.conditions-summary-assessed",
  "expedition.planning.validate-team-readiness": "expedition.planning.team-readiness-validated",
  "expedition.planning.assemble-expedition-plan": "expedition.planning.expedition-plan-assembled"
}
```

## Example Workflow Identity

```json
{
  "workflow_id": "expedition.planning.plan-expedition",
  "workflow_version": "1.0.0"
}
```

## Workflow-backed Capability Identity

```json
{
  "capability_id": "expedition.planning.plan-expedition",
  "implementation_kind": "workflow",
  "workflow_ref": {
    "workflow_id": "expedition.planning.plan-expedition",
    "workflow_version": "1.0.0"
  }
}
```

## Example Workflow Nodes

```json
[
  {
    "node_id": "capture_objective",
    "capability_id": "expedition.planning.capture-expedition-objective",
    "capability_version": "1.0.0"
  },
  {
    "node_id": "interpret_intent",
    "capability_id": "expedition.planning.interpret-expedition-intent",
    "capability_version": "1.0.0"
  },
  {
    "node_id": "assess_conditions",
    "capability_id": "expedition.planning.assess-conditions-summary",
    "capability_version": "1.0.0"
  },
  {
    "node_id": "validate_readiness",
    "capability_id": "expedition.planning.validate-team-readiness",
    "capability_version": "1.0.0"
  },
  {
    "node_id": "assemble_plan",
    "capability_id": "expedition.planning.assemble-expedition-plan",
    "capability_version": "1.0.0"
  }
]
```

## Example Workflow Edges

```json
[
  {
    "edge_id": "capture_to_interpret",
    "from": "capture_objective",
    "to": "interpret_intent",
    "trigger": "direct"
  },
  {
    "edge_id": "interpret_to_conditions",
    "from": "interpret_intent",
    "to": "assess_conditions",
    "trigger": "direct"
  },
  {
    "edge_id": "conditions_to_readiness",
    "from": "assess_conditions",
    "to": "validate_readiness",
    "trigger": "direct"
  },
  {
    "edge_id": "readiness_to_plan",
    "from": "validate_readiness",
    "to": "assemble_plan",
    "trigger": "direct"
  }
]
```

## Workflow Start and Terminal Nodes

```json
{
  "start_node": "capture_objective",
  "terminal_nodes": [
    "assemble_plan"
  ]
}
```

## Example Artifact Semantics

### 1. `capture-expedition-objective`

Produces:

```json
{
  "objective_id": "expedition-objective:<uuid>",
  "destination": "Mount Temple",
  "target_window": {
    "start": "2026-04-10T06:00:00Z",
    "end": "2026-04-10T20:00:00Z"
  },
  "preferences": {
    "style": "alpine_push",
    "risk_tolerance": "moderate",
    "priority": "safety_margin"
  },
  "notes": "Prefer an early start and conservative turnaround time."
}
```

### 2. `interpret-expedition-intent`

Produces:

```json
{
  "intent_id": "expedition-intent:<uuid>",
  "objective_id": "expedition-objective:<uuid>",
  "route_preferences": [
    "early-start",
    "low-exposure-if-possible"
  ],
  "constraints": [
    "same-day-return",
    "conservative-turnaround"
  ],
  "assumptions": [
    "weather-input-will-be-provided-separately"
  ],
  "confidence": 0.86
}
```

### 3. `assess-conditions-summary`

Produces:

```json
{
  "conditions_summary_id": "conditions-summary:<uuid>",
  "objective_id": "expedition-objective:<uuid>",
  "overall_rating": "caution",
  "key_findings": [
    "weather window is usable until early afternoon",
    "route condition is variable near the upper section"
  ],
  "blocking_concerns": [
    "increasing wind after midday"
  ]
}
```

### 4. `validate-team-readiness`

Produces:

```json
{
  "readiness_result_id": "team-readiness:<uuid>",
  "objective_id": "expedition-objective:<uuid>",
  "status": "caution",
  "reasons": [
    "one participant has limited experience on comparable terrain"
  ],
  "required_actions": [
    "confirm turnaround decision authority",
    "recheck required equipment before departure"
  ]
}
```

### 5. `assemble-expedition-plan`

Produces:

```json
{
  "plan_id": "expedition-plan:<uuid>",
  "objective_id": "expedition-objective:<uuid>",
  "status": "ready",
  "recommended_route_style": "conservative-alpine-push",
  "key_steps": [
    "depart before sunrise",
    "reassess winds at mid-route checkpoint",
    "apply conservative turnaround time"
  ],
  "constraints": [
    "same-day-return",
    "weather-window-before-midday-shift"
  ],
  "readiness_notes": [
    "team may proceed with caution after readiness actions are confirmed"
  ],
  "summary": "Proceed with a conservative same-day ascent plan under a limited morning weather window."
}
```

## Example Workflow Definition Skeleton

```json
{
  "kind": "workflow_definition",
  "schema_version": "1.0.0",
  "id": "expedition.planning.plan-expedition",
  "name": "plan-expedition",
  "version": "1.0.0",
  "lifecycle": "active",
  "owner": {
    "team": "traverse-core",
    "contact": "enrico.piovesan10@gmail.com"
  },
  "summary": "Capture, interpret, validate, and assemble an expedition plan deterministically.",
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
  "start_node": "capture_objective",
  "terminal_nodes": [
    "assemble_plan"
  ],
  "tags": [
    "expedition",
    "planning",
    "example-domain"
  ],
  "governing_spec": "007-workflow-registry-traversal"
}
```

## Notes

- future example contracts should use these ids directly unless a new governing spec changes them
- the example workflow remains direct-edge deterministic in this slice
- future event-driven examples may layer on top of these same governed event ids

# Data Model: Traverse Expedition Example Domain

## Purpose

This data model makes the expedition example domain implementation-tight enough for future example contracts, workflow artifacts, and demo surfaces.

It does not define the full generic Traverse platform schema. It defines the example-domain shapes that later contracts and workflows should use consistently.

## Capability Set

The first five example capabilities are:

1. `capture-expedition-objective`
2. `interpret-expedition-intent`
3. `assess-conditions-summary`
4. `validate-team-readiness`
5. `assemble-expedition-plan`

## Capability Roles

### `capture-expedition-objective`

Role:
- deterministic entry capability

Input shape:

```json
{
  "destination": "string",
  "target_window": {
    "start": "RFC3339 timestamp or date string",
    "end": "RFC3339 timestamp or date string"
  },
  "preferences": {
    "style": "day_trip | overnight | alpine_push | flexible",
    "risk_tolerance": "low | moderate | high",
    "priority": "summit | scenery | training | safety_margin"
  },
  "notes": "string"
}
```

Output shape:

```json
{
  "objective_id": "string",
  "destination": "string",
  "target_window": {
    "start": "string",
    "end": "string"
  },
  "preferences": {
    "style": "string",
    "risk_tolerance": "string",
    "priority": "string"
  },
  "notes": "string"
}
```

Emits:
- `expedition-objective-captured`

### `interpret-expedition-intent`

Role:
- AI-assisted interpretation capability

Input shape:

```json
{
  "objective": {
    "objective_id": "string",
    "destination": "string",
    "target_window": {
      "start": "string",
      "end": "string"
    },
    "preferences": {
      "style": "string",
      "risk_tolerance": "string",
      "priority": "string"
    },
    "notes": "string"
  },
  "free_form_intent": "string"
}
```

Output shape:

```json
{
  "intent_id": "string",
  "objective_id": "string",
  "route_preferences": [
    "string"
  ],
  "constraints": [
    "string"
  ],
  "assumptions": [
    "string"
  ],
  "confidence": 0.0
}
```

Notes:
- this capability may use AI
- output remains advisory/structured input for later deterministic steps

Emits:
- `expedition-intent-interpreted`

### `assess-conditions-summary`

Role:
- deterministic context assembly capability

Input shape:

```json
{
  "objective": {
    "objective_id": "string"
  },
  "intent": {
    "intent_id": "string"
  },
  "conditions_inputs": {
    "weather_summary": "string",
    "route_status": "string",
    "hazard_notes": [
      "string"
    ]
  }
}
```

Output shape:

```json
{
  "conditions_summary_id": "string",
  "objective_id": "string",
  "overall_rating": "favorable | caution | unfavorable",
  "key_findings": [
    "string"
  ],
  "blocking_concerns": [
    "string"
  ]
}
```

Emits:
- `conditions-summary-assessed`

### `validate-team-readiness`

Role:
- deterministic guard capability

Input shape:

```json
{
  "objective": {
    "objective_id": "string"
  },
  "conditions_summary": {
    "conditions_summary_id": "string"
  },
  "team_profile": {
    "members": [
      {
        "name": "string",
        "experience_level": "novice | intermediate | advanced",
        "roles": [
          "string"
        ]
      }
    ],
    "equipment_status": {
      "required_items_ready": true,
      "missing_items": [
        "string"
      ]
    }
  }
}
```

Output shape:

```json
{
  "readiness_result_id": "string",
  "objective_id": "string",
  "status": "ready | caution | blocked",
  "reasons": [
    "string"
  ],
  "required_actions": [
    "string"
  ]
}
```

Emits:
- `team-readiness-validated`

### `assemble-expedition-plan`

Role:
- deterministic composed outcome capability

Input shape:

```json
{
  "objective": {
    "objective_id": "string"
  },
  "intent": {
    "intent_id": "string"
  },
  "conditions_summary": {
    "conditions_summary_id": "string"
  },
  "readiness_result": {
    "readiness_result_id": "string"
  }
}
```

Output shape:

```json
{
  "plan_id": "string",
  "objective_id": "string",
  "status": "drafted | ready | blocked",
  "recommended_route_style": "string",
  "key_steps": [
    "string"
  ],
  "constraints": [
    "string"
  ],
  "readiness_notes": [
    "string"
  ],
  "summary": "string"
}
```

Emits:
- `expedition-plan-assembled`

## Canonical Workflow

Workflow id:

```json
"plan-expedition"
```

Deterministic node order:

```json
[
  "capture-expedition-objective",
  "interpret-expedition-intent",
  "assess-conditions-summary",
  "validate-team-readiness",
  "assemble-expedition-plan"
]
```

Start node:

```json
"capture-expedition-objective"
```

Terminal node:

```json
"assemble-expedition-plan"
```

## Example Event Contracts

Minimum event ids:

```json
[
  "expedition-objective-captured",
  "expedition-intent-interpreted",
  "conditions-summary-assessed",
  "team-readiness-validated",
  "expedition-plan-assembled"
]
```

Example event payload envelope:

```json
{
  "event_id": "string",
  "subject_id": "string",
  "capability_id": "string",
  "version": "string",
  "occurred_at": "RFC3339 timestamp",
  "payload": {}
}
```

## AI Boundary Rules

The expedition example domain applies these explicit rules:

- `interpret-expedition-intent` is the only AI-assisted capability in the first five.
- AI-generated output must be transformed into structured contract-conforming data.
- downstream deterministic capabilities must not assume AI output is authoritative simply because it exists.
- readiness validation and final plan assembly remain deterministic and governed.

## Composed Capability Direction

Future workflow-backed composed capability:

```json
{
  "capability_id": "plan-expedition",
  "implementation_kind": "workflow",
  "workflow_ref": {
    "workflow_id": "plan-expedition",
    "workflow_version": "1.0.0"
  }
}
```

## Notes for Future Implementation

- example contracts should use these names exactly unless a new governing spec changes them
- browser, Android, and macOS demos can all use the same canonical workflow language
- future MCP and UI demos should subscribe to the expedition event set rather than inventing alternate example terms

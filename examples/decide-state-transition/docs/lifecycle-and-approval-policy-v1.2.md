# Lifecycle & Approval Policy v1.2 (TOML-derived)

**Status:** authored for Traverse implementation  
**policy_version:** `toml-derived-1.2.0`  
**Source of truth:** `decide-state-transition-v1.1.0.toml` use cases (HP-01…HP-06, UP-01…UP-08)  
**Note:** The TOML referenced `docs/lifecycle-and-approval-policy-v1.1.md`, which was never provided. This document supplies the missing tables with **explicit defaults** so the capability can be completed and tested. Change a default → bump `policy_version`.

## Defaults (named, intentional)

| Knob | Value | Used by |
|------|-------|---------|
| `expense.auto_approve_max` | `100` (same currency units as `context.amount`) | HP-01, HP-02, UP-03 |
| `order.cancel_deadline` | ISO-8601 string in `context.cancel_deadline`; open iff `context.now <= cancel_deadline` (lexicographic, Zulu timestamps) | HP-05, UP-04 |
| Dual approval roles | `legal` + `finance` (both required, `logic: all`) | UP-08 |
| Ticket escalate flag | `context.priority == "critical"` | HP-06 |
| Ticket close block | `context.has_open_children == true` | UP-05 |

## Entity lifecycles (legal edges)

### `expense`

| From | To | Who / condition | Decision |
|------|----|-----------------|----------|
| `draft` | `submitted` | any role; `amount` missing | `requires_additional_info` / `MISSING_AMOUNT` (UP-03) |
| `draft` | `submitted` | any role; `amount < 100` | `allowed` / `AUTO_APPROVED` (HP-01) |
| `draft` | `submitted` | any role; `amount >= 100` | `requires_approval` / finance (HP-02 path) |
| `submitted` | `approved` | actor has `finance` **or** (`manager` **and** `finance`) | `allowed` (HP-03 when both) |
| `submitted` | `approved` | actor has only `manager` | `requires_approval` / still needs finance (HP-02) |
| `submitted` | `approved` | actor lacks manager/finance | `denied` / `INSUFFICIENT_ROLE` (UP-02) |
| `submitted` | `approved` | dual `legal`+`finance` required (when `context.requires_dual_approval == true`) and only one collected | `requires_approval` listing missing (UP-08) |
| other | other | — | `denied` / `ILLEGAL_TRANSITION` |

### `order`

| From | To | Condition | Decision |
|------|----|-----------|----------|
| `placed` | `cancelled` | `now <= cancel_deadline` | `allowed` / `CANCEL_WITHIN_WINDOW` (HP-05) |
| `placed` | `cancelled` | `now > cancel_deadline` | `denied` / `CANCEL_WINDOW_EXPIRED` (UP-04) |
| `fulfilled` | `draft` | — | `denied` / `ILLEGAL_TRANSITION` (UP-01) |
| other | other | — | `denied` / `ILLEGAL_TRANSITION` or `UNKNOWN_STATE` |

### `ticket`

| From | To | Condition | Decision |
|------|----|-----------|----------|
| `open` | `escalated` | `priority == critical` | `allowed` / `PRIORITY_ESCALATION` (HP-06) |
| `open` | `escalated` | priority not critical | `denied` / `PRIORITY_NOT_CRITICAL` |
| `open` | `closed` | `has_open_children == true` | `denied` / `HAS_OPEN_CHILDREN` (UP-05) |
| `open` | `closed` | no open children | `allowed` / `TICKET_CLOSED` |
| other | other | — | `denied` / `ILLEGAL_TRANSITION` |

## Cross-cutting

| Rule | Decision | Use case |
|------|----------|----------|
| `actor.roles` empty / missing | `denied` / `ACTOR_HAS_NO_ROLES` | UP-06 |
| Unknown `entity_type` | `denied` / `UNKNOWN_ENTITY_TYPE` | UP-07 |
| Unknown current/proposed state for a known entity | `denied` / `UNKNOWN_STATE` or `ILLEGAL_TRANSITION` | UP-07 |
| `mode == query` or proposed omitted/equal current | return `next_legal_states` for actor; decision `denied` + `QUERY_ONLY` | HP-04 |

## Query: next legal states (HP-04)

Returned lists are filtered by actor roles where applicable.

- **expense / draft:** `["submitted"]` if actor has any role  
- **expense / submitted:** `["approved"]` if actor has `manager` or `finance`  
- **order / placed:** `["cancelled"]` if any role (window checked at decide time)  
- **ticket / open:** `["escalated","closed"]` if any role  

## Versioning

- Contract/package version: `1.2.0` (capability I/O unchanged; behavior expanded).  
- Registry: publish new immutable `1.2.0`; leave `1.1.0` probe as historical.

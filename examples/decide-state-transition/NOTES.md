# decide-state-transition

Governed example for `platform.decide-state-transition@1.2.0`.

Policy: `toml-derived-1.2.0` — authored from the product TOML HP/UP stories with explicit defaults in [`docs/lifecycle-and-approval-policy-v1.2.md`](docs/lifecycle-and-approval-policy-v1.2.md).

## Run

```bash
bash scripts/ci/decide_state_transition_example_smoke.sh
```

## Coverage

| ID | Fixture | Expected |
|----|---------|----------|
| HP-01 | `hp01-low-value-allow.json` | `allowed` / `AUTO_APPROVED` |
| HP-02 | `hp02-manager-needs-finance.json` | `requires_approval` / finance |
| HP-03 | `hp03-finance-manager-approve.json` | `allowed` / `APPROVED_BY_FINANCE` |
| HP-04 | `hp04-query-next-states.json` | `QUERY_ONLY` + `next_legal_states` |
| HP-05 | `hp05-cancel-within-window.json` | `allowed` / `CANCEL_WITHIN_WINDOW` |
| HP-06 | `hp06-priority-escalate.json` | `allowed` / `PRIORITY_ESCALATION` |
| UP-01 | `up01-illegal-jump-deny.json` | `denied` / `ILLEGAL_TRANSITION` |
| UP-02 | `up02-insufficient-role.json` | `denied` / `INSUFFICIENT_ROLE` |
| UP-03 | `up03-missing-amount.json` | `requires_additional_info` / `MISSING_AMOUNT` |
| UP-04 | `up04-cancel-window-closed.json` | `denied` / `CANCEL_WINDOW_EXPIRED` |
| UP-05 | `up05-open-children-block.json` | `denied` / `HAS_OPEN_CHILDREN` |
| UP-06 | `up06-no-roles.json` | `denied` / `ACTOR_HAS_NO_ROLES` |
| UP-07 | `up07-unknown-entity.json` | `denied` / `UNKNOWN_ENTITY_TYPE` |
| UP-08 | `up08-partial-approvals.json` | `requires_approval` / `PARTIAL_APPROVALS` |

Registry: publish `1.2.0` after this package lands (immutable; leave probe `1.1.0` in place).

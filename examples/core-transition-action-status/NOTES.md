# core.transition-action-status

First Loop package capability adapted for the Traverse registry publication path
([#1026](https://github.com/traverse-framework/traverse/issues/1026)).

Pure, configurable status state machine. Policy (`allowed_transitions`,
`owner_only`) is supplied at invocation — the guest never mutates storage.

## Product gap closed in this adaptation

Loop WF4 says this capability “now includes `snoozed`”, but the package’s
shipped `1.0.0` contract enum omitted it. The registry-ready contract and guest
include `snoozed` so follow-up workflows can compose without a silent schema hole.

## Registry publish gap closed in this adaptation

`capability publish` requires every `use_cases[].persona_ref` to resolve under
`registry/personas/<id>/<version>/persona.json`. The Loop package used free-form
refs (`owner`, `system`); this contract maps them to existing personas
(`meeting-organizer`, `runtime-engineer`).

## Run

```bash
bash scripts/ci/core_transition_action_status_example_smoke.sh
```

## Coverage

| ID | Fixture | Expected `reason_code` |
|----|---------|------------------------|
| UC-01 | `uc01-open-to-in-progress.json` | `ok` (`allowed: true`) |
| UC-02 | `uc02-done-to-open-illegal.json` | `illegal_transition` |
| UC-03 | `uc03-open-to-snoozed.json` | `ok` (`new_status: snoozed`) |
| UC-04 | `uc04-non-owner-denied.json` | `not_owner` |

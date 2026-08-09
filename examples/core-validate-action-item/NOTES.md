# core.validate-action-item

Second Loop package capability adapted for the Traverse registry publication path
([#1029](https://github.com/traverse-framework/traverse/issues/1029)).

Pure commitment gatekeeper: title/owner/due-date rules plus optional duplicate
detection against existing open items. Policy is supplied at invocation.

## Run

```bash
bash scripts/ci/core_validate_action_item_example_smoke.sh
```

## Coverage

| ID | Fixture | Expected `reason_code` |
|----|---------|------------------------|
| UC-01 | `uc01-valid-item.json` | `ok` |
| UC-02 | `uc02-past-due.json` | `validation_failed` (`past_due`) |
| UC-03 | `uc03-missing-owner.json` | `validation_failed` (`missing_owner`) |
| UC-04 | `uc04-duplicate.json` | `duplicate` |

# core.assign-ownership

Third Loop package capability adapted for registry publication
([#1031](https://github.com/traverse-framework/traverse/issues/1031)).

Resolves a suggested owner (id / email / name) against workspace members with
configurable fallback (`creator` | `unassigned` | `fail`).

## Run

```bash
bash scripts/ci/core_assign_ownership_example_smoke.sh
```

## Coverage

| ID | Fixture | Expected |
|----|---------|----------|
| UC-01 | `uc01-name-match.json` | `ok` / `name_match` → `user-ada` |
| UC-02 | `uc02-email-match.json` | `ok` / `email_match` → `user-bob` |
| UC-03 | `uc03-null-fallback-creator.json` | `ok` / `fallback_creator` → `user-carol` |
| UC-04 | `uc04-unresolved-fail.json` | `unresolved` |

# Developer journal — decide-state-transition

## 1.2.0 (TOML-derived matrix)

- Policy doc authored at `docs/lifecycle-and-approval-policy-v1.2.md` (TOML HP/UP + explicit defaults; original markdown never existed).
- Guest implements HP-01…HP-06 / UP-01…UP-08.
- `policy_version`: `toml-derived-1.2.0`.
- Smoke: `bash scripts/ci/decide_state_transition_example_smoke.sh`.

## 1.1.0 (probe)

Historical probe subset shipped to the public registry as `1.1.0` (`policy_version: probe-1.0.0`). Left immutable.

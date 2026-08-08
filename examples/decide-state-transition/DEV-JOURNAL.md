# Developer / LLM E2E journal — decide-state-transition

**Persona:** new Traverse capability author  
**Context:** product requirements for a deterministic lifecycle/approval decision capability  
**Intended tools:** traverse-app-builder skill + traverse-cli  
**Not in scope:** ingesting the requirements file as a Traverse format

## Current status (2026-08-07)

DX gaps found in the original probe are closed on main (`#986`–`#991`, Spec 100, Decision 54):

| Step | Command | Status |
|------|---------|--------|
| Create | `traverse-cli capability new <id>` | scaffolds real `capability_package` + `build-fixture.sh` |
| Build | `bash …/build-fixture.sh` | compiles WASM **and** writes `binary.expected_digest` |
| Inspect | `capability inspect` / `capability-package inspect` | wired |
| Execute | `capability-package execute` | real `capability_version` + full JSON output |

Locked smokes:

```bash
bash scripts/ci/capability_new_e2e_smoke.sh          # cold create path
bash scripts/ci/decide_state_transition_example_smoke.sh  # this package
```

## Original probe findings (historical)

These were true when the probe was first run; keep for archaeology only:

1. No first-class create CLI → **fixed** (`capability new`)
2. `capability inspect` advertised but unwired → **fixed**
3. Stale scaffolds (`component new`, `scripts/scaffold/new-capability.sh`) → **retired / redirected**
4. Execute hardcoded version + demo output allowlist → **fixed**
5. Manual digest paste loop → **fixed** (build-fixture auto-writes digest)
6. Full lifecycle policy tables still need an explicit policy source (not invented here)

## Remaining product gaps (not DX)

- Probe policy is `probe-1.0.0` (expense subset for HP-01 / high-value / illegal jump).
- Full TOML use-case matrix needs `docs/lifecycle-and-approval-policy-v1.1.md`.
- Not published to the public registry yet.

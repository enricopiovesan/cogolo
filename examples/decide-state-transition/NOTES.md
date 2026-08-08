# decide-state-transition E2E example

Starting artifact: `~/Downloads/decide-state-transition-v1.1.0.toml`

Governed local example for the capability authoring path (Spec 100). Probe policy only — not a full lifecycle policy engine.

## Run

```bash
# rebuild wasm + refresh digest, then inspect/execute fixtures
bash scripts/ci/decide_state_transition_example_smoke.sh
```

Or step by step:

```bash
bash examples/decide-state-transition/build-fixture.sh
cargo run -q -p traverse-cli-rs -- capability inspect \
  examples/decide-state-transition/contract.json
cargo run -q -p traverse-cli-rs -- capability-package inspect \
  examples/decide-state-transition/manifest.json
cargo run -q -p traverse-cli-rs -- capability-package execute \
  examples/decide-state-transition/manifest.json \
  examples/decide-state-transition/runtime-requests/hp01-low-value-allow.json
```

## Fixtures

| Request | Expected decision | Code |
|---------|-------------------|------|
| `runtime-requests/hp01-low-value-allow.json` | `allowed` | `AUTO_APPROVED` |
| `runtime-requests/hp-high-value-requires-approval.json` | `requires_approval` | `AMOUNT_EXCEEDS_LIMIT` |
| `runtime-requests/up-illegal-jump-deny.json` | `denied` | `ILLEGAL_TRANSITION` |

## Cold create path (umbrella DoD)

```bash
bash scripts/ci/capability_new_e2e_smoke.sh
```

## Still not product-complete

- Policy version is `probe-1.0.0` (tiny expense subset).
- Full TOML matrix needs `docs/lifecycle-and-approval-policy-v1.1.md`.
- Not published to the registry yet.

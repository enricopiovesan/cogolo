# Expedition Example Smoke Validation

Run the expedition artifact smoke path with:

```bash
bash scripts/ci/expedition_artifact_smoke.sh
```

What it validates:

- the six expected expedition capability contracts exist
- the five expected expedition event contracts exist
- the canonical `plan-expedition` workflow artifact exists
- the governed capability, event, and workflow ids match the approved expedition example specs

To verify the failure path without touching the checked-in artifacts:

```bash
tmpdir="$(mktemp -d)"
cp -R contracts workflows "$tmpdir/"
rm "$tmpdir/contracts/examples/expedition/events/expedition-plan-assembled/contract.json"
TRAVERSE_REPO_ROOT="$tmpdir" bash scripts/ci/expedition_artifact_smoke.sh
```

The command should fail with a missing-artifact message.

Run the expedition execution smoke path with:

```bash
bash scripts/ci/expedition_execution_smoke.sh
```

What it validates:

- the canonical expedition runtime request can execute `expedition.planning.plan-expedition`
- the output includes a completed workflow-backed planning result
- invalid expedition execution input fails deterministically

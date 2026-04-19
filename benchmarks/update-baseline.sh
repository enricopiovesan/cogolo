#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SUMMARY="$REPO_ROOT/benchmarks/results/summary.json"
BASELINE="$REPO_ROOT/benchmarks/results/baseline.json"

echo "This will update the benchmark baseline."
echo "Run 'bash benchmarks/run.sh' first to generate a fresh summary."
echo ""

if [[ ! -f "$SUMMARY" ]]; then
  echo "No summary found. Run: bash benchmarks/run.sh" >&2
  exit 1
fi

echo "Current summary:"
cat "$SUMMARY"
echo ""
read -r -p "Update baseline with these numbers? [y/N] " confirm
if [[ "$confirm" != "y" && "$confirm" != "Y" ]]; then
  echo "Aborted."
  exit 0
fi

python3 - "$SUMMARY" "$BASELINE" <<'PYEOF'
import json, sys
from datetime import datetime, timezone

summary = json.load(open(sys.argv[1]))
baseline = {
    "created_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "git_sha": summary.get("git_sha", "unknown"),
    "platform": summary.get("platform", "unknown"),
    "note": "Updated via benchmarks/update-baseline.sh",
    "cold_start_ms": {
        "mean": summary["cold_start_ms"]["mean"],
        "max": summary["cold_start_ms"]["max"]
    },
    "steady_state_ms": {
        "mean": summary["steady_state_ms"]["mean"],
        "max": summary["steady_state_ms"]["max"]
    }
}
with open(sys.argv[2], "w") as f:
    json.dump(baseline, f, indent=2)
    f.write("\n")
print(f"Baseline updated: {sys.argv[2]}")
PYEOF

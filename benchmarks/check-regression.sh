#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BASELINE="$REPO_ROOT/benchmarks/results/baseline.json"
SUMMARY="$REPO_ROOT/benchmarks/results/summary.json"
THRESHOLD="${TRAVERSE_BENCH_THRESHOLD_PCT:-15}"

if [[ ! -f "$BASELINE" ]]; then
  echo "ERROR: baseline not found at $BASELINE" >&2
  echo "Run: bash benchmarks/update-baseline.sh" >&2
  exit 1
fi

if [[ ! -f "$SUMMARY" ]]; then
  echo "ERROR: summary not found at $SUMMARY" >&2
  echo "Run: bash benchmarks/run.sh first" >&2
  exit 1
fi

python3 - "$BASELINE" "$SUMMARY" "$THRESHOLD" <<'PYEOF'
import json, sys

baseline = json.load(open(sys.argv[1]))
summary  = json.load(open(sys.argv[2]))
threshold = float(sys.argv[3]) / 100.0

failures = []

def check(metric, b_key, s_key):
    b = baseline[b_key]["mean"]
    s = summary[s_key]["mean"]
    pct = (s - b) / b if b > 0 else 0
    status = "PASS" if pct <= threshold else "FAIL"
    print(f"  {metric}: baseline={b}ms  current={s}ms  delta={pct*100:+.1f}%  [{status}]")
    if pct > threshold:
        failures.append(f"{metric} regressed by {pct*100:.1f}% (threshold {threshold*100:.0f}%)")

print("=== Benchmark Regression Check ===")
check("cold_start",   "cold_start_ms",   "cold_start_ms")
check("steady_state", "steady_state_ms", "steady_state_ms")

if failures:
    print("\nFAIL — regressions detected:")
    for f in failures:
        print(f"  - {f}")
    print("\nTo update the baseline after an intentional change:")
    print("  bash benchmarks/update-baseline.sh")
    sys.exit(1)
else:
    print("\nPASS — no regressions detected")
PYEOF

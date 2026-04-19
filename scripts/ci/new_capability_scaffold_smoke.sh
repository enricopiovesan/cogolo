#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="${TRAVERSE_REPO_ROOT:-$(pwd)}"
OUT="/tmp/scaffold-smoke-$$"

bash "$REPO_ROOT/scripts/scaffold/new-capability.sh" \
  --name smoke-test \
  --namespace ci.smoke \
  --output-dir "$OUT"

for f in contract.json Cargo.toml src/main.rs build-fixture.sh runtime-request.json; do
  [[ -f "$OUT/$f" ]] || { echo "MISSING: $OUT/$f" >&2; exit 1; }
done

ID=$(python3 -c "import json; d=json.load(open('$OUT/contract.json')); print(d['id'])")
[[ "$ID" == "ci.smoke.smoke-test" ]] || { echo "Wrong id: $ID" >&2; exit 1; }

echo "Scaffold smoke: PASS"
rm -rf "$OUT"

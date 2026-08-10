#!/usr/bin/env bash
# Spec 102 FR-007 / Decision 58: each use_cases[i] in examples/core-* packages
# must have a matching runtime-requests/ucNN-*.json fixture (1-based, zero-padded).

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

failures=0
checked=0

for contract in examples/core-*/contract.json; do
  [[ -f "$contract" ]] || continue
  package_dir="$(dirname "$contract")"
  checked=$((checked + 1))

  use_case_count="$(
    python3 - "$contract" <<'PY'
import json, sys
contract = json.load(open(sys.argv[1], encoding="utf-8"))
use_cases = contract.get("use_cases") or []
if not isinstance(use_cases, list):
    raise SystemExit("use_cases must be an array")
print(len(use_cases))
PY
  )"

  if [[ "$use_case_count" -eq 0 ]]; then
    echo "FAIL: $package_dir — use_cases missing or empty (spec 102 FR-004/FR-007)"
    failures=$((failures + 1))
    continue
  fi

  requests_dir="$package_dir/runtime-requests"
  if [[ ! -d "$requests_dir" ]]; then
    echo "FAIL: $package_dir — runtime-requests/ directory missing (spec 102 FR-007)"
    failures=$((failures + 1))
    continue
  fi

  for ((index = 1; index <= use_case_count; index++)); do
    prefix="$(printf 'uc%02d-' "$index")"
    matches=("$requests_dir"/"$prefix"*.json)
    if [[ ! -e "${matches[0]}" ]]; then
      echo "FAIL: $package_dir — use_cases[$((index - 1))] lacks runtime-requests/${prefix}*.json (spec 102 FR-007)"
      failures=$((failures + 1))
    fi
  done
done

if [[ "$checked" -eq 0 ]]; then
  echo "FAIL: no examples/core-*/contract.json packages found"
  exit 1
fi

if [[ "$failures" -ne 0 ]]; then
  echo "use_case smoke coverage check failed ($failures gap(s) across $checked package(s))"
  exit 1
fi

echo "use_case smoke coverage check passed ($checked examples/core-* package(s))"

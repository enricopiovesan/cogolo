#!/usr/bin/env bash
set -euo pipefail
repo_root="${TRAVERSE_REPO_ROOT:-$(pwd)}"
cd "$repo_root"
pkg="examples/core-evaluate-completion-quality"
cli=(cargo run -q -p traverse-cli-rs --)
fail() { printf 'FAIL: %s\n' "$1" >&2; exit 1; }
require_match() { grep -q "$2" <<<"$1" || fail "$3 (missing: $2)"; }
echo "==> build-fixture"; bash "$pkg/build-fixture.sh"
echo "==> capability inspect"
out="$("${cli[@]}" capability inspect "$pkg/contract.json")"; printf '%s\n' "$out"
require_match "$out" "id: core.evaluate-completion-quality" "id"
echo "==> wasm abi verify"
out="$("${cli[@]}" wasm abi verify "$pkg/artifacts/core-evaluate-completion-quality.wasm")"; printf '%s\n' "$out"
require_match "$out" "import whitelist passed" "abi"
echo "==> capability-package inspect"
out="$("${cli[@]}" capability-package inspect "$pkg/manifest.json")"; printf '%s\n' "$out"
require_match "$out" "package_id: core.evaluate-completion-quality-agent" "package"
assert_execute() {
  local request="$1" code="$2" label="$3"; shift 3
  echo "==> execute $label"
  local out; out="$("${cli[@]}" capability-package execute "$pkg/manifest.json" "$request")"
  printf '%s\n' "$out"
  require_match "$out" "status: completed" "$label status"
  require_match "$out" "\"reason_code\": \"$code\"" "$label code"
  while [[ $# -gt 0 ]]; do
    require_match "$out" "$1" "$label extra"
    shift
  done
}
assert_execute "$pkg/runtime-requests/uc01-pass-with-evidence.json" "ok" "UC-01" \
  '"verdict": "pass"' \
  '"quality_score": 1.0'
assert_execute "$pkg/runtime-requests/uc02-needs-evidence.json" "ok" "UC-02" \
  '"verdict": "needs_evidence"' \
  'missing_evidence'
echo "OK: core.evaluate-completion-quality 1.0.0 E2E smoke passed"

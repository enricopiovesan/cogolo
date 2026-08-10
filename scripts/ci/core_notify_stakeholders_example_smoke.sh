#!/usr/bin/env bash
set -euo pipefail
repo_root="${TRAVERSE_REPO_ROOT:-$(pwd)}"
cd "$repo_root"
pkg="examples/core-notify-stakeholders"
cli=(cargo run -q -p traverse-cli-rs --)
fail() { printf 'FAIL: %s\n' "$1" >&2; exit 1; }
require_match() { grep -q "$2" <<<"$1" || fail "$3 (missing: $2)"; }
echo "==> build-fixture"; bash "$pkg/build-fixture.sh"
echo "==> capability inspect"
out="$("${cli[@]}" capability inspect "$pkg/contract.json")"; printf '%s\n' "$out"
require_match "$out" "id: core.notify-stakeholders" "id"
echo "==> wasm abi verify"
out="$("${cli[@]}" wasm abi verify "$pkg/artifacts/core-notify-stakeholders.wasm")"; printf '%s\n' "$out"
require_match "$out" "import whitelist passed" "abi"
echo "==> capability-package inspect"
out="$("${cli[@]}" capability-package inspect "$pkg/manifest.json")"; printf '%s\n' "$out"
require_match "$out" "package_id: core.notify-stakeholders-agent" "package"
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
assert_execute "$pkg/runtime-requests/uc01-completed.json" "ok" "UC-01" \
  '"intent_count": 2' \
  'Action completed' \
  'user-carol'
assert_execute "$pkg/runtime-requests/uc02-empty.json" "nothing_to_notify" "UC-02" \
  '"intent_count": 0'
assert_execute "$pkg/runtime-requests/uc03-status-changed.json" "ok" "UC-03"   'Action updated'   '"intent_count": 1'
assert_execute "$pkg/runtime-requests/uc04-blocked.json" "ok" "UC-04"   'Action blocked'   '"intent_count": 1'
echo "OK: core.notify-stakeholders 1.0.1 E2E smoke passed"

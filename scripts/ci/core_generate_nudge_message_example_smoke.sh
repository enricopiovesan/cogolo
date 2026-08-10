#!/usr/bin/env bash
set -euo pipefail
repo_root="${TRAVERSE_REPO_ROOT:-$(pwd)}"
cd "$repo_root"
pkg="examples/core-generate-nudge-message"
cli=(cargo run -q -p traverse-cli-rs --)
fail() { printf 'FAIL: %s\n' "$1" >&2; exit 1; }
require_match() { grep -q "$2" <<<"$1" || fail "$3 (missing: $2)"; }
echo "==> build-fixture"; bash "$pkg/build-fixture.sh"
echo "==> capability inspect"
out="$("${cli[@]}" capability inspect "$pkg/contract.json")"; printf '%s\n' "$out"
require_match "$out" "id: core.generate-nudge-message" "id"
echo "==> wasm abi verify"
out="$("${cli[@]}" wasm abi verify "$pkg/artifacts/core-generate-nudge-message.wasm")"; printf '%s\n' "$out"
require_match "$out" "import whitelist passed" "abi"
echo "==> capability-package inspect"
out="$("${cli[@]}" capability-package inspect "$pkg/manifest.json")"; printf '%s\n' "$out"
require_match "$out" "package_id: core.generate-nudge-message-agent" "package"
assert_execute() {
  local request="$1" code="$2" label="$3" extra="${4:-}"
  echo "==> execute $label"
  local out; out="$("${cli[@]}" capability-package execute "$pkg/manifest.json" "$request")"
  printf '%s\n' "$out"
  require_match "$out" "status: completed" "$label status"
  require_match "$out" "\"reason_code\": \"$code\"" "$label code"
  [[ -z "$extra" ]] || require_match "$out" "$extra" "$label extra"
}
assert_execute "$pkg/runtime-requests/uc01-soft-friendly.json" "ok" "UC-01" 'Send the revised proposal'
assert_execute "$pkg/runtime-requests/uc02-escalate.json" "ok" "UC-02" 'Escalation'
assert_execute "$pkg/runtime-requests/uc03-direct-neutral.json" "ok" "UC-03" 'Please complete'
assert_execute "$pkg/runtime-requests/uc04-config-error.json" "config_error" "UC-04"
echo "OK: core.generate-nudge-message 1.0.1 E2E smoke passed"

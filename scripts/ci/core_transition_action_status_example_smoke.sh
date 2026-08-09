#!/usr/bin/env bash
# End-to-end smoke for examples/core-transition-action-status.

set -euo pipefail

repo_root="${TRAVERSE_REPO_ROOT:-$(pwd)}"
cd "$repo_root"

pkg="examples/core-transition-action-status"
cli=(cargo run -q -p traverse-cli-rs --)

fail() {
  printf 'FAIL: %s\n' "$1" >&2
  exit 1
}

require_match() {
  local haystack="$1"
  local needle="$2"
  local label="$3"
  grep -q "$needle" <<<"$haystack" || fail "$label (missing: $needle)"
}

echo "==> build-fixture"
bash "$pkg/build-fixture.sh"

echo "==> capability inspect"
contract_out="$("${cli[@]}" capability inspect "$pkg/contract.json")"
printf '%s\n' "$contract_out"
require_match "$contract_out" "id: core.transition-action-status" "contract inspect id"
require_match "$contract_out" "version: 1.0.0" "contract inspect version"

echo "==> wasm abi verify"
abi_out="$("${cli[@]}" wasm abi verify "$pkg/artifacts/core-transition-action-status.wasm")"
printf '%s\n' "$abi_out"
require_match "$abi_out" "import whitelist passed" "abi verify"

echo "==> capability-package inspect"
pkg_out="$("${cli[@]}" capability-package inspect "$pkg/manifest.json")"
printf '%s\n' "$pkg_out"
require_match "$pkg_out" "package_id: core.transition-action-status-agent" "package_id"
require_match "$pkg_out" "capability_version: 1.0.0" "capability_version"

assert_execute() {
  local request="$1"
  local allowed="$2"
  local code="$3"
  local label="$4"
  local extra="${5:-}"

  echo "==> execute $label"
  local out
  out="$("${cli[@]}" capability-package execute "$pkg/manifest.json" "$request")"
  printf '%s\n' "$out"
  require_match "$out" "status: completed" "$label status"
  require_match "$out" "capability_version: 1.0.0" "$label capability_version"
  require_match "$out" "\"allowed\": $allowed" "$label allowed"
  require_match "$out" "\"reason_code\": \"$code\"" "$label reason_code"
  if [[ -n "$extra" ]]; then
    require_match "$out" "$extra" "$label extra"
  fi
}

assert_execute "$pkg/runtime-requests/uc01-open-to-in-progress.json" "true" "ok" "UC-01" '"new_status": "in_progress"'
assert_execute "$pkg/runtime-requests/uc02-done-to-open-illegal.json" "false" "illegal_transition" "UC-02" '"new_status": "done"'
assert_execute "$pkg/runtime-requests/uc03-open-to-snoozed.json" "true" "ok" "UC-03" '"new_status": "snoozed"'
assert_execute "$pkg/runtime-requests/uc04-non-owner-denied.json" "false" "not_owner" "UC-04" '"new_status": "open"'

echo "OK: core.transition-action-status 1.0.0 E2E smoke passed"

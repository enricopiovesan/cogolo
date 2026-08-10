#!/usr/bin/env bash
# End-to-end smoke for examples/core-select-items-for-followup.

set -euo pipefail

repo_root="${TRAVERSE_REPO_ROOT:-$(pwd)}"
cd "$repo_root"

pkg="examples/core-select-items-for-followup"
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
require_match "$contract_out" "id: core.select-items-for-followup" "contract inspect id"
require_match "$contract_out" "version: 1.1.1" "contract inspect version"

echo "==> wasm abi verify"
abi_out="$("${cli[@]}" wasm abi verify "$pkg/artifacts/core-select-items-for-followup.wasm")"
printf '%s\n' "$abi_out"
require_match "$abi_out" "import whitelist passed" "abi verify"

echo "==> capability-package inspect"
pkg_out="$("${cli[@]}" capability-package inspect "$pkg/manifest.json")"
printf '%s\n' "$pkg_out"
require_match "$pkg_out" "package_id: core.select-items-for-followup-agent" "package_id"
require_match "$pkg_out" "capability_version: 1.1.1" "capability_version"

assert_execute() {
  local request="$1"
  local code="$2"
  local label="$3"
  local extra="${4:-}"

  echo "==> execute $label"
  local out
  out="$("${cli[@]}" capability-package execute "$pkg/manifest.json" "$request")"
  printf '%s\n' "$out"
  require_match "$out" "status: completed" "$label status"
  require_match "$out" "capability_version: 1.1.1" "$label capability_version"
  require_match "$out" "\"reason_code\": \"$code\"" "$label reason_code"
  if [[ -n "$extra" ]]; then
    require_match "$out" "$extra" "$label extra"
  fi
}

assert_execute "$pkg/runtime-requests/uc01-quiet-hours.json" "ok" "UC-01" '"reason": "quiet_hours"'
assert_execute "$pkg/runtime-requests/uc01-quiet-hours.json" "ok" "UC-01 selected empty" '"selected": \[\]'
assert_execute "$pkg/runtime-requests/uc02-escalate.json" "ok" "UC-02" '"intensity": "escalate"'
assert_execute "$pkg/runtime-requests/uc02-escalate.json" "ok" "UC-02 channel" '"recommended_channel": "manager"'
assert_execute "$pkg/runtime-requests/uc03-config-error.json" "config_error" "UC-03"

echo "OK: core.select-items-for-followup 1.1.1 E2E smoke passed"

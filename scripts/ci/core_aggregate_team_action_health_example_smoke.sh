#!/usr/bin/env bash
# End-to-end smoke for examples/core-aggregate-team-action-health.

set -euo pipefail

repo_root="${TRAVERSE_REPO_ROOT:-$(pwd)}"
cd "$repo_root"

pkg="examples/core-aggregate-team-action-health"
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
require_match "$contract_out" "id: core.aggregate-team-action-health" "contract inspect id"
require_match "$contract_out" "version: 1.0.0" "contract inspect version"

echo "==> wasm abi verify"
abi_out="$("${cli[@]}" wasm abi verify "$pkg/artifacts/core-aggregate-team-action-health.wasm")"
printf '%s\n' "$abi_out"
require_match "$abi_out" "import whitelist passed" "abi verify"

echo "==> capability-package inspect"
pkg_out="$("${cli[@]}" capability-package inspect "$pkg/manifest.json")"
printf '%s\n' "$pkg_out"
require_match "$pkg_out" "package_id: core.aggregate-team-action-health-agent" "package_id"
require_match "$pkg_out" "capability_version: 1.0.0" "capability_version"

assert_execute() {
  local request="$1"
  local label="$2"
  shift 2

  echo "==> execute $label"
  local out
  out="$("${cli[@]}" capability-package execute "$pkg/manifest.json" "$request")"
  printf '%s\n' "$out"
  require_match "$out" "status: completed" "$label status"
  require_match "$out" "capability_version: 1.0.0" "$label capability_version"
  require_match "$out" "\"reason_code\": \"ok\"" "$label reason_code"
  while [[ $# -gt 0 ]]; do
    require_match "$out" "$1" "$label extra"
    shift
  done
}

assert_execute "$pkg/runtime-requests/uc01-team-pulse.json" "UC-01" \
  '"total_open": 3' \
  '"overdue_count": 1' \
  '"on_track_pct": 66.6' \
  '"owner_id": "user-ada"' \
  '"open_count": 2' \
  '"ai-3"' \
  '"ai-1"'

echo "OK: core.aggregate-team-action-health 1.0.0 E2E smoke passed"

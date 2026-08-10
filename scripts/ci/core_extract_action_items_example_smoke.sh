#!/usr/bin/env bash
# End-to-end smoke for examples/core-extract-action-items (core.extract-action-items@1.2.0).

set -euo pipefail

repo_root="${TRAVERSE_REPO_ROOT:-$(pwd)}"
cd "$repo_root"

pkg="examples/core-extract-action-items"
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
require_match "$contract_out" "id: core.extract-action-items" "contract inspect id"
require_match "$contract_out" "version: 1.2.0" "contract inspect version"

echo "==> wasm abi verify"
abi_out="$("${cli[@]}" wasm abi verify "$pkg/artifacts/core-extract-action-items.wasm")"
printf '%s\n' "$abi_out"
require_match "$abi_out" "import whitelist passed" "abi verify"

echo "==> capability-package inspect"
pkg_out="$("${cli[@]}" capability-package inspect "$pkg/manifest.json")"
printf '%s\n' "$pkg_out"
require_match "$pkg_out" "package_id: core.extract-action-items-agent" "package_id"
require_match "$pkg_out" "capability_version: 1.2.0" "capability_version"

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
  require_match "$out" "capability_version: 1.2.0" "$label capability_version"
  require_match "$out" "\"reason_code\": \"$code\"" "$label reason_code"
  if [[ -n "$extra" ]]; then
    require_match "$out" "$extra" "$label extra"
  fi
}

echo "==> execute UC-01"
uc01_out="$("${cli[@]}" capability-package execute "$pkg/manifest.json" "$pkg/runtime-requests/uc01-extract-mixed.json")"
printf '%s\n' "$uc01_out"
require_match "$uc01_out" "status: completed" "UC-01 status"
require_match "$uc01_out" "capability_version: 1.2.0" "UC-01 capability_version"
require_match "$uc01_out" "\"reason_code\": \"ok\"" "UC-01 reason_code"
require_match "$uc01_out" '"title": "Send the revised proposal"' "UC-01 ada title"
require_match "$uc01_out" '"suggested_owner": "Ada Lovelace"' "UC-01 ada owner"
require_match "$uc01_out" '"suggested_due_date": "2026-08-09"' "UC-01 ada due"
require_match "$uc01_out" '"confidence": 0.93' "UC-01 ada confidence"
require_match "$uc01_out" '"title": "Review security notes"' "UC-01 bob title"
require_match "$uc01_out" '"suggested_owner": "Bob Smith"' "UC-01 bob owner"
require_match "$uc01_out" '"suggested_due_date": "2026-08-14"' "UC-01 bob due"
require_match "$uc01_out" '"confidence": 0.88' "UC-01 bob confidence"
require_match "$uc01_out" '"reason": "vague_language"' "UC-01 vague reason"
require_match "$uc01_out" '"confidence": 0.58' "UC-01 vague confidence"

assert_execute "$pkg/runtime-requests/uc02-no-actions.json" "no_action_items_found" "UC-02" '"needs_human_review": \[\]'

echo "OK: core.extract-action-items 1.2.0 E2E smoke passed"

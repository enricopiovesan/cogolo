#!/usr/bin/env bash
# End-to-end smoke for examples/core-validate-action-item (core.validate-action-item@1.1.0).

set -euo pipefail

repo_root="${TRAVERSE_REPO_ROOT:-$(pwd)}"
cd "$repo_root"

pkg="examples/core-validate-action-item"
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
require_match "$contract_out" "id: core.validate-action-item" "contract inspect id"
require_match "$contract_out" "version: 1.1.0" "contract inspect version"

echo "==> wasm abi verify"
abi_out="$("${cli[@]}" wasm abi verify "$pkg/artifacts/core-validate-action-item.wasm")"
printf '%s\n' "$abi_out"
require_match "$abi_out" "import whitelist passed" "abi verify"

echo "==> capability-package inspect"
pkg_out="$("${cli[@]}" capability-package inspect "$pkg/manifest.json")"
printf '%s\n' "$pkg_out"
require_match "$pkg_out" "package_id: core.validate-action-item-agent" "package_id"
require_match "$pkg_out" "capability_version: 1.1.0" "capability_version"

assert_execute() {
  local request="$1"
  local valid="$2"
  local code="$3"
  local label="$4"
  local extra="${5:-}"

  echo "==> execute $label"
  local out
  out="$("${cli[@]}" capability-package execute "$pkg/manifest.json" "$request")"
  printf '%s\n' "$out"
  require_match "$out" "status: completed" "$label status"
  require_match "$out" "capability_version: 1.1.0" "$label capability_version"
  require_match "$out" "\"valid\": $valid" "$label valid"
  require_match "$out" "\"reason_code\": \"$code\"" "$label reason_code"
  if [[ -n "$extra" ]]; then
    require_match "$out" "$extra" "$label extra"
  fi
}

assert_execute "$pkg/runtime-requests/uc01-valid-item.json" "true" "ok" "UC-01" '"title": "Send proposal"'
assert_execute "$pkg/runtime-requests/uc02-past-due.json" "false" "validation_failed" "UC-02" '"code": "past_due"'
assert_execute "$pkg/runtime-requests/uc03-missing-owner.json" "false" "validation_failed" "UC-03" '"code": "missing_owner"'
assert_execute "$pkg/runtime-requests/uc04-duplicate.json" "false" "duplicate" "UC-04" '"code": "duplicate"'
assert_execute "$pkg/runtime-requests/uc05-duplicate-title-only.json" "false" "duplicate" "UC-05" '"code": "duplicate"'
assert_execute "$pkg/runtime-requests/uc06-invalid-config.json" "false" "invalid_config" "UC-06"

echo "OK: core.validate-action-item 1.1.0 E2E smoke passed"

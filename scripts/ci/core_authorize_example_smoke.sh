#!/usr/bin/env bash
# End-to-end smoke for examples/core-authorize (core.authorize@1.1.1).

set -euo pipefail

repo_root="${TRAVERSE_REPO_ROOT:-$(pwd)}"
cd "$repo_root"

pkg="examples/core-authorize"
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
require_match "$contract_out" "id: core.authorize" "contract inspect id"
require_match "$contract_out" "version: 1.1.1" "contract inspect version"

echo "==> wasm abi verify"
abi_out="$("${cli[@]}" wasm abi verify "$pkg/artifacts/core-authorize.wasm")"
printf '%s\n' "$abi_out"
require_match "$abi_out" "import whitelist passed" "abi verify"

echo "==> capability-package inspect"
pkg_out="$("${cli[@]}" capability-package inspect "$pkg/manifest.json")"
printf '%s\n' "$pkg_out"
require_match "$pkg_out" "package_id: core.authorize-agent" "package_id"
require_match "$pkg_out" "capability_version: 1.1.1" "capability_version"

assert_execute() {
  local request="$1"
  local decision="$2"
  local code="$3"
  local label="$4"
  local extra="${5:-}"

  echo "==> execute $label"
  local out
  out="$("${cli[@]}" capability-package execute "$pkg/manifest.json" "$request")"
  printf '%s\n' "$out"
  require_match "$out" "status: completed" "$label status"
  require_match "$out" "capability_version: 1.1.1" "$label capability_version"
  require_match "$out" "\"decision\": \"$decision\"" "$label decision"
  require_match "$out" "\"reason_code\": \"$code\"" "$label reason_code"
  if [[ -n "$extra" ]]; then
    require_match "$out" "$extra" "$label extra"
  fi
}

assert_execute "$pkg/runtime-requests/uc01-admin-delete-allow.json" "allow" "matched_allow_rule" "UC-01"
assert_execute "$pkg/runtime-requests/uc02-tenant-isolation-deny.json" "deny" "matched_deny_rule" "UC-02" '"rule_id": "tenant-isolation"'
assert_execute "$pkg/runtime-requests/uc03-owner-update-allow.json" "allow" "matched_allow_rule" "UC-03" '"rule_id": "owner-update"'
assert_execute "$pkg/runtime-requests/uc04-suspended-deny.json" "deny" "matched_deny_rule" "UC-04" '"rule_id": "suspended-deny"'
assert_execute "$pkg/runtime-requests/uc05-obligations-allow.json" "allow" "matched_allow_rule" "UC-05" '"type": "require_mfa"'
assert_execute "$pkg/runtime-requests/uc06-no-match-deny.json" "deny" "no_matching_rule" "UC-06"
assert_execute "$pkg/runtime-requests/uc07-empty-policy-deny.json" "deny" "empty_or_invalid_policy" "UC-07"
assert_execute "$pkg/runtime-requests/uc08-break-glass-allow.json" "allow" "break_glass_override" "UC-08" '"break_glass_used": true'
assert_execute "$pkg/runtime-requests/uc09-invalid-principal-deny.json" "deny" "invalid_principal" "UC-09" '"policy_hash": null'
assert_execute "$pkg/runtime-requests/uc10-invalid-action-deny.json" "deny" "invalid_action" "UC-10"
assert_execute "$pkg/runtime-requests/uc11-invalid-resource-deny.json" "deny" "invalid_resource" "UC-11"

echo "OK: core.authorize 1.1.1 E2E smoke passed"

#!/usr/bin/env bash
# End-to-end smoke for examples/core-assign-ownership (core.assign-ownership@1.0.1).

set -euo pipefail

repo_root="${TRAVERSE_REPO_ROOT:-$(pwd)}"
cd "$repo_root"

pkg="examples/core-assign-ownership"
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
require_match "$contract_out" "id: core.assign-ownership" "contract inspect id"
require_match "$contract_out" "version: 1.0.1" "contract inspect version"

echo "==> wasm abi verify"
abi_out="$("${cli[@]}" wasm abi verify "$pkg/artifacts/core-assign-ownership.wasm")"
printf '%s\n' "$abi_out"
require_match "$abi_out" "import whitelist passed" "abi verify"

echo "==> capability-package inspect"
pkg_out="$("${cli[@]}" capability-package inspect "$pkg/manifest.json")"
printf '%s\n' "$pkg_out"
require_match "$pkg_out" "package_id: core.assign-ownership-agent" "package_id"
require_match "$pkg_out" "capability_version: 1.0.1" "capability_version"

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
  require_match "$out" "capability_version: 1.0.1" "$label capability_version"
  require_match "$out" "\"reason_code\": \"$code\"" "$label reason_code"
  if [[ -n "$extra" ]]; then
    require_match "$out" "$extra" "$label extra"
  fi
}

assert_execute "$pkg/runtime-requests/uc01-name-match.json" "ok" "UC-01" '"owner_id": "user-ada"'
assert_execute "$pkg/runtime-requests/uc02-email-match.json" "ok" "UC-02" '"owner_id": "user-bob"'
assert_execute "$pkg/runtime-requests/uc03-null-fallback-creator.json" "ok" "UC-03" '"owner_id": "user-carol"'
assert_execute "$pkg/runtime-requests/uc04-unresolved-fail.json" "unresolved" "UC-04" '"owner_id": null'
assert_execute "$pkg/runtime-requests/uc05-inactive-member.json" "inactive_member" "UC-05"
assert_execute "$pkg/runtime-requests/uc06-config-error.json" "config_error" "UC-06"
assert_execute "$pkg/runtime-requests/uc07-fallback-unassigned.json" "ok" "UC-07" '"resolution_method": "fallback_unassigned"'

echo "OK: core.assign-ownership 1.0.1 E2E smoke passed"

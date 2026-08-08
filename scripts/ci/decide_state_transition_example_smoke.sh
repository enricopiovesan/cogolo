#!/usr/bin/env bash
# End-to-end smoke for examples/decide-state-transition (policy toml-derived-1.2.0).

set -euo pipefail

repo_root="${TRAVERSE_REPO_ROOT:-$(pwd)}"
cd "$repo_root"

pkg="examples/decide-state-transition"
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
require_match "$contract_out" "id: platform.decide-state-transition" "contract inspect id"
require_match "$contract_out" "version: 1.2.0" "contract inspect version"

echo "==> wasm abi verify"
abi_out="$("${cli[@]}" wasm abi verify "$pkg/artifacts/decide-state-transition.wasm")"
printf '%s\n' "$abi_out"
require_match "$abi_out" "import whitelist passed" "abi verify"

echo "==> capability-package inspect"
pkg_out="$("${cli[@]}" capability-package inspect "$pkg/manifest.json")"
printf '%s\n' "$pkg_out"
require_match "$pkg_out" "package_id: platform.decide-state-transition-agent" "package_id"
require_match "$pkg_out" "capability_version: 1.2.0" "capability_version"

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
  require_match "$out" "capability_version: 1.2.0" "$label capability_version"
  require_match "$out" "\"decision\": \"$decision\"" "$label decision"
  require_match "$out" "\"code\": \"$code\"" "$label code"
  require_match "$out" "\"policy_version\": \"toml-derived-1.2.0\"" "$label policy_version"
  if [[ -n "$extra" ]]; then
    require_match "$out" "$extra" "$label extra"
  fi
}

assert_execute "$pkg/runtime-requests/hp01-low-value-allow.json" "allowed" "AUTO_APPROVED" "HP-01"
assert_execute "$pkg/runtime-requests/hp02-manager-needs-finance.json" "requires_approval" "AMOUNT_EXCEEDS_LIMIT" "HP-02"
assert_execute "$pkg/runtime-requests/hp03-finance-manager-approve.json" "allowed" "APPROVED_BY_FINANCE" "HP-03"
assert_execute "$pkg/runtime-requests/hp04-query-next-states.json" "denied" "QUERY_ONLY" "HP-04" '"submitted"'
assert_execute "$pkg/runtime-requests/hp05-cancel-within-window.json" "allowed" "CANCEL_WITHIN_WINDOW" "HP-05"
assert_execute "$pkg/runtime-requests/hp06-priority-escalate.json" "allowed" "PRIORITY_ESCALATION" "HP-06"
assert_execute "$pkg/runtime-requests/up01-illegal-jump-deny.json" "denied" "ILLEGAL_TRANSITION" "UP-01"
assert_execute "$pkg/runtime-requests/up02-insufficient-role.json" "denied" "INSUFFICIENT_ROLE" "UP-02"
assert_execute "$pkg/runtime-requests/up03-missing-amount.json" "requires_additional_info" "MISSING_AMOUNT" "UP-03"
assert_execute "$pkg/runtime-requests/up04-cancel-window-closed.json" "denied" "CANCEL_WINDOW_EXPIRED" "UP-04"
assert_execute "$pkg/runtime-requests/up05-open-children-block.json" "denied" "HAS_OPEN_CHILDREN" "UP-05"
assert_execute "$pkg/runtime-requests/up06-no-roles.json" "denied" "ACTOR_HAS_NO_ROLES" "UP-06"
assert_execute "$pkg/runtime-requests/up07-unknown-entity.json" "denied" "UNKNOWN_ENTITY_TYPE" "UP-07"
assert_execute "$pkg/runtime-requests/up08-partial-approvals.json" "requires_approval" "PARTIAL_APPROVALS" "UP-08" '"role": "finance"'
assert_execute "$pkg/runtime-requests/hp-high-value-requires-approval.json" "requires_approval" "AMOUNT_EXCEEDS_LIMIT" "high-value draft"

echo "OK: decide-state-transition 1.2.0 E2E smoke passed"

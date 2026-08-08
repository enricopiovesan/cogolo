#!/usr/bin/env bash
# End-to-end smoke for examples/decide-state-transition.
# Covers: build-fixture (auto-digest) → contract inspect → ABI verify →
# package inspect → three execute fixtures.

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
require_match "$contract_out" "version: 1.1.0" "contract inspect version"

echo "==> wasm abi verify"
abi_out="$("${cli[@]}" wasm abi verify "$pkg/artifacts/decide-state-transition.wasm")"
printf '%s\n' "$abi_out"
require_match "$abi_out" "import whitelist passed" "abi verify"

echo "==> capability-package inspect"
pkg_out="$("${cli[@]}" capability-package inspect "$pkg/manifest.json")"
printf '%s\n' "$pkg_out"
require_match "$pkg_out" "package_id: platform.decide-state-transition-agent" "package_id"
require_match "$pkg_out" "capability_id: platform.decide-state-transition" "capability_id"
require_match "$pkg_out" "capability_version: 1.1.0" "capability_version"

assert_execute() {
  local request="$1"
  local decision="$2"
  local code="$3"
  local label="$4"

  echo "==> execute $label"
  local out
  out="$("${cli[@]}" capability-package execute "$pkg/manifest.json" "$request")"
  printf '%s\n' "$out"
  require_match "$out" "status: completed" "$label status"
  require_match "$out" "capability_version: 1.1.0" "$label capability_version"
  require_match "$out" "\"decision\": \"$decision\"" "$label decision"
  require_match "$out" "\"code\": \"$code\"" "$label code"
}

assert_execute \
  "$pkg/runtime-requests/hp01-low-value-allow.json" \
  "allowed" \
  "AUTO_APPROVED" \
  "HP-01 low-value allow"

assert_execute \
  "$pkg/runtime-requests/hp-high-value-requires-approval.json" \
  "requires_approval" \
  "AMOUNT_EXCEEDS_LIMIT" \
  "high-value requires approval"

assert_execute \
  "$pkg/runtime-requests/up-illegal-jump-deny.json" \
  "denied" \
  "ILLEGAL_TRANSITION" \
  "illegal jump deny"

echo "OK: decide-state-transition E2E smoke passed"

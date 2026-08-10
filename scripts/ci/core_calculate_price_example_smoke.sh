#!/usr/bin/env bash
set -euo pipefail
repo_root="${TRAVERSE_REPO_ROOT:-$(pwd)}"
cd "$repo_root"
pkg="examples/core-calculate-price"
cli=(cargo run -q -p traverse-cli-rs --)
fail() { printf 'FAIL: %s\n' "$1" >&2; exit 1; }
require_match() { grep -q "$2" <<<"$1" || fail "$3 (missing: $2)"; }
echo "==> build-fixture"; bash "$pkg/build-fixture.sh"
echo "==> capability inspect"
out="$("${cli[@]}" capability inspect "$pkg/contract.json")"; printf '%s\n' "$out"
require_match "$out" "id: core.calculate-price" "id"
require_match "$out" "version: 1.0.1" "version"
echo "==> wasm abi verify"
out="$("${cli[@]}" wasm abi verify "$pkg/artifacts/core-calculate-price.wasm")"; printf '%s\n' "$out"
require_match "$out" "import whitelist passed" "abi"
echo "==> capability-package inspect"
out="$("${cli[@]}" capability-package inspect "$pkg/manifest.json")"; printf '%s\n' "$out"
require_match "$out" "package_id: core.calculate-price-agent" "package"
require_match "$out" "capability_version: 1.0.1" "capability_version"
assert_execute() {
  local request="$1" code="$2" label="$3"; shift 3
  echo "==> execute $label"
  local out; out="$("${cli[@]}" capability-package execute "$pkg/manifest.json" "$request")"
  printf '%s\n' "$out"
  require_match "$out" "status: completed" "$label status"
  require_match "$out" "\"reason_code\": \"$code\"" "$label code"
  while [[ $# -gt 0 ]]; do
    require_match "$out" "$1" "$label extra"
    shift
  done
}
assert_execute "$pkg/runtime-requests/uc01-percentage-discount-tax.json" "ok" "UC-01" '"net": 97.2'
assert_execute "$pkg/runtime-requests/uc02-fixed-discount.json" "ok" "UC-02" '"net": 94.0'
assert_execute "$pkg/runtime-requests/uc03-empty-rules.json" "ok" "UC-03" '"net": 50.0'
assert_execute "$pkg/runtime-requests/uc04-invalid-quantity.json" "invalid_quantity" "UC-04"
assert_execute "$pkg/runtime-requests/uc05-invalid-unit-price.json" "invalid_unit_price" "UC-05"
assert_execute "$pkg/runtime-requests/uc06-invalid-config.json" "invalid_config" "UC-06"
assert_execute "$pkg/runtime-requests/uc07-currency-mismatch.json" "currency_mismatch" "UC-07"
assert_execute "$pkg/runtime-requests/uc08-empty-cart.json" "empty_cart" "UC-08"
echo "OK: core.calculate-price 1.0.1 E2E smoke passed"

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
echo "==> wasm abi verify"
out="$("${cli[@]}" wasm abi verify "$pkg/artifacts/core-calculate-price.wasm")"; printf '%s\n' "$out"
require_match "$out" "import whitelist passed" "abi"
echo "==> capability-package inspect"
out="$("${cli[@]}" capability-package inspect "$pkg/manifest.json")"; printf '%s\n' "$out"
require_match "$out" "package_id: core.calculate-price-agent" "package"
assert_execute() {
  local request="$1" code="$2" label="$3" extra="${4:-}"
  echo "==> execute $label"
  local out; out="$("${cli[@]}" capability-package execute "$pkg/manifest.json" "$request")"
  printf '%s\n' "$out"
  require_match "$out" "status: completed" "$label status"
  require_match "$out" "\"reason_code\": \"$code\"" "$label code"
  [[ -z "$extra" ]] || require_match "$out" "$extra" "$label extra"
}
assert_execute "$pkg/runtime-requests/uc01-percentage-discount-tax.json" "ok" "UC-01" '"net": 97.2'
assert_execute "$pkg/runtime-requests/uc02-invalid-quantity.json" "invalid_quantity" "UC-02" 'quantity must be > 0'
echo "OK: core.calculate-price 1.0.0 E2E smoke passed"

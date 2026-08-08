#!/usr/bin/env bash
# Cold create path: capability new → build-fixture (auto-digest) → inspect → execute.
# Proves Spec 100 / umbrella #988 DoD without tribal skill knowledge.

set -euo pipefail

repo_root="${TRAVERSE_REPO_ROOT:-$(pwd)}"
cd "$repo_root"

tmp_root="$(mktemp -d "${TMPDIR:-/tmp}/traverse-capability-new-e2e.XXXXXX")"
cleanup() {
  rm -rf "$tmp_root"
}
trap cleanup EXIT

capability_id="ci.smoke.decide-create-path"
cli=(cargo run -q --manifest-path "$repo_root/Cargo.toml" -p traverse-cli-rs --)

echo "==> capability new (cwd=$tmp_root)"
(
  cd "$tmp_root"
  "${cli[@]}" capability new "$capability_id"
)

cap_dir="$tmp_root/capabilities/$capability_id"
manifest="$cap_dir/manifest.json"
request="$cap_dir/runtime-requests/decide-create-path.json"

test -f "$manifest"
test -f "$cap_dir/build-fixture.sh"
test -f "$request"

echo "==> build-fixture (auto-digest)"
bash "$cap_dir/build-fixture.sh"
test -f "$cap_dir/artifacts/decide-create-path.wasm"
grep -q '"expected_digest": "fnv1a64:' "$manifest"
grep -vq '"expected_digest": "fnv1a64:0000000000000000"' "$manifest"

echo "==> capability-package inspect"
inspect_out="$("${cli[@]}" capability-package inspect "$manifest")"
printf '%s\n' "$inspect_out"
grep -q "package_id: ${capability_id}" <<<"$inspect_out"
grep -q "capability_id: ${capability_id}" <<<"$inspect_out"
grep -q "binary_digest: fnv1a64:" <<<"$inspect_out"

echo "==> capability-package execute"
exec_out="$("${cli[@]}" capability-package execute "$manifest" "$request")"
printf '%s\n' "$exec_out"
grep -q "status: completed" <<<"$exec_out"
grep -q "capability_version: 1.0.0" <<<"$exec_out"
grep -q '"output_value": "replace with real capability output"' <<<"$exec_out"

echo "OK: capability new E2E smoke passed"

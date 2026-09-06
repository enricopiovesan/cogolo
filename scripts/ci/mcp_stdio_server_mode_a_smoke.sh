#!/usr/bin/env bash

# Spec 119 Mode A: prove the stdio server discovers and executes a verified
# public kit from host-supplied, digest-verified registry state — not the
# expedition catalog — and fails closed without prepared state.

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cache_dir="${repo_root}/crates/traverse-mcp/tests/fixtures/mode-a-cache"

stdout_log=$(mktemp)
stderr_log=$(mktemp)
absent_stdout_log=$(mktemp)
absent_stderr_log=$(mktemp)
absent_cache_dir=$(mktemp -d)

cleanup() {
  rm -f "${stdout_log}" "${stderr_log}" "${absent_stdout_log}" "${absent_stderr_log}"
  rm -rf "${absent_cache_dir}"
}
trap cleanup EXIT

if [[ ! -f "${cache_dir}/public-metadata/current.json" ]]; then
  echo "Missing verified kit fixture at ${cache_dir}." >&2
  echo "Regenerate with: cargo test -p traverse-mcp --lib -- --ignored --exact stdio_server::tests::mode_a::regenerate_committed_fixture" >&2
  exit 1
fi

kit_id="core.normalize-participants"
kit_version="1.1.0"
# The stdio protocol is one JSON command per line, so the inline request must be
# compact.
inline_request=$(python3 -c 'import json,sys; print(json.dumps(json.load(sys.stdin)))' \
  <"${repo_root}/examples/core-normalize-participants/runtime-requests/uc01-mixed-match.json")

printf '%s\n' \
  '{"command":"describe_server"}' \
  '{"command":"list_entrypoints"}' \
  "{\"command\":\"execute_entrypoint\",\"entrypoint_kind\":\"capability\",\"id\":\"${kit_id}\",\"version\":\"${kit_version}\",\"request\":${inline_request}}" \
  '{"command":"shutdown"}' \
  | TRAVERSE_MCP_REGISTRY_CACHE="${cache_dir}" cargo run -p traverse-mcp -- stdio \
      >"${stdout_log}" 2>"${stderr_log}"

grep -q '"mode":"verified_public"' "${stdout_log}"
grep -q '"governing_spec":"119-verified-registry-mcp-mode-a"' "${stdout_log}"
grep -q '"kind":"host_verified_public_registry"' "${stdout_log}"
grep -q "\"id\":\"${kit_id}\"" "${stdout_log}"
grep -q '"kind":"mcp_stdio_server_entrypoint_execution"' "${stdout_log}"
grep -q '"status":"completed"' "${stdout_log}"
grep -q '"request_source":"inline"' "${stdout_log}"
grep -q '"digest_matches_public_state":true' "${stdout_log}"

if grep -q 'expedition' "${stdout_log}"; then
  echo "Mode A output must not reference the expedition catalog." >&2
  exit 1
fi

# Fail closed: an empty cache directory is not prepared verified state.
set +e
printf '%s\n' '{"command":"describe_server"}' \
  | TRAVERSE_MCP_REGISTRY_CACHE="${absent_cache_dir}" cargo run -p traverse-mcp -- stdio \
      >"${absent_stdout_log}" 2>"${absent_stderr_log}"
absent_status=$?
set -e

if [[ ${absent_status} -eq 0 ]]; then
  echo "Expected Mode A to fail closed without prepared verified state." >&2
  exit 1
fi
grep -q '"code":"registry_sync_missing"' "${absent_stderr_log}"
test ! -s "${absent_stdout_log}"

echo "MCP stdio server Mode A smoke passed."

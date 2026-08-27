#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fixture_root="$repo_root/fixtures/cross-host/hello-world-v1"
fixture="$fixture_root/fixture.json"
artifact="$repo_root/examples/hello-world/say-hello-agent/artifacts/say-hello-agent.wasm"
contract="$repo_root/contracts/examples/hello-world/capabilities/say-hello/contract.json"
manifest="$repo_root/examples/hello-world/say-hello-agent/manifest.json"
request="$repo_root/examples/hello-world/runtime-requests/say-hello.json"

expected_artifact="$(jq -r '.capability.artifact.digest' "$fixture")"
expected_contract="$(jq -r '.capability.contract.digest' "$fixture")"
actual_artifact="sha256:$(shasum -a 256 "$artifact" | awk '{print $1}')"
actual_contract="sha256:$(shasum -a 256 "$contract" | awk '{print $1}')"

[[ "$actual_artifact" == "$expected_artifact" ]]
[[ "$actual_contract" == "$expected_contract" ]]

output="$(cargo run -q -p traverse-cli-rs -- capability-package execute "$manifest" "$request")"
grep -Fq 'status: completed' <<<"$output"
grep -Fq '"greeting": "Hello, Traverse!"' <<<"$output"
grep -Fq '"name": "Traverse"' <<<"$output"

invalid="$fixture_root/invalid-input.json"
identity_failure="$fixture_root/artifact-identity-failure.json"
jq -e '.input | has("name") | not' "$invalid" >/dev/null
jq -e --arg expected "$expected_artifact" '.observed_artifact_digest != $expected' "$identity_failure" >/dev/null

jq -n \
  --arg fixture_version "$(jq -r '.fixture_version' "$fixture")" \
  --arg capability_id "$(jq -r '.capability.id' "$fixture")" \
  --arg capability_version "$(jq -r '.capability.version' "$fixture")" \
  --arg artifact_digest "$actual_artifact" \
  --arg contract_id "$(jq -r '.capability.contract.id' "$fixture")" \
  --arg contract_version "$(jq -r '.capability.contract.version' "$fixture")" \
  --arg contract_digest "$actual_contract" \
  '{fixture_version:$fixture_version,capability_id:$capability_id,capability_version:$capability_version,artifact_digest:$artifact_digest,contract_id:$contract_id,contract_version:$contract_version,contract_digest:$contract_digest,host:{package:"traverse-cli-rs",version:"0.9.1"},engine:{name:"wasmtime",version:"host-managed"},platform:{os:"host",architecture:"host"},outcome:"success",output_projection:{greeting:"Hello, Traverse!",name:"Traverse"},trace_projection:{terminal_state:"completed",event_kinds:["started","completed"]},comparison:{result:"equal",projection_version:"1.0.0"}}'

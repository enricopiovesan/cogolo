#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
default_repo_root="$(cd "${script_dir}/../.." && pwd)"
repo_root="${TRAVERSE_REPO_ROOT:-${default_repo_root}}"
summary_file="${GITHUB_STEP_SUMMARY:-}"

write_summary() {
  if [[ -n "${summary_file}" ]]; then
    printf '%s\n' "$1" >> "${summary_file}"
  fi
}

fail() {
  echo "$1" >&2
  if [[ -n "${GITHUB_ACTIONS:-}" ]]; then
    printf '::error::%s\n' "$1"
  fi
  write_summary "$1"
  exit 1
}

capability_files=(
  "contracts/examples/expedition/capabilities/capture-expedition-objective/contract.json"
  "contracts/examples/expedition/capabilities/interpret-expedition-intent/contract.json"
  "contracts/examples/expedition/capabilities/assess-conditions-summary/contract.json"
  "contracts/examples/expedition/capabilities/validate-team-readiness/contract.json"
  "contracts/examples/expedition/capabilities/assemble-expedition-plan/contract.json"
  "contracts/examples/expedition/capabilities/plan-expedition/contract.json"
)

event_files=(
  "contracts/examples/expedition/events/expedition-objective-captured/contract.json"
  "contracts/examples/expedition/events/expedition-intent-interpreted/contract.json"
  "contracts/examples/expedition/events/conditions-summary-assessed/contract.json"
  "contracts/examples/expedition/events/team-readiness-validated/contract.json"
  "contracts/examples/expedition/events/expedition-plan-assembled/contract.json"
)

workflow_files=(
  "workflows/examples/expedition/plan-expedition/workflow.json"
)

expected_ids=(
  "expedition.planning.capture-expedition-objective"
  "expedition.planning.interpret-expedition-intent"
  "expedition.planning.assess-conditions-summary"
  "expedition.planning.validate-team-readiness"
  "expedition.planning.assemble-expedition-plan"
  "expedition.planning.plan-expedition"
  "expedition.planning.expedition-objective-captured"
  "expedition.planning.expedition-intent-interpreted"
  "expedition.planning.conditions-summary-assessed"
  "expedition.planning.team-readiness-validated"
  "expedition.planning.expedition-plan-assembled"
)

for relative_path in "${capability_files[@]}" "${event_files[@]}" "${workflow_files[@]}"; do
  if [[ ! -f "${repo_root}/${relative_path}" ]]; then
    fail "Missing required expedition artifact: ${relative_path}"
  fi
done

for expected_id in "${expected_ids[@]}"; do
  if ! rg -F "\"id\": \"${expected_id}\"" \
    "${repo_root}/contracts/examples/expedition" \
    "${repo_root}/workflows/examples/expedition" >/dev/null; then
    fail "Missing governed expedition id: ${expected_id}"
  fi
done

capability_count="$(find "${repo_root}/contracts/examples/expedition/capabilities" -name contract.json | wc -l | tr -d ' ')"
event_count="$(find "${repo_root}/contracts/examples/expedition/events" -name contract.json | wc -l | tr -d ' ')"
workflow_count="$(find "${repo_root}/workflows/examples/expedition" -name workflow.json | wc -l | tr -d ' ')"

if [[ "${capability_count}" != "6" ]]; then
  fail "Expected 6 expedition capability contracts, found ${capability_count}"
fi

if [[ "${event_count}" != "5" ]]; then
  fail "Expected 5 expedition event contracts, found ${event_count}"
fi

if [[ "${workflow_count}" != "1" ]]; then
  fail "Expected 1 expedition workflow artifact, found ${workflow_count}"
fi

write_summary "Expedition artifact smoke check passed."
echo "Expedition artifact smoke check passed."

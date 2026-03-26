#!/usr/bin/env bash

set -euo pipefail

readonly TARGETS_FILE="ci/coverage-targets.txt"

if [[ ! -f "${TARGETS_FILE}" ]]; then
  echo "Missing coverage target configuration: ${TARGETS_FILE}" >&2
  exit 1
fi

targets=()
while IFS= read -r line; do
  targets+=("${line}")
done < <(grep -Ev '^\s*(#|$)' "${TARGETS_FILE}" || true)

if [[ ${#targets[@]} -eq 0 ]]; then
  echo "No protected coverage targets configured yet. Coverage gate passes by design."
  exit 0
fi

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
  echo "cargo-llvm-cov is required for the coverage gate." >&2
  exit 1
fi

failed=0

for entry in "${targets[@]}"; do
  crate_name="$(awk '{print $1}' <<<"${entry}")"
  minimum_percent="$(awk '{print $2}' <<<"${entry}")"

  if [[ -z "${crate_name}" || -z "${minimum_percent}" ]]; then
    echo "Invalid coverage target entry: ${entry}" >&2
    failed=1
    continue
  fi

  echo "Measuring line coverage for ${crate_name} with threshold ${minimum_percent}%"
  coverage_output="$(cargo llvm-cov --package "${crate_name}" --summary-only)"
  line_percent="$(
    awk '/^TOTAL/ {gsub(/%/, "", $NF); print $NF}' <<<"${coverage_output}" | tail -n 1
  )"

  if [[ -z "${line_percent}" ]]; then
    echo "Unable to parse line coverage for ${crate_name}" >&2
    echo "${coverage_output}" >&2
    failed=1
    continue
  fi

  printf 'Line coverage for %s: %s%%\n' "${crate_name}" "${line_percent}"

  if ! awk -v actual="${line_percent}" -v required="${minimum_percent}" \
    'BEGIN { exit (actual + 0 >= required + 0) ? 0 : 1 }'; then
    echo "Coverage gate failed for ${crate_name}: ${line_percent}% < ${minimum_percent}%." >&2
    failed=1
  fi
done

if [[ ${failed} -ne 0 ]]; then
  exit 1
fi

echo "Coverage gate passed."

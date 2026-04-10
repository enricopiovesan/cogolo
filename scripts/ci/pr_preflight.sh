#!/usr/bin/env bash

set -euo pipefail

body_file="${1:-}"

if [[ -z "${body_file}" ]]; then
  echo "Usage: $0 <pr-body-file>" >&2
  exit 1
fi

test -s "${body_file}"

git fetch origin main --quiet

behind_count="$(git rev-list --left-right --count HEAD...origin/main | awk '{print $2}')"
if [[ "${behind_count}" != "0" ]]; then
  echo "Branch is behind origin/main. Rebase before opening the PR." >&2
  exit 1
fi

base_sha="$(git merge-base origin/main HEAD)"

BASE_SHA="${base_sha}" HEAD_SHA="HEAD" bash scripts/ci/pr_body_check.sh "${body_file}"
BASE_SHA="${base_sha}" HEAD_SHA="HEAD" bash scripts/ci/spec_alignment_check.sh "${body_file}"
bash scripts/ci/repository_checks.sh

echo "PR preflight passed."

#!/usr/bin/env bash

set -euo pipefail

repo="enricopiovesan/Traverse"

open_in_progress_issue_numbers=$(
  gh issue list --repo "$repo" --state open --json number,labels \
    | jq -r '.[] | select(any(.labels[]?; .name == "in-progress")) | .number'
)

project_items_json=$(gh project item-list 1 --owner enricopiovesan --limit 200 --format json)

failures=0

for issue_number in $open_in_progress_issue_numbers; do
  project_status=$(
    jq -r --argjson issue_number "$issue_number" '
      .items[]
      | select(.content.number == $issue_number)
      | .status
    ' <<<"$project_items_json" | head -n 1
  )

  if [[ "$project_status" != "In Progress" ]]; then
    echo "Issue #$issue_number is labeled in-progress but Project 1 status is '${project_status:-missing}'" >&2
    failures=$((failures + 1))
  fi
done

open_pr_bodies=$(
  gh pr list --repo "$repo" --state open --json body \
    | jq -r '.[] | .body'
)

while IFS= read -r pr_body; do
  [[ -z "$pr_body" ]] && continue

  issue_number=$(
    grep -Eo 'Project Item[[:space:]]*[-:][[:space:]]*#[0-9]+' <<<"$pr_body" \
      | grep -Eo '[0-9]+' \
      | head -n 1
  )

  if [[ -z "${issue_number:-}" ]]; then
    continue
  fi

  issue_labels=$(
    gh issue view "$issue_number" --repo "$repo" --json labels \
      | jq -r '.labels[].name'
  )

  if ! grep -qx "in-progress" <<<"$issue_labels"; then
    echo "Open PR for issue #$issue_number exists but the issue is not labeled in-progress" >&2
    failures=$((failures + 1))
  fi
done <<<"$open_pr_bodies"

if [[ "$failures" -ne 0 ]]; then
  exit 1
fi

echo "Project state audit passed."

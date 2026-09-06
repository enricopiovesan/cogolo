#!/usr/bin/env bash

set -euo pipefail

repo="traverse-framework/traverse"

project_items_query='query {
  organization(login: "traverse-framework") {
    projectV2(number: 1) {
      items(first: 100) {
        nodes {
          content { ... on Issue { number repository { nameWithOwner } } }
          status: fieldValueByName(name: "Status") { ... on ProjectV2ItemFieldSingleSelectValue { name } }
          note: fieldValueByName(name: "Note") { ... on ProjectV2ItemFieldTextValue { text } }
        }
        pageInfo { endCursor hasNextPage }
      }
    }
  }
}'

project_items_after_query='query($after: String!) {
  organization(login: "traverse-framework") {
    projectV2(number: 1) {
      items(first: 100, after: $after) {
        nodes {
          content { ... on Issue { number repository { nameWithOwner } } }
          status: fieldValueByName(name: "Status") { ... on ProjectV2ItemFieldSingleSelectValue { name } }
          note: fieldValueByName(name: "Note") { ... on ProjectV2ItemFieldTextValue { text } }
        }
        pageInfo { endCursor hasNextPage }
      }
    }
  }
}'

project_items_json='{"items":[]}'
after=''
has_next_page=true
while [[ "$has_next_page" == "true" ]]; do
  if [[ -z "$after" ]]; then
    page_json=$(gh api graphql -f query="$project_items_query")
  else
    page_json=$(gh api graphql -f query="$project_items_after_query" -f after="$after")
  fi

  page_items=$(jq '[
    .data.organization.projectV2.items.nodes[]
    | select(.content.number != null)
    | {
        content: {
          type: "Issue",
          repository: .content.repository.nameWithOwner,
          number: .content.number
        },
        status: (.status.name // ""),
        note: (.note.text // "")
      }
  ]' <<<"$page_json")
  project_items_json=$(jq --argjson page_items "$page_items" '.items += $page_items' <<<"$project_items_json")
  after=$(jq -r '.data.organization.projectV2.items.pageInfo.endCursor // empty' <<<"$page_json")
  has_next_page=$(jq -r '.data.organization.projectV2.items.pageInfo.hasNextPage' <<<"$page_json")
done

failures=0

todo_issue_numbers=$(
  jq -r '
    .items[]
    | select(.content.type == "Issue")
    | select(.content.repository == "'"$repo"'")
    | select(.status == "Todo")
    | .content.number
  ' <<<"$project_items_json"
)

for issue_number in $todo_issue_numbers; do
  echo "Issue #$issue_number is still 'Todo'. Open issues must be moved to 'Ready', 'Blocked', or 'In Progress'." >&2
  failures=$((failures + 1))
done

blocked_without_note_issue_numbers=$(
  jq -r '
    .items[]
    | select(.content.type == "Issue")
    | select(.content.repository == "'"$repo"'")
    | select(.status == "Blocked")
    | select((.note // "") | gsub("[[:space:]]+"; "") == "")
    | .content.number
  ' <<<"$project_items_json"
)

for issue_number in $blocked_without_note_issue_numbers; do
  echo "Issue #$issue_number is 'Blocked' but has no Note. Blocked items must explain the blocker in Project 1's Note field." >&2
  failures=$((failures + 1))
done

open_pr_bodies=$(
  gh api "repos/$repo/pulls?state=open&per_page=100" --jq '.[].body'
)

while IFS= read -r pr_body; do
  [[ -z "$pr_body" ]] && continue

  issue_number=$(
    set +o pipefail
    grep -Eo 'Project Item[[:space:]]*[-:][[:space:]]*#[0-9]+' <<<"$pr_body" \
      | grep -Eo '[0-9]+' \
      | head -n 1 \
      || true
  )

  if [[ -z "${issue_number:-}" ]]; then
    continue
  fi

  project_status=$(
    jq -r --argjson issue_number "$issue_number" '
      .items[]
      | select(.content.type == "Issue")
      | select(.content.repository == "'"$repo"'")
      | select(.content.number == $issue_number)
      | .status
    ' <<<"$project_items_json" | head -n 1
  )

  if [[ -z "${project_status:-}" ]]; then
    echo "Open PR references issue #$issue_number but that issue is missing from Project 1." >&2
    failures=$((failures + 1))
    continue
  fi

  if [[ "$project_status" != "In Progress" ]]; then
    echo "Open PR references issue #$issue_number but Project 1 status is '${project_status}'. It must be 'In Progress' while a PR is open." >&2
    failures=$((failures + 1))
  fi
done <<<"$open_pr_bodies"

if [[ "$failures" -ne 0 ]]; then
  exit 1
fi

echo "Project state audit passed."

#!/usr/bin/env bash

set -euo pipefail

repo="${TRAVERSE_REPO:-enricopiovesan/Traverse}"
project_owner="${PROJECT_OWNER:-enricopiovesan}"
project_number="${PROJECT_NUMBER:-1}"
pr_number="${PR_NUMBER:-}"
pr_event_action="${PR_EVENT_ACTION:-}"
pr_merged="${PR_MERGED:-false}"

if [[ -z "$pr_number" ]]; then
  echo "No PR_NUMBER provided; skipping backlog sync." >&2
  exit 0
fi

if [[ "$pr_event_action" == "closed" && "$pr_merged" != "true" ]]; then
  echo "PR #$pr_number was closed without merge; no backlog sync performed." >&2
  exit 0
fi

if [[ "$pr_event_action" != "closed" && "$pr_event_action" != "opened" && "$pr_event_action" != "reopened" && "$pr_event_action" != "synchronize" && "$pr_event_action" != "ready_for_review" ]]; then
  echo "PR #$pr_number action '$pr_event_action' does not require backlog sync." >&2
  exit 0
fi

pr_json=$(gh pr view "$pr_number" --repo "$repo" --json body,closingIssuesReferences,title,mergedAt)
body=$(jq -r '.body // ""' <<<"$pr_json")

mapfile -t issue_numbers < <(
  jq -r '.closingIssuesReferences[]?.number // empty' <<<"$pr_json" | sort -n -u
)

if [[ "${#issue_numbers[@]}" -eq 0 ]]; then
  mapfile -t issue_numbers < <(
    grep -Eoi '(Closes|Fixes|Resolves|Implements|issue)[[:space:]]*#[0-9]+' <<<"$body" \
      | grep -Eo '[0-9]+' \
      | sort -n -u
  )
fi

if [[ "${#issue_numbers[@]}" -eq 0 ]]; then
  project_item_line=$(
    awk '
      BEGIN { in_section = 0 }
      /^## Project Item$/ { in_section = 1; next }
      /^## [^#]/ && in_section { exit }
      in_section && NF { print; exit }
    ' <<<"$body"
  )

  project_item_line="${project_item_line#- }"
  project_item_line="${project_item_line#Project 1 item: }"
  project_item_line="${project_item_line#Project Item: }"
  project_item_line="${project_item_line#GitHub Project 1 item for issue #}"

  if [[ "$project_item_line" =~ ^[0-9]+$ ]]; then
    issue_numbers+=("$project_item_line")
  elif [[ -n "$project_item_line" ]]; then
    mapfile -t issue_numbers < <(
      gh issue list --repo "$repo" --state all --limit 200 --json number,title \
        | jq -r --arg title "$project_item_line" '.[] | select(.title == $title) | .number' \
        | sort -n -u
    )
  fi
fi

if [[ "${#issue_numbers[@]}" -eq 0 ]]; then
  echo "No linked issue found for PR #$pr_number; nothing to sync." >&2
  exit 0
fi

project_json=$(gh project view "$project_number" --owner "$project_owner" --format json)
project_id=$(jq -r '.id' <<<"$project_json")
field_json=$(gh project field-list "$project_number" --owner "$project_owner" --format json)
status_field_id=$(jq -r '.fields[] | select(.name == "Status") | .id' <<<"$field_json")
done_option_id=$(jq -r '.fields[] | select(.name == "Status") | .options[] | select(.name == "Done") | .id' <<<"$field_json")
note_field_id=$(jq -r '.fields[] | select(.name == "Note") | .id' <<<"$field_json")
project_items_json=$(gh project item-list "$project_number" --owner "$project_owner" --format json --limit 200)

workflow_labels=(in-progress blocked ready future needs-spec)

for issue_number in "${issue_numbers[@]}"; do
  issue_json=$(gh issue view "$issue_number" --repo "$repo" --json state,labels,title)
  issue_state=$(jq -r '.state' <<<"$issue_json")
  issue_labels=$(jq -r '.labels[].name' <<<"$issue_json" 2>/dev/null || true)

  remove_args=()
  for label in "${workflow_labels[@]}"; do
    if grep -qx "$label" <<<"$issue_labels"; then
      remove_args+=(--remove-label "$label")
    fi
  done

  if [[ "$pr_merged" == "true" ]]; then
    if [[ "${#remove_args[@]}" -gt 0 ]]; then
      gh issue edit "$issue_number" --repo "$repo" "${remove_args[@]}"
    fi

    if [[ "$issue_state" == "OPEN" ]]; then
      gh issue close "$issue_number" --repo "$repo"
    fi
  else
    remove_args+=(--add-label in-progress)
    if [[ "${#remove_args[@]}" -gt 0 ]]; then
      gh issue edit "$issue_number" --repo "$repo" "${remove_args[@]}"
    fi
  fi

  item_id=$(jq -r --argjson n "$issue_number" '.items[] | select(.content.number == $n) | .id' <<<"$project_items_json" | head -n1)
  if [[ -z "${item_id:-}" || "$item_id" == "null" ]]; then
    echo "Issue #$issue_number has no Project 1 row; skipping project sync." >&2
    continue
  fi

  current_status=$(jq -r --argjson n "$issue_number" '.items[] | select(.content.number == $n) | .status' <<<"$project_items_json" | head -n1)

  if [[ "$pr_merged" == "true" ]]; then
    if [[ "$current_status" != "Done" ]]; then
      gh project item-edit \
        --project-id "$project_id" \
        --id "$item_id" \
        --field-id "$status_field_id" \
        --single-select-option-id "$done_option_id"
    fi

    gh project item-edit \
      --project-id "$project_id" \
      --id "$item_id" \
      --field-id "$note_field_id" \
      --text "Done: merged in PR #$pr_number."
  else
    if [[ "$current_status" != "In Progress" ]]; then
      gh project item-edit \
        --project-id "$project_id" \
        --id "$item_id" \
        --field-id "$status_field_id" \
        --single-select-option-id "$(jq -r '.fields[] | select(.name == \"Status\") | .options[] | select(.name == \"In Progress\") | .id' <<<"$field_json")"
    fi

    gh project item-edit \
      --project-id "$project_id" \
      --id "$item_id" \
      --field-id "$note_field_id" \
      --text "In progress: PR #$pr_number is open."
  fi
done

if [[ "$pr_merged" == "true" ]]; then
  echo "Backlog sync passed for merged PR #$pr_number."
else
  echo "Backlog sync passed for active PR #$pr_number."
fi

#!/usr/bin/env bash

set -euo pipefail

owner="${1:-enricopiovesan}"
project_number="${2:-1}"

tmp_json="$(mktemp)"
trap 'rm -f "$tmp_json"' EXIT

gh api graphql -f query="
query {
  user(login: \"$owner\") {
    projectV2(number: $project_number) {
      items(first: 100) {
        nodes {
          content {
            __typename
            ... on Issue {
              number
              state
              title
            }
          }
          fieldValues(first: 20) {
            nodes {
              __typename
              ... on ProjectV2ItemFieldSingleSelectValue {
                field {
                  ... on ProjectV2SingleSelectField {
                    name
                  }
                }
                name
              }
            }
          }
        }
      }
    }
  }
}
" > "$tmp_json"

python3 - "$tmp_json" <<'PY'
import json
import sys

path = sys.argv[1]
with open(path) as f:
    data = json.load(f)

items = data["data"]["user"]["projectV2"]["items"]["nodes"]
errors = []

for item in items:
    content = item.get("content") or {}
    if content.get("__typename") != "Issue":
        continue

    number = content["number"]
    title = content["title"]
    state = content["state"]
    status = None

    for field_value in item["fieldValues"]["nodes"]:
        if (
            field_value.get("__typename") == "ProjectV2ItemFieldSingleSelectValue"
            and field_value["field"]["name"] == "Status"
        ):
            status = field_value.get("name")
            break

    if status is None:
        errors.append(f"#{number} {title}: missing Project 1 status")
        continue

    if state == "CLOSED" and status != "Done":
        errors.append(
            f"#{number} {title}: issue is CLOSED but Project 1 status is {status}"
        )

    if state == "OPEN" and status == "Done":
        errors.append(
            f"#{number} {title}: issue is OPEN but Project 1 status is Done"
        )

    if state == "OPEN" and status == "Todo":
        errors.append(
            f"#{number} {title}: issue is OPEN but Project 1 status is Todo; use Ready or Blocked"
        )

if errors:
    print("Project board drift detected:")
    for error in errors:
        print(f"- {error}")
    sys.exit(1)

print("Project board audit passed.")
PY

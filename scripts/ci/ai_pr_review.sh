#!/usr/bin/env bash

set -euo pipefail

if [[ -z "${PR_NUMBER:-}" ]]; then
  echo "PR_NUMBER must be set." >&2
  exit 1
fi

if [[ -z "${REPOSITORY:-}" ]]; then
  echo "REPOSITORY must be set." >&2
  exit 1
fi

if [[ -z "${GITHUB_TOKEN:-}" ]]; then
  echo "GITHUB_TOKEN must be set." >&2
  exit 1
fi

if [[ -z "${OPENAI_API_KEY:-}" ]]; then
  echo "OPENAI_API_KEY is not configured. Skipping AI PR review."
  exit 0
fi

readonly OPENAI_MODEL="${OPENAI_MODEL:-gpt-4o-mini}"
readonly COMMENT_MARKER="<!-- cogolo-ai-review -->"
readonly DIFF_LIMIT=50000
readonly FILES_LIMIT=50

tmpdir="$(mktemp -d)"
trap 'rm -rf "${tmpdir}"' EXIT

pr_json_path="${tmpdir}/pr.json"
files_json_path="${tmpdir}/files.json"
diff_path="${tmpdir}/diff.txt"
prompt_path="${tmpdir}/prompt.txt"
request_path="${tmpdir}/request.json"
response_path="${tmpdir}/response.json"
review_json_path="${tmpdir}/review.json"
comment_body_path="${tmpdir}/comment.md"
patch_path="${tmpdir}/comment-patch.json"

gh api "repos/${REPOSITORY}/pulls/${PR_NUMBER}" >"${pr_json_path}"
gh api "repos/${REPOSITORY}/pulls/${PR_NUMBER}/files?per_page=100" >"${files_json_path}"
gh api "repos/${REPOSITORY}/pulls/${PR_NUMBER}" \
  -H "Accept: application/vnd.github.v3.diff" >"${diff_path}"

pr_title="$(jq -r '.title' "${pr_json_path}")"
pr_body="$(jq -r '.body // ""' "${pr_json_path}")"
base_ref="$(jq -r '.base.ref' "${pr_json_path}")"
head_ref="$(jq -r '.head.ref' "${pr_json_path}")"
files_summary="$(
  jq -r --argjson limit "${FILES_LIMIT}" '
    .[:$limit]
    | map("- " + .filename + " (" + .status + ", +" + (.additions | tostring) + ", -" + (.deletions | tostring) + ")")
    | join("\n")
  ' "${files_json_path}"
)"

if [[ "$(wc -c <"${diff_path}")" -gt "${DIFF_LIMIT}" ]]; then
  head -c "${DIFF_LIMIT}" "${diff_path}" >"${tmpdir}/diff-truncated.txt"
  mv "${tmpdir}/diff-truncated.txt" "${diff_path}"
fi

cat >"${prompt_path}" <<EOF
Review this pull request for Cogolo.

Project review priorities:
- spec alignment
- contract alignment
- compatibility risks
- missing tests
- hidden bypasses of contract, policy, constraint, or trace paths
- maintainability and production-grade quality

Return only JSON matching the provided schema.
Treat findings as blocking only when they should stop merge.
Keep the summary concise.

Repository: ${REPOSITORY}
PR number: ${PR_NUMBER}
Base branch: ${base_ref}
Head branch: ${head_ref}
Title: ${pr_title}

PR body:
${pr_body}

Changed files:
${files_summary}

Diff excerpt:
$(cat "${diff_path}")
EOF

jq -n \
  --arg model "${OPENAI_MODEL}" \
  --rawfile prompt "${prompt_path}" \
  '{
    model: $model,
    input: [
      {
        role: "system",
        content: [
          {
            type: "input_text",
            text: "You are a senior code reviewer for a spec-driven Rust and WASM platform project. Review pull requests carefully and return strict JSON only."
          }
        ]
      },
      {
        role: "user",
        content: [
          {
            type: "input_text",
            text: $prompt
          }
        ]
      }
    ],
    text: {
      format: {
        type: "json_schema",
        name: "pr_review",
        strict: true,
        schema: {
          type: "object",
          additionalProperties: false,
          properties: {
            summary: { type: "string" },
            blocking: { type: "boolean" },
            findings: {
              type: "array",
              items: {
                type: "object",
                additionalProperties: false,
                properties: {
                  severity: {
                    type: "string",
                    enum: ["high", "medium", "low"]
                  },
                  title: { type: "string" },
                  body: { type: "string" },
                  file: { type: "string" },
                  line: {
                    anyOf: [
                      { type: "integer" },
                      { type: "null" }
                    ]
                  },
                  blocking: { type: "boolean" }
                },
                required: [
                  "severity",
                  "title",
                  "body",
                  "file",
                  "line",
                  "blocking"
                ]
              }
            }
          },
          required: ["summary", "blocking", "findings"]
        }
      }
    }
  }' >"${request_path}"

curl -fsS https://api.openai.com/v1/responses \
  -H "Authorization: Bearer ${OPENAI_API_KEY}" \
  -H "Content-Type: application/json" \
  -d @"${request_path}" >"${response_path}"

jq -r '
  .output[]
  | select(.type == "message")
  | .content[]
  | select(.type == "output_text")
  | .text
' "${response_path}" >"${review_json_path}"

jq empty "${review_json_path}"

{
  echo "${COMMENT_MARKER}"
  echo
  echo "## AI Review"
  echo
  jq -r '.summary' "${review_json_path}"
  echo
  if [[ "$(jq '.findings | length' "${review_json_path}")" -eq 0 ]]; then
    echo "No findings."
  else
    echo "### Findings"
    echo
    jq -r '
      .findings[]
      | "- [" + (if .blocking then "blocking" else "non-blocking" end) + "] "
        + .severity + ": " + .title
        + (if .file == "" then "" else " (" + .file + (if .line == null then "" else ":" + (.line | tostring) end) + ")" end)
        + "\n  " + .body
    ' "${review_json_path}"
  fi
} >"${comment_body_path}"

existing_comment_id="$(
  gh api "repos/${REPOSITORY}/issues/${PR_NUMBER}/comments?per_page=100" --jq \
    '.[] | select(.user.login == "github-actions[bot]" and (.body | contains("<!-- cogolo-ai-review -->"))) | .id' \
    | tail -n 1
)"

if [[ -n "${existing_comment_id}" ]]; then
  jq -n --rawfile body "${comment_body_path}" '{body: $body}' >"${patch_path}"
  gh api -X PATCH "repos/${REPOSITORY}/issues/comments/${existing_comment_id}" --input "${patch_path}" >/dev/null
else
  gh pr comment "${PR_NUMBER}" --body-file "${comment_body_path}" >/dev/null
fi

if jq -e '.blocking == true' "${review_json_path}" >/dev/null; then
  echo "AI review found blocking issues." >&2
  exit 1
fi

echo "AI review completed without blocking issues."

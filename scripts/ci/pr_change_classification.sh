#!/usr/bin/env bash

set -euo pipefail

# Classify a PR as documentation-only only when every changed path is explicitly
# allowlisted. Unknown paths deliberately require full validation.
readonly DOCS_ONLY_PATHS=(
  'README.md'
  'CONTRIBUTING.md'
  'CODE_OF_CONDUCT.md'
  'SECURITY.md'
  'SUPPORT.md'
  'quickstart.md'
)

is_docs_only_path() {
  local path="$1"

  case "$path" in
    docs/*|adr/*)
      return 0
      ;;
    specs/governance/*)
      # This registry determines the scope of the spec-alignment gate.
      return 1
      ;;
    specs/*)
      return 0
      ;;
  esac

  local allowed
  for allowed in "${DOCS_ONLY_PATHS[@]}"; do
    [[ "$path" == "$allowed" ]] && return 0
  done

  return 1
}

read_changed_paths() {
  if [[ "${1:-}" == "--paths-stdin" ]]; then
    cat
    return
  fi

  local base_sha="${1:-${GITHUB_BASE_SHA:-}}"
  local head_sha="${2:-${GITHUB_HEAD_SHA:-HEAD}}"
  [[ -n "$base_sha" ]] || {
    echo "Usage: $0 <base-sha> [head-sha], or --paths-stdin" >&2
    exit 1
  }
  git diff --name-only "${base_sha}...${head_sha}"
}

full_ci=false
reason="documentation-only change"
paths=()

while IFS= read -r path; do
  [[ -z "$path" ]] && continue
  paths+=("$path")
  if ! is_docs_only_path "$path"; then
    full_ci=true
    reason="full validation required by ${path}"
    break
  fi
done < <(read_changed_paths "$@")

if [[ ${#paths[@]} -eq 0 ]]; then
  full_ci=true
  reason="no changed paths were available"
fi

printf 'full_ci=%s\n' "$full_ci"
printf 'reason=%s\n' "$reason"

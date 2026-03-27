#!/usr/bin/env bash

set -euo pipefail

readonly APPROVED_SPECS_FILE="specs/governance/approved-specs.json"
readonly PR_BODY_FILE="${1:-}"
readonly BASE_SHA="${BASE_SHA:-${GITHUB_BASE_SHA:-}}"
readonly HEAD_SHA="${HEAD_SHA:-${GITHUB_HEAD_SHA:-HEAD}}"

if [[ -z "${PR_BODY_FILE}" ]]; then
  echo "Usage: $0 <pr-body-file>" >&2
  exit 1
fi

if [[ ! -f "${APPROVED_SPECS_FILE}" ]]; then
  echo "Missing approved spec registry: ${APPROVED_SPECS_FILE}" >&2
  exit 1
fi

if [[ ! -s "${PR_BODY_FILE}" ]]; then
  echo "PR body file is missing or empty: ${PR_BODY_FILE}" >&2
  exit 1
fi

if [[ -z "${BASE_SHA}" ]]; then
  echo "BASE_SHA or GITHUB_BASE_SHA must be set for spec alignment checks." >&2
  exit 1
fi

if ! git rev-parse --verify "${BASE_SHA}^{commit}" >/dev/null 2>&1; then
  echo "Base commit not available locally: ${BASE_SHA}" >&2
  exit 1
fi

if ! git rev-parse --verify "${HEAD_SHA}^{commit}" >/dev/null 2>&1; then
  echo "Head commit not available locally: ${HEAD_SHA}" >&2
  exit 1
fi

tmp_json="$(mktemp)"
trap 'rm -f "${tmp_json}"' EXIT

python3 - "${APPROVED_SPECS_FILE}" "${PR_BODY_FILE}" "${BASE_SHA}" "${HEAD_SHA}" > "${tmp_json}" <<'PY'
import json
import subprocess
import sys
from pathlib import Path

approved_specs_file = Path(sys.argv[1])
pr_body_file = Path(sys.argv[2])
base_sha = sys.argv[3]
head_sha = sys.argv[4]

failures = []

try:
    registry = json.loads(approved_specs_file.read_text())
except Exception as exc:
    failures.append(
        {
            "code": "registry.invalid_json",
            "path": str(approved_specs_file),
            "message": f"Unable to parse approved spec registry: {exc}",
        }
    )
    print(json.dumps({"status": "failed", "failures": failures}))
    sys.exit(0)

if registry.get("schema_version") != "1.0.0":
    failures.append(
        {
            "code": "registry.schema_version_invalid",
            "path": str(approved_specs_file),
            "message": "Approved spec registry schema_version must be 1.0.0.",
        }
    )

specs = registry.get("specs")
if not isinstance(specs, list) or not specs:
    failures.append(
        {
            "code": "registry.specs_missing",
            "path": str(approved_specs_file),
            "message": "Approved spec registry must contain at least one spec record.",
        }
    )
    print(json.dumps({"status": "failed", "failures": failures}))
    sys.exit(0)

by_id = {}
paths_seen = set()

for spec in specs:
    spec_id = spec.get("id")
    spec_path = spec.get("path")

    if not spec_id:
        failures.append(
            {
                "code": "registry.spec_id_missing",
                "path": str(approved_specs_file),
                "message": "Spec record is missing id.",
            }
        )
        continue

    if spec_id in by_id:
        failures.append(
            {
                "code": "registry.spec_id_duplicate",
                "path": str(approved_specs_file),
                "message": f"Duplicate approved spec id: {spec_id}",
            }
        )
        continue

    by_id[spec_id] = spec

    if not spec_path:
        failures.append(
            {
                "code": "registry.spec_path_missing",
                "path": str(approved_specs_file),
                "message": f"Approved spec {spec_id} is missing path.",
            }
        )
    elif spec_path in paths_seen:
        failures.append(
            {
                "code": "registry.spec_path_duplicate",
                "path": str(approved_specs_file),
                "message": f"Duplicate approved spec path: {spec_path}",
            }
        )
    else:
        paths_seen.add(spec_path)
        if not Path(spec_path).is_file():
            failures.append(
                {
                    "code": "registry.spec_path_missing_on_disk",
                    "path": spec_path,
                    "message": f"Approved spec path does not exist: {spec_path}",
                }
            )

    if spec.get("status") != "approved":
        failures.append(
            {
                "code": "registry.spec_not_approved",
                "path": str(approved_specs_file),
                "message": f"Approved spec registry entry {spec_id} must have status=approved.",
            }
        )

    if spec.get("immutable") is not True:
        failures.append(
            {
                "code": "registry.spec_not_immutable",
                "path": str(approved_specs_file),
                "message": f"Approved spec registry entry {spec_id} must have immutable=true.",
            }
        )

    governs = spec.get("governs")
    if not isinstance(governs, list) or not governs:
        failures.append(
            {
                "code": "registry.governs_missing",
                "path": str(approved_specs_file),
                "message": f"Approved spec {spec_id} must declare at least one governed path prefix.",
            }
        )

try:
    pr_body = pr_body_file.read_text()
except Exception as exc:
    failures.append(
        {
            "code": "input.pr_body_unreadable",
            "path": str(pr_body_file),
            "message": f"Unable to read PR body file: {exc}",
        }
    )
    print(json.dumps({"status": "failed", "failures": failures}))
    sys.exit(0)

declared_spec_ids = []
in_section = False
for raw_line in pr_body.splitlines():
    line = raw_line.rstrip()
    if line == "## Governing Spec":
        in_section = True
        continue
    if in_section and line.startswith("## "):
        break
    if in_section:
        stripped = line.strip()
        if not stripped:
            continue
        if stripped.startswith("- "):
            declared_spec_ids.append(stripped[2:].strip())

if not in_section:
    failures.append(
        {
            "code": "input.pr_governing_spec_section_missing",
            "path": str(pr_body_file),
            "message": "PR body must include a ## Governing Spec section.",
        }
    )

declared_spec_ids = list(dict.fromkeys(declared_spec_ids))

for spec_id in declared_spec_ids:
    spec = by_id.get(spec_id)
    if spec is None:
        failures.append(
            {
                "code": "registry.spec_missing",
                "path": str(pr_body_file),
                "message": f"Declared governing spec is not in the approved registry: {spec_id}",
            }
        )
        continue
    if spec.get("status") != "approved":
        failures.append(
            {
                "code": "registry.spec_not_approved",
                "path": str(pr_body_file),
                "message": f"Declared governing spec is not approved: {spec_id}",
            }
        )
    if spec.get("immutable") is not True:
        failures.append(
            {
                "code": "registry.spec_not_immutable",
                "path": str(pr_body_file),
                "message": f"Declared governing spec is not immutable: {spec_id}",
            }
        )

changed_files = subprocess.check_output(
    ["git", "diff", "--name-only", f"{base_sha}...{head_sha}"],
    text=True,
).splitlines()
changed_files = [path.strip() for path in changed_files if path.strip()]

governed_files = []
required_spec_ids = set()

for path in changed_files:
    matching_spec_ids = []
    for spec_id, spec in by_id.items():
        for prefix in spec.get("governs", []):
            if path.startswith(prefix):
                matching_spec_ids.append(spec_id)
    if matching_spec_ids:
        governed_files.append(path)
        required_spec_ids.update(matching_spec_ids)
    elif path.startswith(("crates/", "scripts/ci/", ".github/workflows/", "specs/governance/")):
        failures.append(
            {
                "code": "coverage.file_unmapped",
                "path": path,
                "message": f"Governed file is not covered by an approved spec record: {path}",
            }
        )

for spec_id in sorted(required_spec_ids):
    if spec_id not in declared_spec_ids:
        failures.append(
            {
                "code": "coverage.spec_not_declared",
                "path": str(pr_body_file),
                "message": f"Changed governed files require governing spec {spec_id}, but it was not declared in the PR body.",
            }
        )

status = "passed" if not failures else "failed"

print(
    json.dumps(
        {
            "status": status,
            "changed_files": changed_files,
            "governed_files": governed_files,
            "required_spec_ids": sorted(required_spec_ids),
            "declared_spec_ids": declared_spec_ids,
            "failures": failures,
        }
    )
)
PY

status="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["status"])' "${tmp_json}")"

echo "Spec alignment evaluation:"
python3 - "${tmp_json}" <<'PY'
import json
import sys

result = json.load(open(sys.argv[1]))
print(f"  status: {result['status']}")
print(f"  declared specs: {', '.join(result.get('declared_spec_ids', [])) or '(none)'}")
print(f"  required specs: {', '.join(result.get('required_spec_ids', [])) or '(none)'}")
if result.get("governed_files"):
    print("  governed files:")
    for path in result["governed_files"]:
        print(f"    - {path}")
if result.get("failures"):
    print("  failures:")
    for failure in result["failures"]:
        print(f"    - [{failure['code']}] {failure['message']}")
PY

if [[ "${status}" != "passed" ]]; then
  exit 1
fi

echo "Spec alignment check passed."

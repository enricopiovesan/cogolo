#!/usr/bin/env bash

set -euo pipefail

repo_root="${TRAVERSE_REPO_ROOT:-$(pwd)}"
cd "$repo_root"

template_dir="examples/templates/executable-capability-package"
manifest_path="$template_dir/manifest.template.json"
build_script="$template_dir/build-fixture.sh"
source_stub="$template_dir/src/implementation.rs"
artifact_ignore="$template_dir/artifacts/.gitignore"

test -f "$manifest_path"
test -f "$build_script"
test -f "$source_stub"
test -f "$artifact_ignore"

grep -q '"kind": "agent_package"' "$manifest_path"
grep -q '"package_id": "example.domain.replace-me-agent"' "$manifest_path"
grep -q '"capability_ref"' "$manifest_path"
grep -q '"workflow_refs"' "$manifest_path"
grep -q '"model_dependencies"' "$manifest_path"
grep -q '"expected_digest": "fnv1a64:dffc31d6401c84d6"' "$manifest_path"
grep -q 'printf' "$build_script"

printf 'Executable package template smoke passed.\n'

#!/usr/bin/env bash
set -euo pipefail

repo_root="${TRAVERSE_REPO_ROOT:-$(pwd)}"

required_files=(
  "apps/android-demo/README.md"
  "apps/android-demo/settings.gradle.kts"
  "apps/android-demo/build.gradle.kts"
  "apps/android-demo/app/build.gradle.kts"
  "apps/android-demo/app/src/main/AndroidManifest.xml"
  "apps/android-demo/app/src/main/java/com/traverse/demo/android/MainActivity.kt"
  "apps/android-demo/app/src/main/res/values/strings.xml"
  "apps/android-demo/app/src/main/assets/expedition-runtime-session.json"
)

for file in "${required_files[@]}"; do
  if [[ ! -f "${repo_root}/${file}" ]]; then
    echo "missing Android demo artifact: ${file}" >&2
    exit 1
  fi
done

grep -q 'setContent' "${repo_root}/apps/android-demo/app/src/main/java/com/traverse/demo/android/MainActivity.kt"
grep -q 'Traverse Android Demo' "${repo_root}/apps/android-demo/app/src/main/res/values/strings.xml"
grep -q '"status": "completed"' "${repo_root}/apps/android-demo/app/src/main/assets/expedition-runtime-session.json"

echo "Android demo smoke passed"

#!/usr/bin/env bash

set -euo pipefail

repo_root="${TRAVERSE_REPO_ROOT:-$(pwd)}"
app_dir="$repo_root/apps/react-demo"

test -f "${app_dir}/index.html"
test -f "${app_dir}/public/expedition-runtime-session.json"
test -f "${app_dir}/src/main.js"
test -f "${app_dir}/src/styles.css"
test -f "${app_dir}/vendor/react.development.js"
test -f "${app_dir}/vendor/react-dom.development.js"

grep -q "Traverse React Demo" "${app_dir}/index.html"
grep -q '"status": "completed"' "${app_dir}/public/expedition-runtime-session.json"
grep -q "Submit approved request" "${app_dir}/src/main.js"
grep -q "react.development.js" "${app_dir}/index.html"
grep -q "react-dom.development.js" "${app_dir}/index.html"
node --check "${app_dir}/src/main.js"

printf 'React demo smoke passed.\n'

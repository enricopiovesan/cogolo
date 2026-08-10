#!/usr/bin/env bash
set -euo pipefail
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
artifact_dir="$script_dir/artifacts"
artifact_path="$artifact_dir/core-generate-nudge-message.wasm"
manifest_path="$script_dir/manifest.json"
mkdir -p "$artifact_dir"
rustup run "$(rustup show active-toolchain | awk '{print $1}')" rustc "$script_dir/src/agent.rs" \
  --target wasm32-unknown-unknown --crate-type cdylib -O -C panic=abort -C strip=symbols \
  --remap-path-prefix "$script_dir=/traverse-repo/agent" -o "$artifact_path"
digest="$(python3 - "$artifact_path" <<'PY'
import sys
from pathlib import Path
data = Path(sys.argv[1]).read_bytes()
h = 0xcbf29ce484222325
for b in data:
    h ^= b
    h = (h * 0x100000001b3) & 0xFFFFFFFFFFFFFFFF
print(f"fnv1a64:{h:016x}")
PY
)"
python3 - "$manifest_path" "$digest" <<'PY'
import json, sys
from pathlib import Path
manifest = Path(sys.argv[1])
digest = sys.argv[2]
data = json.loads(manifest.read_text())
data.setdefault("binary", {})["expected_digest"] = digest
manifest.write_text(json.dumps(data, indent=2) + "\n")
print(f"updated {manifest}: binary.expected_digest={digest}")
PY
printf 'built %s\n' "$artifact_path"

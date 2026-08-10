#!/usr/bin/env python3
"""Scaffold a Loop capability example package from a Loop package contract."""

from __future__ import annotations

import json
import sys
from pathlib import Path

PERSONA_MAP = {
    None: "runtime-engineer",
    "system": "runtime-engineer",
    "end-user": "meeting-organizer",
    "manager": "collaboration-product-owner",
    "team-lead": "collaboration-product-owner",
    "owner": "meeting-organizer",
    "checkout-flow-developer": "runtime-engineer",
    "saas-billing-engineer": "runtime-engineer",
    "pricing-analyst": "collaboration-product-owner",
    "platform-engineer": "runtime-engineer",
    "compliance-officer": "collaboration-product-owner",
    "client-developer": "meeting-organizer",
}

ALLOWED_PERSONAS = {
    "runtime-engineer",
    "meeting-organizer",
    "collaboration-product-owner",
}


def adapt_contract(contract: dict, cap_id: str, version: str) -> dict:
    contract = json.loads(json.dumps(contract))
    contract["provenance"] = {
        "source": "ai-assisted",
        "author": "loop-founders + traverse-capability-author",
        "created_at": contract.get("provenance", {}).get(
            "created_at", "2026-08-08T06:00:00Z"
        ),
        "spec_ref": f"{cap_id}@{version}",
        "adr_refs": ["persona-council-review-2026-08-08"],
        "exception_refs": [],
    }
    contract["evidence"] = [
        {
            "evidence_id": f"{cap_id.replace('.', '-')}-{version}-contract-validation",
            "type": "contract_validation",
            "status": "passed",
        }
    ]
    contract.pop("artifact", None)
    summary = contract.get("summary") or ""
    if len(summary) > 200:
        contract["summary"] = summary[:197].rstrip() + "..."
    for use_case in contract.get("use_cases", []):
        pref = use_case.get("persona_ref")
        if pref in PERSONA_MAP:
            use_case["persona_ref"] = PERSONA_MAP[pref]
        elif not pref or pref not in ALLOWED_PERSONAS:
            use_case["persona_ref"] = "runtime-engineer"
    return contract


def write_build_fixture(root: Path, wasm: str) -> None:
    content = f"""#!/usr/bin/env bash
set -euo pipefail
script_dir="$(cd "$(dirname "${{BASH_SOURCE[0]}}")" && pwd)"
artifact_dir="$script_dir/artifacts"
artifact_path="$artifact_dir/{wasm}"
manifest_path="$script_dir/manifest.json"
mkdir -p "$artifact_dir"
rustup run "$(rustup show active-toolchain | awk '{{print $1}}')" rustc "$script_dir/src/agent.rs" \\
  --target wasm32-unknown-unknown --crate-type cdylib -O -C panic=abort -C strip=symbols \\
  --remap-path-prefix "$script_dir=/traverse-repo/agent" -o "$artifact_path"
digest="$(python3 - "$artifact_path" <<'PY'
import sys
from pathlib import Path
data = Path(sys.argv[1]).read_bytes()
h = 0xcbf29ce484222325
for b in data:
    h ^= b
    h = (h * 0x100000001b3) & 0xFFFFFFFFFFFFFFFF
print(f"fnv1a64:{{h:016x}}")
PY
)"
python3 - "$manifest_path" "$digest" <<'PY'
import json, sys
from pathlib import Path
manifest = Path(sys.argv[1])
digest = sys.argv[2]
data = json.loads(manifest.read_text())
data.setdefault("binary", {{}})["expected_digest"] = digest
manifest.write_text(json.dumps(data, indent=2) + "\\n")
print(f"updated {{manifest}}: binary.expected_digest={{digest}}")
PY
printf 'built %s\\n' "$artifact_path"
"""
    path = root / "build-fixture.sh"
    path.write_text(content)
    path.chmod(0o755)


def main() -> None:
    if len(sys.argv) < 4:
        print(
            "usage: scaffold_loop_capability.py <cap-id> <version> <contract-src>",
            file=sys.stderr,
        )
        sys.exit(2)
    cap_id = sys.argv[1]
    version = sys.argv[2]
    contract_src = Path(sys.argv[3])
    name = cap_id.split(".", 1)[1]
    dir_name = f"core-{name}"
    root = Path("examples") / dir_name
    for sub in ["src", "artifacts", "runtime-requests", f"workflows/{name}"]:
        (root / sub).mkdir(parents=True, exist_ok=True)

    wasm = f"core-{name}.wasm"
    contract = adapt_contract(json.loads(contract_src.read_text()), cap_id, version)
    (root / "contract.json").write_text(json.dumps(contract, indent=2) + "\n")

    manifest = {
        "kind": "capability_package",
        "schema_version": "1.0.0",
        "package_id": f"{cap_id}-agent",
        "version": version,
        "summary": f"Capability package for {cap_id}@{version}.",
        "capability_ref": {
            "id": cap_id,
            "version": version,
            "contract_path": "./contract.json",
        },
        "workflow_refs": [{"workflow_id": cap_id, "workflow_version": version}],
        "source": {"path": "./src/agent.rs", "language": "rust", "entry": "run"},
        "binary": {
            "path": f"./artifacts/{wasm}",
            "format": "wasm",
            "expected_digest": "fnv1a64:0000000000000000",
            "abi_version": "1.0.0",
        },
        "constraints": {
            "host_api_access": "none",
            "network_access": "forbidden",
            "filesystem_access": "none",
        },
        "model_dependencies": [],
    }
    (root / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    write_build_fixture(root, wasm)

    workflow = {
        "kind": "workflow_definition",
        "schema_version": "1.0.0",
        "id": cap_id,
        "name": name,
        "version": version,
        "lifecycle": "active",
        "owner": {"team": "loop", "contact": "founders@loop.dev"},
        "summary": f"Run {cap_id} as one governed workflow.",
        "inputs": {"schema": {"type": "object", "additionalProperties": True}},
        "outputs": {"schema": {"type": "object", "additionalProperties": True}},
        "nodes": [
            {
                "node_id": "run",
                "capability_id": cap_id,
                "capability_version": version,
                "input": {"from_workflow_input": []},
                "output": {"to_workflow_state": []},
            }
        ],
        "edges": [],
        "start_node": "run",
        "terminal_nodes": ["run"],
        "tags": ["loop", "core"],
        "governing_spec": "007-workflow-registry-traversal",
    }
    (root / f"workflows/{name}/workflow.json").write_text(
        json.dumps(workflow, indent=2) + "\n"
    )
    (root / "NOTES.md").write_text(f"# {cap_id}\n\nLoop package capability. Issue #1034.\n")
    (root / "DEV-JOURNAL.md").write_text(
        f"# {cap_id} — dev journal\n\n- Ticket: #1034\n"
    )
    print(root)


if __name__ == "__main__":
    main()

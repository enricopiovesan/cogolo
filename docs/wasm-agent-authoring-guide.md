# Traverse WASM Agent Authoring Guide

This guide shows how to create a new governed WASM agent in Traverse without inventing a separate packaging model.

Use the checked-in examples as the source of truth:

- [`examples/templates/executable-capability-package/manifest.template.json`](../examples/templates/executable-capability-package/manifest.template.json)
- [`examples/capabilities/expedition-intent-agent/manifest.json`](../examples/capabilities/expedition-intent-agent/manifest.json)
- [`examples/capabilities/team-readiness-agent/manifest.json`](../examples/capabilities/team-readiness-agent/manifest.json)
- [`docs/wasm-io-contract.md`](wasm-io-contract.md)

## From Hello World to Your First Custom Capability

If you've completed the zero-to-hero path, you have a working `say-hello` agent. This section walks you from that starting point to a capability with your own interface.

### Generate a capability scaffold

```bash
traverse-cli capability new acme.ml.my-classifier
```

This generates `capabilities/acme.ml.my-classifier/` with a real, loadable `kind: capability_package` manifest, a contract with authorable placeholder input/output fields, a `#![no_std]` WASI guest stub, and a sample runtime request — the same shape `traverse-cli capability-package inspect`/`traverse-cli capability-package execute` load in production. `scripts/scaffold/new-capability.sh` is retired (spec `100-capability-package-authoring` FR-009); it now exits non-zero and points here. `component new` is retired too (FR-008): it redirects to `capability new`.

### What to change

1. **`contract.json`** — replace the placeholder `input_value`/`output_value` fields under `inputs.schema`/`outputs.schema` with your actual fields. The `description` field is required.
2. **`src/agent.rs`** — replace the static placeholder output with your computation. Read JSON from stdin, write JSON to stdout.
3. **Build and verify** — `bash capabilities/acme.ml.my-classifier/build-fixture.sh` compiles to WASM; `traverse-cli capability-package inspect <manifest>` reports the real digest to paste into `manifest.json`'s `binary.expected_digest` if it doesn't match yet, then `traverse-cli capability-package execute <manifest> <request>` runs it.

### Hand-rolled JSON parsing

The no-std stub includes `find_key_at_depth` for guests that parse JSON bytes
directly. Use it for root fields that can also appear in nested objects:
serde does not promise object-key order, so an unscoped search for
`"principal"` can accidentally select `policy.rules[].principal` before the
top-level field. Root object fields are depth `1`; once a containing object is
selected, parse nested fields only within that scoped slice. See
`examples/core-authorize/src/agent.rs` for the complete pattern.

### Common mistake

Do not skip the contract edit. The runtime validates inputs and outputs against the schema before executing. A permissive schema (`additionalProperties: true`) will pass validation but defeat governance.

If a use case must return a structured guest denial for a missing field, keep
that field schema-optional and enforce it in the guest — host `required` will
otherwise reject the request before the guest runs. Document the field as
guest-enforced and cover it with a use case (see
[`docs/capability-contract-authoring-guide.md`](capability-contract-authoring-guide.md)
“Guest-enforced fields”; example: `examples/core-authorize` UC-09).

## Start From a Governed Package

Begin with the executable capability package template, then specialize it for the new agent:

- choose one governed `package_id`
- bind exactly one approved capability contract
- bind the agent to the workflow it participates in
- keep the source entry point explicit
- keep the binary path and digest explicit
- keep host API, network, and filesystem access governed and narrow
- declare model dependencies as abstract interfaces, not direct implementation hooks

## Minimal Package Shape

A new agent package should make these fields obvious:

- `package_id`
- `version`
- `summary`
- `capability_ref`
- `known_compositions` (optional advisory metadata; legacy `workflow_refs` is
  accepted only during the schema-v1 migration window)
- `source`
- `binary`
- `constraints`
- `model_dependencies`

The package must remain a portable WASM-backed artifact bundle, not a generic host-bound executable.

Its execution boundary is the governed stdin/stdout JSON contract documented in
[`docs/wasm-io-contract.md`](wasm-io-contract.md).

## Authoring Steps

1. Copy the template manifest into a new agent directory.
2. Replace the placeholder capability reference; add known compositions only
   when they are real tested compositions, never as an execution prerequisite.
3. Point `source.path` at the agent implementation file.
4. Build the deterministic local fixture for the agent package.
5. Update the expected digest after the fixture is built.
6. Verify the package with `traverse-cli capability-package inspect`.
7. Verify the runtime path with `traverse-cli capability-package execute`.
8. Run the example smoke script before opening a PR.

## Validation

Run the agent authoring smoke path with:

```bash
bash scripts/ci/wasm_agent_authoring_guide_smoke.sh
```

That smoke path confirms the guide points at the governed template, the approved example packages, and the deterministic Traverse CLI validation flow.

## Common Mistakes

- skipping the governed manifest and improvising a package shape
- declaring host access that is broader than the approved capability contract allows
- treating the example as a general microservice instead of a governed agent package
- inventing a workflow reference so an independently useful package can execute
- changing the binary digest without rebuilding the fixture

---

## Template Stub vs. Real Implementation (#289)

The package template at `examples/templates/executable-capability-package/src/implementation.rs` contains a minimal stub:

```rust
pub fn run() -> &'static str { "" }
```

This is a placeholder **not a starting point for logic**. A real WASM agent reads a JSON payload from stdin, processes it, and writes a JSON result to stdout. The expedition agent (`crates/traverse-expedition-wasm/src/main.rs`) is the canonical reference for a complete implementation.

### Minimal real WASM agent

```rust
use std::io::{self, Read, Write};

fn main() {
    // Read entire stdin as JSON input
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap_or_default();

    let request: serde_json::Value = serde_json::from_str(&input)
        .unwrap_or(serde_json::Value::Null);

    // Process — replace with your logic
    let subject = request
        .get("subject")
        .and_then(|v| v.as_str())
        .unwrap_or("world");

    let output = serde_json::json!({ "greeting": format!("Hello, {}!", subject) });

    // Write JSON result to stdout
    let _ = io::stdout().write_all(output.to_string().as_bytes());
}
```

The stdin/stdout JSON I/O contract is documented in [`docs/wasm-io-contract.md`](wasm-io-contract.md).

### Building for WASM

```bash
cargo build \
  --manifest-path examples/your-agent/Cargo.toml \
  --target wasm32-wasip1 \
  --release
```

The output binary is at `target/wasm32-wasip1/release/your-agent.wasm`.

---

## `build-fixture.sh` Scripts (#299)

Example agents include a `build-fixture.sh` script (e.g. `examples/hello-world/say-hello-agent/build-fixture.sh`). This script:

1. Compiles the WASM binary from source using `cargo build --target wasm32-wasip1`
2. Copies the output to a deterministic `fixture/` path inside the example directory
3. Updates the expected SHA-256 digest in the agent's `manifest.json`

**Why fixtures matter**: The capability registry enforces immutability — once a version is registered, its digest must not change. If you rebuild a WASM binary and the digest changes, re-registration of the same version will fail with `ImmutableVersionConflict`. Fixtures ensure the checked-in binary and digest are always in sync.

**Do all agents need one?** Only agents with integration tests or CI smoke paths. If your agent is only used locally and not part of the CI smoke path, you do not need a fixture script. But any agent included in `scripts/ci/` smoke paths must have a deterministic fixture.

### Minimal `build-fixture.sh` template

```bash
#!/usr/bin/env bash
set -euo pipefail

AGENT_DIR="$(cd "$(dirname "$0")" && pwd)"
MANIFEST="$AGENT_DIR/manifest.json"

# Build the WASM binary
cargo build \
  --manifest-path "$AGENT_DIR/Cargo.toml" \
  --target wasm32-wasip1 \
  --release 2>&1

# Copy to fixture location
mkdir -p "$AGENT_DIR/fixture"
cp "target/wasm32-wasip1/release/$(basename "$AGENT_DIR").wasm" \
   "$AGENT_DIR/fixture/agent.wasm"

# Print the new digest for manual update of manifest.json
shasum -a 256 "$AGENT_DIR/fixture/agent.wasm"
echo "Update manifest.json binary.digest with the value above."
```

---

## `model_dependencies` (#296)

Agent manifests may declare `model_dependencies` — abstract interface names that describe which language model interfaces the agent relies on:

```json
"model_dependencies": [
  "expedition-intent-interpretation-v1"
]
```

**What they are**: Named contracts for LLM interface behaviour (e.g. "given this prompt format, return this JSON shape"). They are abstract — not tied to a specific model provider.

**How the runtime uses them**: App-level `model_dependencies` are governed runtime dependencies. Traverse loads the registered app declaration, resolves an available candidate for the requested abstract inference interface, invokes the governed provider implementation, and returns model resolution evidence with the inference output. A WASM agent must not hardcode Ollama, llama.cpp, WebLLM, cloud APIs, provider URLs, credentials, or provider SDK calls inside its binary.

**How to declare a new interface**: Use the governed interface `traverse.inference.generate` unless an approved spec adds another interface. Declare concrete candidates in the application manifest, keep provider configuration in runtime-local workspace config, and route execution through Traverse's governed model dependency surface.

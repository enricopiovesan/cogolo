# Traverse CLI Reference

This page documents the supported `traverse-cli` command surface as it exists today.

It is meant to be a stable reference for humans and agents. It distinguishes the public command surface from example-only and test-only paths so downstream users do not have to infer the API from source code.

## Help Behavior

The current binary exposes a single top-level usage string.

These invocations all print the same usage synopsis today:

- `cargo run -p traverse-cli -- --help`
- `cargo run -p traverse-cli -- help`
- `cargo run -p traverse-cli -- agent --help`
- `cargo run -p traverse-cli -- browser-adapter serve --help`

There is not yet a nested help tree with per-subcommand long-form help pages.

## Supported Commands

| Command | Purpose | Example | Expected Output |
|---|---|---|---|
| `bundle inspect <manifest-path>` | Validate and summarize a registry bundle manifest. | `cargo run -p traverse-cli -- bundle inspect examples/expedition/registry-bundle/manifest.json` | Prints `bundle_id`, `version`, `scope`, artifact counts, and the discovered capability/event/workflow ids. |
| `bundle register <manifest-path>` | Load a registry bundle and register its contents into in-memory registries. | `cargo run -p traverse-cli -- bundle register examples/expedition/registry-bundle/manifest.json` | Prints `bundle_id`, `version`, `scope`, registered counts, and registration record summaries. |
| `browser-adapter serve [--bind <address>]` | Start the local browser adapter for the governed browser consumer path. | `cargo run -p traverse-cli -- browser-adapter serve --bind 127.0.0.1:4174` | Prints `local browser adapter listening on http://...` and stays running until stopped. |
| `agent inspect <manifest-path>` | Load and summarize a governed WASM agent package manifest. | `cargo run -p traverse-cli -- agent inspect examples/agents/expedition-intent-agent/manifest.json` | Prints `path`, `package_id`, `package_version`, `capability_id`, binary location, digest, and model/workflow references. |
| `agent execute <manifest-path> <request-path>` | Load a governed WASM agent package and execute it against a runtime request. | `cargo run -p traverse-cli -- agent execute examples/agents/expedition-intent-agent/manifest.json examples/agents/runtime-requests/interpret-expedition-intent.json` | Prints `request_id`, `execution_id`, `package_id`, `capability_id`, `trace_ref`, `status`, and capability-specific result fields. |
| `event inspect <contract-path>` | Parse and validate an event contract. | `cargo run -p traverse-cli -- event inspect contracts/examples/expedition/events/expedition-objective-captured/contract.json` | Prints `path`, `id`, `version`, lifecycle, classification, publisher/subscriber counts, and publisher/subscriber ids. |
| `trace inspect <trace-path>` | Parse and summarize a runtime trace artifact. | `cargo run -p traverse-cli -- trace inspect target/traces/plan-expedition.json` | Prints `trace_id`, `execution_id`, `request_id`, governing spec, state-machine validation, state transition counts, and terminal outcome details. |
| `workflow inspect <workflow-path>` | Parse and summarize a workflow definition artifact. | `cargo run -p traverse-cli -- workflow inspect workflows/examples/expedition/plan-expedition/workflow.json` | Prints `id`, `version`, lifecycle, start node, terminal nodes, node/edge counts, and workflow edges. |
| `expedition execute <request-path> [--trace-out <trace-path>]` | Execute the canonical expedition workflow through the Traverse runtime. | `cargo run -p traverse-cli -- expedition execute examples/expedition/runtime-requests/plan-expedition.json --trace-out target/traces/plan-expedition.json` | Prints `request_id`, `execution_id`, `capability_id`, `status`, `trace_ref`, and expedition result fields such as `plan_id` and `summary`. |

## Stable Public Surface

The following command families are intended to be the public documented surface for the current release line:

- `bundle inspect`
- `bundle register`
- `browser-adapter serve`
- `agent inspect`
- `agent execute`
- `event inspect`
- `trace inspect`
- `workflow inspect`
- `expedition execute`

These commands are the ones a downstream developer should rely on when working with Traverse from the terminal.

## Internal Or Test-Only Paths

The following are not CLI commands and should be treated as reference assets, examples, or validation helpers rather than stable user-facing entry points:

- `scripts/ci/*.sh` smoke and validation scripts
- artifact trees under `contracts/examples/`
- workflow trees under `workflows/examples/`
- example package trees under `examples/`
- generated trace artifacts under `target/` or other local output directories

Those paths are useful for testing and documentation, but they are not a promise of a product API.

## Notes On Outputs

- Inspect commands print structured summaries to stdout.
- Execute commands print structured execution summaries to stdout.
- The browser adapter prints a listening address and then remains active.
- Error cases are written to stderr and return a non-zero exit code.

## Validation

This reference was checked against the live CLI behavior with:

- `cargo run -p traverse-cli -- --help`
- `cargo run -p traverse-cli -- help`
- `cargo run -p traverse-cli -- agent --help`
- `cargo run -p traverse-cli -- browser-adapter serve --help`

It should also remain consistent with:

- `bash scripts/ci/repository_checks.sh`

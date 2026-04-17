# Traverse MCP Stdio Server Package

The dedicated Traverse MCP WASM server package is the thin, governed host-facing surface for the app-consumable MCP path.

The packaged MCP server artifact is defined in [docs/packaged-traverse-mcp-server-artifact.md](packaged-traverse-mcp-server-artifact.md).

It is intentionally narrow:

- it stays a façade over Traverse runtime authority
- it uses the canonical expedition registry bundle as its source of truth
- it exposes discovery, description, validation, execution, and execution-report rendering through one stdio command surface
- it is documented and runnable locally

## Supported Bootstrap Path

The supported developer bootstrap path for the dedicated MCP server is:

```bash
cargo run -p traverse-mcp -- stdio
```

That `stdio` command is the only supported bootstrap mode in the current app-consumable release path.

Unsupported bootstrap attempts fail loudly:

- omitting the command prints the usage line and exits non-zero
- using any command other than `stdio` prints `Unsupported command: <command>` and exits non-zero

Developers and agents should treat other bootstrap ideas as unsupported unless they are explicitly documented in this page or in the packaged artifact docs.

## Start The Server

From the repository root:

```bash
cargo run -p traverse-mcp -- stdio
```

To simulate a deterministic startup failure for validation:

```bash
cargo run -p traverse-mcp -- stdio --simulate-startup-failure
```

## Supported Commands

The package emits deterministic JSON envelopes for:

- `describe_server`
- `list_content_groups`
- `describe_content_group`
- `list_entrypoints`
- `describe_entrypoint`
- `validate_entrypoint`
- `execute_entrypoint`
- `render_execution_report`
- `shutdown`

The server reports governed content groups, capabilities, events, and workflows from the canonical expedition bundle.

## Content Groups

The first content group exposed through the dedicated server is the neutral core-runtime example group:

- `core-runtime-example`

It points at the checked-in executable capability package template and local runtime documentation, so clients can discover a Traverse-native content family that is not expedition-specific.

## Validation

Run the deterministic smoke test for the package surface:

```bash
bash scripts/ci/mcp_stdio_server_smoke.sh
bash scripts/ci/mcp_stdio_server_discovery_smoke.sh
bash scripts/ci/mcp_stdio_server_execution_report_smoke.sh
```

Run repository checks:

```bash
bash scripts/ci/repository_checks.sh
```

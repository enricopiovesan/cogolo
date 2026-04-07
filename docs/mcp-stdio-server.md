# Traverse MCP stdio Server Foundation

The first dedicated Traverse MCP server package starts with one governed host mode: `stdio`.

## Start Command

Run the server locally with:

```bash
cargo run -p traverse-mcp -- stdio
```

The command prints machine-readable startup and shutdown envelopes on standard output and stays ready for later governed MCP operations without redefining Traverse runtime authority.

## Deterministic Failure Check

The server also exposes a deterministic startup-failure path for validation:

```bash
cargo run -p traverse-mcp -- stdio --simulate-startup-failure
```

The failure envelope is machine-readable JSON on standard error so validation can assert the package’s startup contract without guessing.

## Discovery and Description

The dedicated stdio server now supports deterministic discovery and description commands:

- `list_entrypoints` returns the governed capability and workflow catalog.
- `describe_entrypoint` returns a machine-readable description for one capability or workflow by id and version.
- `shutdown` exits the server cleanly after emitting a deterministic shutdown envelope.

## Validation

- `bash scripts/ci/mcp_stdio_server_smoke.sh`
- `bash scripts/ci/mcp_stdio_server_discovery_smoke.sh`
- `bash scripts/ci/repository_checks.sh`

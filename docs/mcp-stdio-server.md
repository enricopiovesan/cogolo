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

## Validation

- `bash scripts/ci/mcp_stdio_server_smoke.sh`
- `bash scripts/ci/repository_checks.sh`

# Browser-Hosted `execute_entrypoint` Path — Validation Result

Governed by `023-browser-hosted-mcp-consumer-model`; decision recorded in
[ADR-0044](adr/0044-single-capability-execution-via-mcp-not-embedder.md) and
Decision 60 (`docs/decision-log.md`). Tracks issue `#1100`.

## What this validates

ADR-0044 decided that single-capability execution from a browser-hosted
client should go through the existing `execute_entrypoint` MCP tool over
"whatever non-stdio transport spec `023` already defines for browser-hosted
consumers," rather than extending `BundleEmbedder` to build manifests
client-side. That decision was made by reading spec `023`'s text, not by
independently re-testing the path — ADR-0044's own Consequences section asked
for exactly that follow-up validation. This document is that validation.

## Result: not reachable today, for three independent reasons

### 1. No non-stdio transport exists for the MCP tool surface

`crates/traverse-mcp` implements exactly one host mode. `crates/traverse-mcp/src/main.rs`
accepts only the literal argument `stdio` and rejects anything else:

```
$ cargo run -p traverse-mcp -- http
Unsupported command: http
```

There is no HTTP, WebSocket, or other network-reachable listener anywhere in
`crates/traverse-mcp/src/*.rs` — the entire command surface
(`describe_server`, `execute_entrypoint`, etc.) is implemented as a
newline-delimited JSON protocol over process stdin/stdout
(`crates/traverse-mcp/src/stdio_server.rs`). A browser cannot launch or pipe
to a local process, so it has no way to reach this surface at all today. Spec
`023`'s FR-006 ("the model MUST define the first supported browser-hosted
transport assumption in a way that does not require stdio-only local
execution") describes an intended property, not a shipped transport — no such
transport is implemented in this crate.

### 2. `execute_entrypoint` takes a local filesystem path, not an inline body

`crates/traverse-mcp/src/stdio_server.rs:597-610` (`entrypoint_artifacts`)
requires a `request_path` field and reads it directly off local disk
(`load_runtime_request`); there is no inline-JSON alternative. Even a
same-origin bridge process could not forward a browser-supplied request body
to this tool without a design change, since the tool's only input mode is "a
path this process can `fs::read_to_string`."

### 3. The server is hardwired to one fixed example bundle, not a live registry

`crates/traverse-mcp/src/main.rs` takes no bundle/catalog argument;
`run_stdio_server` always loads `canonical_expedition_bundle_path()`
(`crates/traverse-mcp/src/stdio_server.rs:179,215,1399`). Live-verified:

```
$ printf '{"command":"list_entrypoints"}\n{"command":"shutdown"}\n' | cargo run -p traverse-mcp -- stdio
```

returns exactly the 6 `expedition.planning.*` capabilities, 5 events, and 1
workflow baked into that bundle — never any capability from a live registry
fetch. Attempting `execute_entrypoint` against a real, independently
published capability outside that bundle (`content.comments.create-comment-draft@1.0.0`,
with a schema-valid `runtime_request.json`) fails deterministically:

```
{"code":"not_found","...","message":"capability entrypoint content.comments.create-comment-draft@1.0.0 was not found"}
```

This means `execute_entrypoint`, as implemented today, cannot run "an
arbitrary, live-fetched registry capability" (the motivating discover.html
use case from `#1100`'s origin) even for a co-located, filesystem-capable
caller — the gap is not transport-only.

## What does work, for the record

For the one workflow the fixed bundle does contain
(`expedition.planning.plan-expedition`), `execute_entrypoint` over stdio
succeeds and returns a response shape that looks sufficient for a real
consumer: `execution_id`, `request_id`, `result.output` (the actual planning
output), a redacted `trace` summary, `trace_redaction` policy metadata, and a
full ordered `observation_messages` lifecycle/state stream (`loading_registry`
→ `ready` → `discovering` → ... → `completed`, plus a terminal result event).
If gaps 1-3 above were closed, this response shape would likely satisfy the
"trace-summary sufficient for a real consumer" half of `#1100`'s DoD.

## Conclusion

`#1100`'s validation question — can a browser-hosted client reach
`execute_entrypoint` over spec `023`'s transport today — is answered **no**,
for three independently verified, cited reasons. Per ADR-0044's own
Consequences section, this is filed as a new gap rather than fixed in this
validation ticket: see
[#1105](https://github.com/traverse-framework/traverse/issues/1105).

## Reproduction

```bash
cargo run -p traverse-mcp -- stdio <<'EOF'
{"command":"list_entrypoints"}
{"command":"shutdown"}
EOF
```

# Traverse v0.1 Quickstart

This is the first app-consumable Traverse path for `Foundation v0.1`.

Use it when you want one clear local flow that proves:

- the Traverse runtime can serve the governed local browser adapter
- a browser app can consume the ordered runtime stream through a same-origin local proxy
- the canonical expedition request completes with a deterministic plan result
- the runtime produces a final trace artifact that the app can inspect

## What You Will Run

This quickstart uses the checked-in, approved local path:

- Traverse local browser adapter from `traverse-cli`
- React browser demo from `apps/react-demo`
- canonical expedition planning request
- governed runtime state, trace, and terminal evidence

## Prerequisites

Install the local tools used by the checked-in demo path:

- Rust toolchain with `cargo`
- Node.js

From the repository root:

```bash
cargo test -p traverse-cli
bash scripts/ci/browser_adapter_smoke.sh
bash scripts/ci/react_demo_live_adapter_smoke.sh
```

Those commands prove the local adapter and browser demo path are healthy before you run the app manually.

## Start The Local Browser Adapter

In one terminal, start the governed local browser adapter:

```bash
cargo run -p traverse-cli -- browser-adapter serve --bind 127.0.0.1:4174
```

Expected output includes a line like:

```text
local browser adapter listening on http://127.0.0.1:4174
```

The adapter exposes:

- `POST /local/browser-subscriptions`
- `GET /local/browser-subscriptions/{subscription_id}/stream`

Those endpoints implement the governed transport in [specs/019-local-browser-adapter-transport/spec.md](/private/tmp/cogolo-issue-122/specs/019-local-browser-adapter-transport/spec.md).

## Start The Browser App

In a second terminal, start the checked-in local proxy and React app:

```bash
node apps/react-demo/server.mjs --adapter http://127.0.0.1:4174 --port 4173
```

Open:

- `http://127.0.0.1:4173`

The proxy keeps the app on one local origin while forwarding governed browser adapter requests to the runtime.

## Run The First App-Consumable Flow

In the app:

1. Open the Traverse React demo.
2. Submit the approved expedition request.
3. Wait for the ordered runtime stream to complete.

This path uses the canonical expedition request and should end with a completed planning result.

Expected successful outcome:

- lifecycle updates appear first
- state updates include the governed runtime progression
- a final trace artifact is rendered
- the terminal result reports `status: completed`
- the resulting plan id is `plan-objective-skypilot`

The live demo smoke path checks the same success condition automatically:

```bash
bash scripts/ci/react_demo_live_adapter_smoke.sh
```

## Inspect The Example Surfaces

The quickstart is built on these checked-in example surfaces:

- browser app guide: [apps/react-demo/README.md](/private/tmp/cogolo-issue-122/apps/react-demo/README.md)
- expedition smoke paths: [docs/expedition-example-smoke.md](/private/tmp/cogolo-issue-122/docs/expedition-example-smoke.md)
- first WASM agent example: [docs/wasm-agent-example.md](/private/tmp/cogolo-issue-122/docs/wasm-agent-example.md)
- downstream app MCP validation: [docs/mcp-consumption-validation.md](/private/tmp/cogolo-issue-122/docs/mcp-consumption-validation.md)

If you want to validate the public downstream-consumer path used for `youaskm3`, run:

```bash
bash scripts/ci/mcp_consumption_validation.sh
```

That smoke path verifies the public app-facing MCP surface separately from the browser demo path.

## Known Limitations

This first consumable slice is intentionally narrow:

- it covers one approved expedition flow only
- it assumes the runtime and browser adapter are started locally by the developer
- it uses a local proxy for the browser app instead of a packaged production transport
- it proves one governed browser path and one governed MCP validation path, not general multi-app onboarding
- native demo validation for macOS and Android can still require fuller local platform toolchains

## Next Docs

After this quickstart, the most useful next references are:

- [docs/local-runtime-home.md](/private/tmp/cogolo-issue-122/docs/local-runtime-home.md)
- [docs/adapter-boundaries.md](/private/tmp/cogolo-issue-122/docs/adapter-boundaries.md)
- [docs/compatibility-policy.md](/private/tmp/cogolo-issue-122/docs/compatibility-policy.md)

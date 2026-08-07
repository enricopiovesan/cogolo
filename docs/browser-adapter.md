# Traverse Browser Subscription Explainer

Browser-hosted apps receive governed runtime subscription updates directly
from the production `traverse-cli serve` HTTP/WebSocket API — there is no
separate local browser adapter process. This doc used to describe a
standalone `browser-adapter serve` binary; that binary was retired (Decision
53, issue #973) once the production WebSocket transport grew the same
ordered message contract natively.

It is not a second runtime and it does not redefine Traverse semantics. It is
a subscription mode over already-governed runtime behavior.

## What Browser Subscription Is For

Use the `browser_subscription` WebSocket mode when you want:

- a browser-hosted app to start from the approved app-consumable path
- governed runtime state updates delivered over the same host you already
  call for `execute`/`traces`/etc.
- a live bridge between the browser consumer and the Traverse runtime with no
  extra process to run alongside `traverse-cli serve`

Relevant docs:

- [quickstart.md](../quickstart.md)
- [docs/youaskm3-canonical-app-http-path.md](youaskm3-canonical-app-http-path.md)
- [docs/app-consumable-entry-path.md](app-consumable-entry-path.md)
- [https://github.com/traverse-framework/App-References/tree/main/apps/browser-consumer/README.md](https://github.com/traverse-framework/App-References/tree/main/apps/browser-consumer/README.md)
- [docs/adapter-boundaries.md](adapter-boundaries.md)

## What Browser Subscription Is Not

Browser subscription is not:

- a separate execution model
- a replacement for the core runtime
- a place to redefine subscription message meaning
- a generic deployment abstraction for every host target

The runtime still owns:

- request validation
- execution
- state progression
- trace artifacts
- subscription payload meaning and ordering

The WebSocket transport only owns how those governed surfaces are exposed to
a browser-capable host path.

## How To Connect

1. Open a WebSocket upgrade against `/v1/workspaces/{workspace_id}/apps/{app_id}/events`
   on a running `traverse-cli serve` instance.
2. Send a subscribe message naming exactly one of `request_id`/`execution_id`
   (spec `013-browser-runtime-subscription` FR-001):

   ```json
   {"type": "subscribe", "mode": "browser_subscription", "request_id": "..."}
   ```

3. Receive the governed ordered message sequence: `subscription_established`
   → state → trace → `terminal_result` → `stream_completed`, then the server
   closes the socket.

It should not invent new runtime states, custom message formats, or host-only
execution semantics.

## Request Limits

The WebSocket server accepts inbound messages up to 64 KiB
(`MAX_WS_INBOUND_MESSAGE_BYTES`) and rejects an oversized or malformed frame
with a structured close code rather than crashing the connection handler.

## When To Use It

Use browser subscription when:

- your app is browser-hosted
- you want a live consumer path rather than an offline preview
- you need ordered runtime updates, trace visibility, and terminal results in the UI

Examples:

- the checked-in React demo
- the browser-consumer package
- a downstream shell such as `youaskm3`

## Host-Target Comparison

### Browser Subscription (WebSocket)

Use when:

- the consumer is a browser-hosted app
- you need live subscription updates in the UI
- you are following the approved app-consumable flow

Primary docs:

- [quickstart.md](../quickstart.md)
- [docs/app-consumable-entry-path.md](app-consumable-entry-path.md)
- [docs/youaskm3-canonical-app-http-path.md](youaskm3-canonical-app-http-path.md)

### MCP Stdio Server

Use when:

- the consumer is an MCP client or agent
- discovery and execution should happen through the governed MCP surface
- you do not need the browser subscription transport

Primary docs:

- [docs/mcp-stdio-server.md](mcp-stdio-server.md)
- [docs/mcp-consumption-validation.md](mcp-consumption-validation.md)

### Direct CLI And Authoring Paths

Use when:

- you are developing contracts, workflows, examples, or executable packages
- you need inspection, registration, or local validation flows
- you are not building a browser host surface yet

Primary docs:

- [docs/getting-started.md](getting-started.md)
- [docs/cli-reference.md](cli-reference.md)
- [docs/expedition-example-authoring.md](expedition-example-authoring.md)

### Packaged Runtime And Consumer Bundle

Use when:

- you are integrating Traverse as a release-facing downstream dependency
- you need the published runtime and MCP artifact story, not just source checkout instructions

Primary docs:

- [docs/app-consumable-consumer-bundle.md](app-consumable-consumer-bundle.md)
- [docs/packaged-traverse-runtime-artifact.md](packaged-traverse-runtime-artifact.md)
- [docs/packaged-traverse-mcp-server-artifact.md](packaged-traverse-mcp-server-artifact.md)

## Practical Rule

If your question is “how does a browser app receive governed live runtime updates?”, this document is the right place to start.

If your question is “how does Traverse execute or govern the behavior underneath that stream?”, start with the runtime, app-consumable, or adapter-boundary docs instead.

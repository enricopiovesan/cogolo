# Traverse Architecture: Execution and Consumption Models

Traverse exposes three distinct surfaces for building and consuming capabilities. Understanding how they relate — and when to use each — is essential for designing your integration.

## The Three Models

### 1. WASM Capabilities (Local Execution)

**What**: A capability is a WASM binary compiled from Rust (or any WASM-compatible language) that reads a JSON payload from stdin and writes a JSON result to stdout. It is registered in the capability registry via a bundle manifest and invoked by the runtime's `WasmExecutor`.

**When to use**:
- You are building a new capability (a unit of computation)
- The capability should be portable across execution targets (local, cloud, edge)
- You want governance (spec, contract, digest immutability)

**How it works**:
```
CLI request → PlacementRouter → WasmExecutor → WASM binary (stdin/stdout) → RuntimeTrace
```

**Entry point**: [`docs/wasm-agent-authoring-guide.md`](wasm-agent-authoring-guide.md)

---

### 2. MCP Surface (Agent/LLM Discovery and Invocation)

**What**: The `traverse-mcp` crate exposes a Model Context Protocol server over stdio plus a Rust library surface. It provides tools that LLMs and AI agents can call to discover registered capabilities, inspect their contracts, and execute them — without knowing the CLI.

**MCP tools exposed**:
| Tool | Description |
|------|-------------|
| `discover_capabilities` | List capabilities matching an intent or filter |
| `get_capability` | Inspect a specific capability contract |
| `list_events` | List events in the event catalog |
| `get_event` | Inspect a specific event contract |
| `execute_capability` | Execute a capability by ID with a JSON input |
| `get_trace` | Retrieve a trace by ID |

**When to use**:
- An LLM or AI agent needs to discover what capabilities are available
- You are integrating Traverse with Claude, GPT, or another tool-use enabled model
- You want the model to drive capability selection rather than hard-coding IDs

**How it works**:
```
LLM tool call → MCP stdio server → traverse-mcp → traverse-runtime → WasmExecutor
```

**Entry point**: [`docs/mcp-stdio-server.md`](mcp-stdio-server.md)

**Important**: Capability discovery is contract/package discovery, not host activation. Discovery may report whether a package is standalone or carries advisory workflow composition metadata, but it must not claim the package is activation-eligible without host activation-resolution evidence.

### Direct Invocation vs Application Activation

Standalone capability packages can be inspected and executed directly through `capability-package execute`, MCP `execute_capability`, or the `traverse-mcp` library API when the caller supplies a valid runtime request. Direct invocation uses the registered capability contract and executable artifact metadata.

Governed application activation is a separate host step. `traverse-cli app activate` resolves application-required contracts to host-installed executable artifacts and records immutable activation evidence. That evidence is the only place discovery consumers should treat a package as activation-eligible for an application. Advisory composition metadata such as known workflow references is useful context, but it does not grant authority, select artifacts, or prove host availability.

---

### 3. Browser Subscription (Live Streaming to a Frontend)

**What**: The production server (`traverse-cli serve`) exposes a `browser_subscription` WebSocket mode that streams runtime state events and execution traces to a browser client. It enables a React or web frontend to display live Traverse execution state. This used to be a separate `traverse-cli browser-adapter serve` process; that binary was retired (Decision 53, issue #973) once the production WebSocket transport grew the same ordered message contract natively.

**When to use**:
- You are building a UI that shows live capability execution status
- You want to stream `RuntimeTrace` updates to a browser in real time
- You are building the `youaskm3` shell or a similar consumer app

**How it works**:
```
Browser client → WebSocket upgrade → traverse-cli serve → traverse-runtime subscription → state events
```

**Entry point**: [`docs/browser-adapter.md`](browser-adapter.md)

**Important**: `app_events` mode delivers events only to actively connected clients; there is no replay for late-connecting clients in v0.1 (see [#312](https://github.com/traverse-framework/traverse/issues/312)). `browser_subscription` mode replays a specific execution's already-recorded trace, so connection timing relative to that execution's completion does not affect delivery.

---

## How the Three Models Interact

```
┌─────────────────────────────────────────────────────────────┐
│                     Your Application                        │
│                                                             │
│  ┌──────────┐    ┌─────────────┐    ┌──────────────────┐   │
│  │  CLI /   │    │  MCP tools  │    │  Browser UI      │   │
│  │  Scripts │    │  (LLM use)  │    │  (React/Web)     │   │
│  └────┬─────┘    └──────┬──────┘    └────────┬─────────┘   │
│       │                 │                    │              │
└───────┼─────────────────┼────────────────────┼─────────────┘
        │                 │                    │
        ▼                 ▼                    ▼
┌───────────────────────────────────────────────────────────┐
│                  traverse-runtime                         │
│  PlacementRouter → WasmExecutor → RuntimeTrace            │
│  EventBroker → subscriptions                              │
└───────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────┐
│  WASM Capabilities  │
│  (stdin/stdout JSON)│
└─────────────────────┘
```

All three surfaces drive the same runtime. A CLI invocation, an MCP tool call, and a browser-triggered execution all go through `PlacementRouter` and produce a `RuntimeTrace`.

---

## Decision Guide

| If you are building... | Use |
|------------------------|-----|
| A new capability (unit of computation) | WASM capability + contract |
| An LLM integration that needs to discover and call capabilities | MCP surface |
| A web UI that shows live execution status | Browser adapter |
| A CI pipeline or script that invokes capabilities | CLI (`traverse-cli expedition execute`) |
| An autonomous agent that needs to register and invoke capabilities programmatically | CLI with `--json` (planned, [#305](https://github.com/traverse-framework/traverse/issues/305)) or MCP |
| A multi-capability workflow | Workflow contract + registry traversal |

---

## Related Docs

- [`docs/wasm-agent-authoring-guide.md`](wasm-agent-authoring-guide.md) — write a WASM capability
- [`docs/mcp-stdio-server.md`](mcp-stdio-server.md) — MCP server setup
- [`docs/browser-adapter.md`](browser-adapter.md) — browser subscription and streaming
- [`docs/workflow-composition-guide.md`](workflow-composition-guide.md) — chain capabilities
- [`quickstart.md`](../quickstart.md) — first browser-consumption flow

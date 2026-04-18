# Traverse v0.1 Quickstart

This quickstart documents the first app-consumable Traverse flow: a browser-hosted app consuming Traverse through the live local browser adapter and the checked-in React demo surface.

It is intentionally narrow. Use this path when you want one approved end-to-end consumer flow from setup through terminal outcome.

## What This Covers

- the Traverse runtime host
- the live local browser adapter proxy
- the React browser demo
- one approved expedition request
- ordered runtime updates, trace evidence, and terminal output

## Prerequisites

- A working Rust toolchain
- Node.js 20 or later
- Two terminals
- The repository checked out with the approved browser adapter and browser demo implementation already merged

## Start The Live Browser Adapter

From the repository root:

```bash
cargo run -p traverse-cli -- browser-adapter serve --bind 127.0.0.1:4174
```

Keep this terminal open. The React demo proxies browser-subscription traffic through this local adapter.

## Start The React Browser Demo

In a second terminal from the repository root:

```bash
node apps/react-demo/server.mjs --adapter http://127.0.0.1:4174 --port 4173
```

Open:

- `http://127.0.0.1:4173`

## Run The Approved Consumer Flow

The demo is preloaded with the approved expedition request. Click **Submit approved request**.

The approved request payload is:

```json
{
  "goal": "Plan a two-day alpine expedition for a four-person team.",
  "requested_target": "local",
  "caller": "browser_demo"
}
```

## What You Should See

- the status pill moves from `ready` to `streaming` and then `completed`
- ordered runtime updates appear in the timeline
- the terminal trace panel stays hidden until the stream completes
- after completion, the trace panel shows the selected capability, emitted events, and final output

The expected final consumer outcome is a completed expedition plan with the governed trace snapshot visible in the app.

## Known Limitations

- This is the first supported consumer path, not a production deployment guide.
- The browser app must proxy through the local browser adapter to use the live path.
- The fallback static preview path in `apps/react-demo/README.md` is useful for offline inspection, but it is not the app-consumable v0.1 path.
- This quickstart does not redefine Traverse internals; it only documents the governed consumer flow that is already checked in.

## Validation

Run the live consumer path smoke test:

```bash
bash scripts/ci/react_demo_live_adapter_smoke.sh
```

Run the fallback static preview smoke test:

```bash
bash scripts/ci/react_demo_smoke.sh
```

Run repository checks:

```bash
bash scripts/ci/repository_checks.sh
```

If one of those commands fails and you need the shortest diagnosis path, use [docs/troubleshooting.md](docs/troubleshooting.md).

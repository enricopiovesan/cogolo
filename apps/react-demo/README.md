# Traverse React Demo

This is the first checked-in React browser demo surface for Traverse.

If you want the shortest end-to-end local path, start with [quickstart.md](/Users/piovese/Documents/cogolo/quickstart.md) and then come back here for the demo-specific details.
If you want the formal acceptance entrypoint, use [docs/app-consumable-acceptance.md](/Users/piovese/Documents/cogolo/docs/app-consumable-acceptance.md).

What it does:
- renders one approved expedition flow
- allows one approved request submission path
- shows ordered runtime state updates streamed from the live local browser adapter
- shows the final trace snapshot and output panel after the stream completes
- keeps the approved browser-subscription session shape while consuming the live adapter through a same-origin local proxy

Local live run path:

```bash
cargo run -p traverse-cli -- browser-adapter serve --bind 127.0.0.1:4174
node apps/react-demo/server.mjs --adapter http://127.0.0.1:4174 --port 4173
```

Open:

- `http://127.0.0.1:4173`

Note:

- the React app consumes the live browser adapter through the local proxy server
- if the local adapter cannot be started automatically, use the fallback preview path below
- Run the local browser adapter proxy again if you need to refresh the live stream setup

Fallback preview path:

```bash
python3 -m http.server 4173 --directory apps/react-demo
```

The fallback preview keeps the checked-in fixture-driven render path available for offline inspection and smoke validation.

For the first app-consumable quickstart that uses the live browser adapter path, see [../../quickstart.md](/Users/piovese/Documents/cogolo/quickstart.md).

Live smoke path:

```bash
bash scripts/ci/react_demo_live_adapter_smoke.sh
```

Fallback smoke path:

```bash
bash scripts/ci/react_demo_smoke.sh
```

Fixture source:

- `apps/react-demo/public/expedition-runtime-session.json`

Runtime assets:

- `apps/react-demo/vendor/react.development.js`
- `apps/react-demo/vendor/react-dom.development.js`

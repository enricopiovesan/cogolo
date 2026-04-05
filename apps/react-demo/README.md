# Traverse React Demo

This is the first checked-in React browser demo surface for Traverse.

What it does:
- renders one approved expedition flow
- allows one approved request submission path
- shows ordered runtime state updates
- shows the final trace snapshot and output panel after the stream completes
- uses the approved browser-subscription session shape through a deterministic fixture-driven rendering path

Local run path:

```bash
python3 -m http.server 4173 --directory apps/react-demo
```

Open:

- `http://127.0.0.1:4173`

Note:

- the documented local run path uses a simple static file server and works on a normal local machine
- the checked-in repo smoke path stays static and does not bind a local port, because the sandboxed validation environment blocks local socket binding

Local smoke path:

```bash
bash scripts/ci/react_demo_smoke.sh
```

Fixture source:

- `apps/react-demo/public/expedition-runtime-session.json`

Runtime assets:

- `apps/react-demo/vendor/react.development.js`
- `apps/react-demo/vendor/react-dom.development.js`

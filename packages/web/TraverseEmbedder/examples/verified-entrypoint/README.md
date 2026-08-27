# Verified entrypoint browser demo

This is the browser proof for issue #1158. It invokes one exact published
capability only through `POST /v1/entrypoints/execute` (Spec 115). The page
does not download, accept, or execute an artifact; the separately started
server must already be configured with verified registry state and verified
materialized artifact state (Specs 118 and 120).

Build the package and serve this page:

```bash
npm run build
node examples/verified-entrypoint/server.mjs
```

Open `http://127.0.0.1:4176`, enter the exact public capability id/version,
and paste its matching `RuntimeRequest` JSON. The result panel displays the
server's result and trace receipt. Verification failures and invalid input are
rendered as safe error envelopes.

The host must be started independently with its explicit verified state, for
example:

```bash
traverse-cli serve --registry-state /srv/traverse/registry-state.json \
  --artifact-state /srv/traverse/artifact-state.json
```

This example intentionally remains separate from `react-integration/`, which
is the existing no-sidecar bundled-WebAssembly proof governed by Spec 068.

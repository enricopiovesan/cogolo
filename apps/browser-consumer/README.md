# Traverse Browser Consumer Package

This is the browser-targeted consumer package for downstream apps such as `youaskm3`.

It stays focused on the first governed browser-hosted path:

- it reuses the approved live browser adapter client
- it exposes a browser-safe subscription flow for the downstream app
- it keeps runtime ordering, trace visibility, and terminal outcomes governed by Traverse surfaces rather than private app logic
- it does not define UI, branding, or app-specific product behavior

## Supported Use

The package is intended to be consumed by a browser-hosted app that can bundle the façade and point it at the live browser adapter proxy.

It provides:

- a browser consumer session shape
- approved subscription request construction
- live subscription execution
- ordered browser subscription message application
- terminal trace summarization

## Browser-Hosted Assumptions

- The browser-hosted app has access to the approved live browser adapter proxy path.
- The browser-hosted app uses the published consumer façade rather than internal Traverse crate layout.
- The browser-hosted app relies on governed runtime updates, trace visibility, and terminal outcomes from Traverse.
- Unsupported auth, remote deployment, and multi-tenant guarantees remain out of scope for the first slice.

## Quick Start

```bash
node -e "const client = require('./apps/browser-consumer'); console.log(client.APPROVED_BROWSER_CONSUMER_SESSION.title)"
```

For the deterministic browser-hosted smoke path, see [../../scripts/ci/browser_consumer_package_smoke.sh](/Users/piovese/Documents/cogolo/scripts/ci/browser_consumer_package_smoke.sh).

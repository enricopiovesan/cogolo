# ADR-0044: Single-Capability Browser Execution Goes Through `execute_entrypoint`, Not an Extended BundleEmbedder

- Status: Accepted
- Governing specs: `023-browser-hosted-mcp-consumer-model`; related `006-runtime-request-execution`, `010-runtime-state-machine`, `068-public-platform-embedder-packages`
- Related issues: #1100, #1097, #1098, #865

## Context

`#1100` found that running an arbitrary, live-fetched registry capability
directly in a browser requires assembling a `manifest.json` (`agent_package`,
with an FNV-1a `expected_digest`) and a `runtime-request.json` before
anything executes — real, enforced ceremony, not optional metadata — and
that `catalog.json` doesn't expose a ready digest to build one from today.
The natural-seeming fix was to extend `BundleEmbedder`
(`traverse-embedder-web`, spec `068`) or add a browser-side helper to
compute the digest and synthesize a manifest client-side.

`traverse-mcp` already exposes an `execute_entrypoint` tool that performs
exactly this manifest/digest/binary resolution server-side and returns a
public trace summary. The ceremony `#1100` was proposing to duplicate in
the browser is already solved once — just not on a path a browser-hosted
client can reach outside of `stdio`.

## Decision

Do not extend `BundleEmbedder` to construct manifests or digests
client-side. Single-capability execution from a browser-hosted client goes
through the existing `execute_entrypoint` MCP tool, called over whatever
non-stdio transport `023` already defines for browser-hosted consumers
(FR-006) — the manifest/digest/binary-resolution ceremony stays
server-side, where it already lives, instead of being duplicated in the
browser. `BundleEmbedder` keeps its existing, narrower job: executing a
pre-vetted, already-reviewed application bundle (spec `044`) shipped with
the app itself, not fetching and running arbitrary live registry content
chosen at runtime. Once `#1098`'s planner (spec `113`) and `109`'s P1
lifecycle produce a submittable, approved multi-step proposal, the same
reasoning applies: execution runs through P1's existing MCP surface, not a
browser-assembled bundle.

This requires no new governing spec. Spec `023` already defines a
non-stdio browser-hosted transport in general terms (FR-006) without
naming or restricting which MCP tools are reachable over it, and
`execute_entrypoint` is one of the ordinary tools `traverse-mcp` exposes —
there is no separate allowlist excluding it from browser-hosted
consumption. What remains is implementation and validation work, not new
governance: confirming end-to-end that a browser-hosted client can
actually reach `execute_entrypoint` over spec `023`'s approved transport,
and that the response is sufficient for a consumer (e.g. discover.html,
`youaskm3`) to show a real result.

## Consequences

`#1100` is rescoped from "extend the embedder with manifest/digest
ceremony" to "validate the already-approved browser-hosted MCP execute
path end-to-end" — an integration/validation ticket, not a spec-needing
one. Should validation surface a real gap (spec `023`'s transport not
actually carrying `execute_entrypoint` calls, or trace-summary redaction
omitting something a browser consumer needs), that is a new finding to
file on its own, since it was not independently re-tested when this ADR
was written.

## Alternatives considered

- Extend `BundleEmbedder` / add a browser-side `resolveManifest(capabilityRef)`
  helper that fetches WASM bytes and computes the FNV-1a digest
  client-side: rejected — duplicates the manifest/digest ceremony in two
  places (server enforcement plus client construction) instead of one, and
  would need the registry to expose a raw digest field it doesn't have
  today.
- Have the registry publish a ready-made `manifest.json` per capability
  version, keeping `BundleEmbedder`'s flow but removing client-side digest
  computation: not rejected outright, but deferred — a registry-side
  change with its own review, not something this browser-execution
  decision should presuppose; revisit only if `execute_entrypoint` proves
  insufficient.
- Author a new spec to formally govern browser-hosted access to
  `execute_entrypoint`: rejected — spec `023` already covers this
  generically (transport and packaging boundary, no tool-level allowlist),
  and a parallel spec re-covering the same surface risks the drift
  Decision 58 already warned against.

# Traverse MCP Mode A — Release & Provenance Evidence

Governed by spec [`119-verified-registry-mcp-mode-a`](../specs/119-verified-registry-mcp-mode-a/spec.md) (FR-006).

Mode A is the product-facing local MCP host for LLM façades (Claude Desktop,
Cursor, and compatible stdio clients). It ships as a **versioned, standalone
`traverse-mcp` binary** that a client pins by version and verifies by checksum
and provenance — without a Rust toolchain or source checkout. Source-run
workflows (`cargo run -p traverse-mcp -- stdio`) remain contributor-only.

## Release form

The Mode A binary is released through the packaged MCP server artifact path
defined in
[docs/packaged-traverse-mcp-server-artifact.md](packaged-traverse-mcp-server-artifact.md)
and the downstream publication strategy in
[specs/023-downstream-publication-strategy/spec.md](../specs/023-downstream-publication-strategy/spec.md).
Each release provides:

- a version tag matching the workspace version (see
  [docs/release-process.md](release-process.md))
- the `traverse-mcp` binary for each supported host target
- a published SHA-256 checksum for every binary
- provenance evidence (the build's `cargo build --locked` invocation and the
  release tag) carried in the supply-chain attestation produced by
  `scripts/ci/supply_chain_check.sh`

## Client pin and verification

A local client configuration pins the exact released version and verifies the
downloaded binary before first launch:

1. Record the pinned version, e.g. `traverse-mcp 0.11.0`.
2. Download the binary for the host target and its published `.sha256`.
3. Recompute and compare: `shasum -a 256 traverse-mcp` must equal the published
   digest.
4. Confirm the provenance attestation names the same version tag and the
   `cargo build --locked` build invocation.
5. Launch with the verified registry cache the host prepared:

   ```bash
   TRAVERSE_MCP_REGISTRY_CACHE=/path/to/verified-registry-cache \
     traverse-mcp stdio
   ```

Discovery and execution then run entirely from the prepared, digest-verified
state — see [docs/mcp-stdio-server.md](mcp-stdio-server.md) for the command
surface.

## v0.1.0 posture

The Spec 080 verified registry cache that Mode A consumes is digest-based and
carries no per-artifact signature, so the runtime executes verified WASM in
development security mode with the artifact digest checked twice (cache resolve
and WASM-executor checksum). Adding an Ed25519 / Sigstore signature layer to the
Mode A execution path is a tracked follow-up and does not change the pin-and-
verify release evidence above.

## Verification

```bash
bash scripts/ci/mcp_stdio_server_mode_a_smoke.sh
```

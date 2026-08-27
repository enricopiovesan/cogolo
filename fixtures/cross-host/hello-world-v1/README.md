# Cross-host hello-world fixture v1

This offline, repository-controlled fixture is the shared corpus for the
browser, CLI/Node, and native conformance runners. It implements the comparison
boundary in [`docs/cross-host-fixture-comparison-contract.md`](../../../docs/cross-host-fixture-comparison-contract.md).

## Pinned sources

The fixture uses the deterministic `hello.world.say-hello` capability. The
source artifact and contract are checked into this repository and are identified
by the SHA-256 digests in `fixture.json`; a runner must verify them before it
uses either source. Rebuild the artifact locally with:

```bash
bash examples/hello-world/say-hello-agent/build-fixture.sh
shasum -a 256 examples/hello-world/say-hello-agent/artifacts/say-hello-agent.wasm
```

The artifact uses no network, filesystem, or host API access. Its output is
deterministic and contains only the supplied public fixture value, making it
suitable for public CI.

## Cases

- `valid-execution.json` verifies the canonical result and lifecycle
  projections for one successful invocation.
- `invalid-input.json` omits the contract-required `name`; a runner must reject
  it before successful capability execution.
- `artifact-identity-failure.json` deliberately supplies a non-matching
  observed digest; a runner must reject it before execution.

The files contain only fixture inputs and expected safe projections. Runners
must not place raw input or output payloads, credentials, headers, local paths,
or opaque trace data in their evidence records.

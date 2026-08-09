# Publishing a capability artifact

`traverse-cli capability publish` creates a human-reviewed registry PR; it
never publishes a capability merely by running locally. The command also
uploads the supplied WASM artifact to a GitHub Release in the target registry
repository, then writes its SHA-256 digest and immutable release URL into the
generated registry `contract.json`.

Run it from a clean checkout of the registry repository:

```bash
traverse-cli capability publish \
  --contract contracts/my-capability/contract.json \
  --artifact target/wasm32-wasip1/release/my-capability.wasm \
  --registry-repo ../registry \
  --json
```

The release tag is `artifacts/<capability-id>-<version>`. The JSON evidence
includes the computed `artifact_digest`, `artifact_release_tag`, and
`artifact_url`, along with the registry PR URL after it is opened.

Use `--dry-run` to validate the contract and artifact and print the exact tag,
URL, digest, branch, and registry path without uploading an asset or changing
the registry checkout. Dry-run and publish both resolve each
`use_cases[].persona_ref` against the target registry checkout's
`personas/<id>/<version>/persona.json` tree and fail fast with
`capability_publish_persona_ref_unresolved` when any referenced persona is
missing. Publish also fails with actionable JSON evidence when GitHub
authentication cannot create the release, the artifact filename cannot safely
form a release URL, or a version's registry path already exists. If a later
branch or PR step fails, the reported partial state identifies the release tag
and registry branch left for retry or explicit cleanup.

Before either dry-run or real publish proceeds, every non-empty
`use_cases[].persona_ref` must resolve to a directory under `personas/` in the
target registry checkout. Missing IDs are reported together with the expected
`personas/<id>/<version>/persona.json` shape, so authors can add the governed
persona before opening a registry PR.

The resulting registry PR still requires registry CI and explicit human review
before the record can become visible to consumers.

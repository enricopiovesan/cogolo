# Publishing `traverse-embedder-web` to npm

Governed by `068-public-platform-embedder-packages`. Tracks issue `#1097`.

`packages/web/TraverseEmbedder` is a complete, versioned, conformance-tested
implementation of `embedder-api/1.0.0` — it builds cleanly, all 42 package
tests pass, and it already satisfies spec 068's package-content requirements
(FR-006 test double, FR-007 documentation, FR-008 release evidence via
`releaseEvidence()`, FR-009 conformance corpus — see `npm run build && npm run test`
and `scripts/ci/embedder_conformance/web_package.sh`). The only remaining gap is
distribution: it has never been published, so nothing outside this repo can
`npm install` it.

Publishing is an irreversible, externally-visible action against a
third-party registry. This repo's automation does not perform it — this
runbook prepares everything up to that point and hands the actual publish
command to whoever holds npm publish rights for the intended package name.

## Preflight checklist (verified in this PR)

- [x] `npm run build` compiles cleanly (`tsc`, no errors).
- [x] `npm test` passes (42/42), including real WASI capability execution and
      a full checked-in `traverse-starter` bundle run.
- [x] `npm pack --dry-run` produces the expected tarball: `dist/**`,
      `README.md`, `LICENSE`, `package.json` (24 → 25 files after adding
      `LICENSE` to `files` in this PR).
- [x] `prepublishOnly` now runs `build` automatically so a stale/missing
      `dist/` can never ship (added in this PR — previously `npm publish`
      would have shipped whatever `dist/` happened to exist locally).
- [x] Name availability re-checked live on 2026-08-22:
      `npm view traverse-embedder-web` → `404 Not Found` (unclaimed, matches
      the package.json `name` field already committed).

## What is NOT verified here

- Whether `traverse-framework` (or whoever publishes) has an npm
  organization/account with the intended publish identity already set up.
- Whether `traverse-embedder-web` is the final desired public package name
  versus a scoped alternative (e.g. `@traverse-framework/embedder-web`) —
  the unscoped name in `package.json` was already a deliberate prior choice,
  kept as-is here rather than relitigated.
- CI-based publish automation (a GitHub Actions workflow gated on a tag or
  manual dispatch, using an `NPM_TOKEN` secret) — not built in this PR; ask
  for it explicitly as a follow-up if wanted, since it requires adding a
  repository secret this session cannot create.

## To actually publish (run this yourself, with your own npm credentials)

```bash
cd packages/web/TraverseEmbedder
npm login                 # if not already authenticated for this identity
npm publish --access public
```

`--access public` is required the first time for an unscoped package name
under an account without a default-private setting.

## Verify after publishing

```bash
npm view traverse-embedder-web
npm install traverse-embedder-web   # from a scratch directory, outside this repo
```

Confirm the installed package's `dist/index.js` exports match
`packages/web/TraverseEmbedder/dist/index.js` from this build, and that
`releaseEvidence()` reports the expected version/digests.

# Traverse v0.1 Published Artifact Consumption Validation for `youaskm3`

This document defines the deterministic validation path that proves `youaskm3` can consume the published Traverse v0.1 runtime and MCP artifacts rather than only local source builds.

It is the published-artifact companion to the existing downstream integration validation and release-prep docs.

## Governing Specs

- `specs/019-downstream-consumer-contract/spec.md`
- `specs/020-downstream-integration-validation/spec.md`
- `specs/021-app-facing-operational-constraints/spec.md`
- `specs/023-downstream-publication-strategy/spec.md`

## Purpose

Use one deterministic repo-local validation flow to prove that `youaskm3` can follow the published Traverse v0.1 release bundle, the packaged runtime artifact, and the packaged MCP server artifact without reverting to repo archaeology or source-only setup.

## Validation Path

Run the release artifact smoke checks first:

```bash
bash scripts/ci/packaged_traverse_runtime_artifact_smoke.sh
bash scripts/ci/packaged_traverse_mcp_server_artifact_smoke.sh
```

Then run the published-artifact validation wrapper:

```bash
bash scripts/ci/youaskm3_published_artifact_validation.sh
```

## Expected Evidence

The validation path should prove:

- `Traverse v0.1.0` is the release pointer used by the first consumer bundle
- the packaged runtime artifact is part of the published release set
- the packaged MCP server artifact is part of the published release set
- `consumer_name: youaskm3`
- `validated_flow_id: youaskm3_published_artifact_validation`
- the validation path refers to published artifacts instead of only local source builds

## Known Failure Modes

The path is expected to fail deterministically when:

- the packaged runtime artifact doc is missing or unlinked
- the packaged MCP server artifact doc is missing or unlinked
- the package release pointer is missing or unlinked
- the downstream validation docs still only describe source-checkout setup
- the published-artifact validation wrapper is missing

## Validation

- `bash scripts/ci/packaged_traverse_runtime_artifact_smoke.sh`
- `bash scripts/ci/packaged_traverse_mcp_server_artifact_smoke.sh`
- `bash scripts/ci/youaskm3_published_artifact_validation.sh`
- `bash scripts/ci/repository_checks.sh`

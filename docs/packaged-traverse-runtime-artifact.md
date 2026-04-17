# Traverse v0.1 Packaged Runtime Artifact

This document defines the first governed packaged Traverse runtime artifact for downstream consumers.

The published runtime artifact is the runtime-facing release form that downstream consumers can obtain and run without relying on source checkout details or repo archaeology.

The downstream publication strategy for packaged Traverse runtime and MCP artifacts is defined in [specs/023-downstream-publication-strategy/spec.md](../specs/023-downstream-publication-strategy/spec.md).

## Publication Shape

The first packaged Traverse runtime release is published as:

- one versioned runtime artifact bundle
- one release note that explains how to obtain and run the runtime artifact
- one release-facing pointer to the approved downstream publication strategy

The runtime artifact bundle is a governed publication record, not a new runtime behavior layer.

## Bundle Contents

The first runtime artifact bundle MUST include:

- the packaged runtime artifact reference
- the release checklist reference
- the versioned consumer bundle reference
- the downstream publication strategy reference
- the app-consumable quickstart reference
- the runtime-related validation evidence reference

## Downstream Use

Downstream consumers SHOULD treat the packaged runtime artifact as the canonical artifact form for consuming Traverse runtime behavior in the v0.1 app-consumable path.

Downstream consumers SHOULD pair the runtime artifact with the versioned consumer bundle and release checklist rather than reconstructing the runtime from source-only instructions.

## Verification

A reviewer can verify the packaged runtime artifact with:

```bash
bash scripts/ci/packaged_traverse_runtime_artifact_smoke.sh
```

That check confirms the runtime artifact doc exists and is linked from the release-facing docs.

## Known Limits

This artifact does not claim:

- a general-purpose package registry workflow
- automatic publication without manual approval
- support for runtime forms outside the first app-consumable release path

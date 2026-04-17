# Traverse v0.1 Packaged MCP Server Artifact

This document defines the first governed packaged Traverse MCP server artifact for downstream consumers.

The published MCP server artifact is the MCP-facing release form that downstream consumers can obtain and run without relying on source checkout details or repo archaeology.

The downstream publication strategy for packaged Traverse runtime and MCP artifacts is defined in [specs/023-downstream-publication-strategy/spec.md](../specs/023-downstream-publication-strategy/spec.md).

## Publication Shape

The first packaged Traverse MCP server release is published as:

- one versioned MCP server artifact bundle
- one release note that explains how to obtain and run the MCP server artifact
- one release-facing pointer to the approved downstream publication strategy

The MCP server artifact bundle is a governed publication record, not a new runtime behavior layer.

## Bundle Contents

The first MCP server artifact bundle MUST include:

- the packaged MCP server artifact reference
- the release checklist reference
- the versioned consumer bundle reference
- the downstream publication strategy reference
- the MCP consumption validation reference
- the real downstream shell validation reference

## Downstream Use

Downstream consumers SHOULD treat the packaged MCP server artifact as the canonical artifact form for consuming Traverse MCP behavior in the v0.1 app-consumable path.

Downstream consumers SHOULD pair the MCP server artifact with the versioned consumer bundle and release checklist rather than reconstructing the MCP server from source-only instructions.

## Verification

A reviewer can verify the packaged MCP server artifact with:

```bash
bash scripts/ci/packaged_traverse_mcp_server_artifact_smoke.sh
```

That check confirms the MCP server artifact doc exists and is linked from the release-facing docs.

## Known Limits

This artifact does not claim:

- a general-purpose package registry workflow
- automatic publication without manual approval
- support for MCP forms outside the first app-consumable release path

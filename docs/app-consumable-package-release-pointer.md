# Traverse v0.1 App-Consumable Package Release Pointer

This document defines the first governed package release pointer for the app-consumable Traverse release.

It exists so downstream app consumers can identify the first governed package publication path without needing to reverse-engineer the repo or infer package identity from source-only templates.

## Publication Shape

The first governed package publication path is represented as:

- one versioned package release pointer for the Traverse v0.1 app-consumable path
- one release bundle definition that links the pointer to the supported consumer surfaces
- one human-readable note that explains how the published artifact should be identified downstream

This pointer is a governed publication record, not a new runtime behavior layer.

## Pointer Contents

The pointer MUST reference:

- the versioned consumer bundle
- the release artifact and publication bundle definition
- the app-consumable quickstart
- the downstream MCP consumption validation path
- the first real `youaskm3` integration validation path
- the real browser-hosted `youaskm3` shell validation path

## Downstream Use

Downstream consumers SHOULD use this pointer together with the approved Traverse v0.1 release tag or equivalent release pointer when identifying the first app-consumable release set.

The pointer does not replace the release checklist or the release artifact definition; it ties those artifacts together in one published reference that can be reviewed without source archaeology.

## Verification

A reviewer can verify the first package release pointer with:

```bash
bash scripts/ci/app_consumable_package_release_pointer.sh
```

That check confirms the pointer doc exists and is linked from the release-facing docs.

## Known Limits

This pointer does not claim:

- a full public package registry workflow
- automated publication without manual approval
- broader package families beyond the first app-consumable release path

# Traverse Spec Numbering And Versioning

Traverse spec identifiers are governance IDs, not a promise that the `specs/` directory will always look strictly sequential.

This document explains why the numbering looks uneven, why some directory prefixes are higher than their approved spec IDs, and how to choose the next spec ID without creating more confusion.

## The Short Version

- the authoritative spec registry is [specs/governance/approved-specs.json](../specs/governance/approved-specs.json)
- the canonical spec identity is the registry `id` field such as `015-capability-discovery-mcp`
- the `specs/<folder>/` directory name is the storage path for that slice, not the only thing that defines its canonical identity
- approved specs are immutable once merged, so Traverse prefers keeping existing history stable over renumbering old slices to make the tree look tidy

## Why The Numbers Are Not Strictly Sequential

Traverse accumulated specs in several waves:

1. the original foundation slices, which produced the early `001` to `011` range
2. later app-consumable, browser, MCP, and WASM slices added after the repo already had working governance history
3. a later branch of work that used higher folder prefixes such as `204`, `205`, `209`, and `212` during drafting, then kept those directory paths after the canonical approved IDs were registered

Because approved specs are treated as immutable governance artifacts, Traverse does not renumber old merged slices just to make the folder list prettier.

## Canonical Identity vs Directory Path

The approved registry is the source of truth.

For example:

- canonical approved id: `024-placement-constraint-evaluator`
- current path: `specs/204-placement-constraint-evaluator/spec.md`

and:

- canonical approved id: `015-capability-discovery-mcp`
- current path: `specs/209-capability-discovery-mcp/spec.md`

This means:

- use the registry `id` in PR bodies, issue bodies, code comments, and spec-alignment declarations
- use the registry `path` when locating the actual files on disk
- do not assume the folder prefix and approved ID prefix are always the same

## How Versioning Works

Each approved entry in [specs/governance/approved-specs.json](../specs/governance/approved-specs.json) records:

- `id`
- `version`
- `status`
- `immutable`
- `path`
- `governs`

Today, approved slices use `version: 1.0.0` because Traverse treats the merged approved artifact as the immutable baseline for that slice.

In practice:

- the spec `id` identifies the governance slice
- the `version` identifies the approved revision of that slice
- the `path` tells the tooling where the files live

## How To Read The Current Tree

When the tree looks odd, interpret it like this:

- low-numbered folders such as `001` to `023` mostly reflect early direct numbering
- missing folders do not mean the spec is missing; they usually mean the numbering was reserved, superseded, or represented under a different preserved path
- higher-numbered folders such as `204` to `212` are preserved historical storage paths for approved slices whose canonical governance IDs are `024` to `028`, `012`, `014`, `015`, `016`, and `027`

If there is any doubt, trust the registry over the folder name.

## How To Choose A New Spec ID

When creating a new spec:

1. check [specs/governance/approved-specs.json](../specs/governance/approved-specs.json)
2. choose the next unused canonical governance ID, not just the next directory number you happen to see
3. keep the chosen ID stable in the spec body, PR body, and issue references
4. register the new spec in the approved registry as part of the governed merge flow

Do not renumber older approved slices just to fill gaps.

## Contributor Rule

If a human or agent needs to know which spec governs a change:

- first read the approved registry
- then open the `path` recorded there
- only then reason about the local folder layout

That order avoids nearly all numbering confusion.

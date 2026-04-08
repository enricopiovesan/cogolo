# Traverse Planning Board

This document is the local planning view for MVP work and mirrors the active backlog in GitHub Project 1.

Default operating model:

1. Delivery lane A: work the highest-priority `Ready` Project 1 item and open one PR for it
2. Delivery lane B: work the next `Ready` Project 1 item only when its write scope is sufficiently separate
3. PR stewardship lane: keep open PRs green, fix blockers, and merge them
4. PM / PO / scrum-master lane: keep backlog tickets, labels, states, notes, and project items accurate

These four lanes should run continuously and stay in their own scope.

Project 1 status meanings:

- `Ready`: can be implemented now under approved specs and current repo rules
- `In Progress`: currently being worked on in an active issue or pull request
- `Blocked`: should not start yet; the item must state why it is blocked
- `Needs Spec`: implementation must not start because the governing slice is not approved yet
- `Needs Enrico`: blocked on product or governance direction from Enrico
- `No Spec Needed`: the work is artifact authoring, docs, or another task already fully governed by approved specs
- `Future`: valid MVP-following work that is tracked but not part of the current active slice

## Active Backlog

### `In Progress`

Only tickets with real active execution should appear in this section.

- none at the moment

### `Ready`

- [#121](https://github.com/enricopiovesan/Traverse/issues/121) `Upgrade React browser demo to consume the live local browser adapter`
  - area: `runtime`
  - status: available now that [#120](https://github.com/enricopiovesan/Traverse/issues/120) is merged
  - done when: the browser demo uses the live adapter path instead of the checked-in fixture flow

- [#129](https://github.com/enricopiovesan/Traverse/issues/129) `Validate the first app-facing MCP consumption path for downstream apps`
  - area: `runtime`, `quality`
  - status: available now that [#109](https://github.com/enricopiovesan/Traverse/pull/109) and [#126](https://github.com/enricopiovesan/Traverse/issues/126) are merged
  - done when: one downstream MCP-consumption path is documented and validated against public Traverse surfaces

- [#53](https://github.com/enricopiovesan/Traverse/issues/53) `Implement second WASM AI agent example`
  - area: `runtime`
  - status: available now that the first agent pattern from [#54](https://github.com/enricopiovesan/Traverse/issues/54) is merged
  - done when: a second distinct governed AI agent example lands with deterministic validation and runnable docs

## Blocked Backlog

### `Blocked` + `No Spec Needed`

- [#121](https://github.com/enricopiovesan/Traverse/issues/121) `Upgrade React browser demo to consume the live local browser adapter`
  - moved to `Ready`

### `Blocked` + `Future`

- [#122](https://github.com/enricopiovesan/Traverse/issues/122) `Write the first app-consumable quickstart for Traverse v0.1`
  - blocked by: [#120](https://github.com/enricopiovesan/Traverse/issues/120) and [#121](https://github.com/enricopiovesan/Traverse/issues/121)
  - unblock path: document the real live setup and app-consumption flow once the live browser demo exists

- [#123](https://github.com/enricopiovesan/Traverse/issues/123) `Add end-to-end acceptance validation for the first app-consumable flow`
  - blocked by: [#121](https://github.com/enricopiovesan/Traverse/issues/121) and [#122](https://github.com/enricopiovesan/Traverse/issues/122)
  - unblock path: prove the first app-consumable path end to end after the live demo and quickstart are in place

- [#128](https://github.com/enricopiovesan/Traverse/issues/128) `Validate the first real youaskm3 integration path against Traverse`
  - blocked by: [#121](https://github.com/enricopiovesan/Traverse/issues/121) and [#122](https://github.com/enricopiovesan/Traverse/issues/122)
  - unblock path: complete the governed browser-consumer path and quickstart first

- [#127](https://github.com/enricopiovesan/Traverse/issues/127) `Prepare the Traverse v0.1 release checklist for app consumers`
  - blocked by: [#123](https://github.com/enricopiovesan/Traverse/issues/123), [#126](https://github.com/enricopiovesan/Traverse/issues/126), and [#128](https://github.com/enricopiovesan/Traverse/issues/128)
  - unblock path: finish the real integration validation and MCP validation evidence before turning release readiness into a governed checklist

- [#129](https://github.com/enricopiovesan/Traverse/issues/129) `Validate the first app-facing MCP consumption path for downstream apps`
  - moved to `Ready`

- [#130](https://github.com/enricopiovesan/Traverse/issues/130) `Define first app-consumable performance baseline`
  - blocked by: missing governing spec for app-facing performance expectations, [#120](https://github.com/enricopiovesan/Traverse/issues/120), [#121](https://github.com/enricopiovesan/Traverse/issues/121), and [#126](https://github.com/enricopiovesan/Traverse/issues/126)
  - unblock path: approve the consumer contract and derive one narrow performance-governance slice for the first supported app path

- [#131](https://github.com/enricopiovesan/Traverse/issues/131) `Define app-facing security and safety boundary for browser and MCP consumers`
  - blocked by: missing governing spec for app-facing security and safety constraints, [#126](https://github.com/enricopiovesan/Traverse/issues/126), and [#129](https://github.com/enricopiovesan/Traverse/issues/129)
  - unblock path: approve the consumer contract and formalize the first browser/MCP safety boundary before release-checklist finalization

## First App-Consumable Gap

The current strongest gap between “implemented foundations” and “first version consumable by an app” is:

1. switch the React browser demo to the live adapter path
2. document the quickstart flow
3. add an end-to-end acceptance path

This chain is tracked explicitly in [#121](https://github.com/enricopiovesan/Traverse/issues/121), [#122](https://github.com/enricopiovesan/Traverse/issues/122), and [#123](https://github.com/enricopiovesan/Traverse/issues/123).

## First External Consumer

The first real downstream consumer is [youaskm3](https://github.com/enricopiovesan/youaskm3), which expects:

- a browser-hosted app shell
- a portable WASM/MCP-friendly runtime model
- a documented integration path rather than repo-private setup knowledge

So the first real Traverse release is not just “a demo exists.” It must be usable by one external app through a stable, documented, app-facing surface.

## Missing For First Release

Already tracked release-critical work:

1. [#109](https://github.com/enricopiovesan/Traverse/pull/109) first WASM AI agent example
2. [#116](https://github.com/enricopiovesan/Traverse/pull/116) checked-in browser demo
3. [#121](https://github.com/enricopiovesan/Traverse/issues/121) live browser demo over the adapter
4. [#122](https://github.com/enricopiovesan/Traverse/issues/122) first app-consumable quickstart
5. [#123](https://github.com/enricopiovesan/Traverse/issues/123) end-to-end acceptance validation
6. [#129](https://github.com/enricopiovesan/Traverse/issues/129) app-facing MCP consumption validation

Tracked consumer-release planning work:

1. [#126](https://github.com/enricopiovesan/Traverse/issues/126) downstream-consumer contract for `youaskm3`
2. [#128](https://github.com/enricopiovesan/Traverse/issues/128) real `youaskm3` integration validation path using only documented Traverse surfaces
3. [#127](https://github.com/enricopiovesan/Traverse/issues/127) v0.1 release checklist that distinguishes blockers from post-release work

## Merge Lane

Active merge candidate:

- none at the moment

Rules while a merge candidate exists:

1. merge the green candidate before starting unrelated work
2. if the candidate is green and behind `main`, update it immediately
3. do not build up a queue of "green but not merged" PRs
4. clean ticket and project state in the same pass as the merge, not hours later

## Quality Rules

- Every active ticket must have:
  - a clear summary
  - explicit dependencies
  - a blocker note if blocked
  - a Definition of Done with no ambiguity
  - exact validation steps

- If a problem is required to make the current slice correct, governed, or mergeable, it must be fixed in the active PR.
- If a problem is valid but not required for the active slice, it must become a `future` ticket instead of silently disappearing.

## Recommended Next Sequence

1. Implement [#121](https://github.com/enricopiovesan/Traverse/issues/121)
2. Implement [#129](https://github.com/enricopiovesan/Traverse/issues/129)
3. Choose whether [#53](https://github.com/enricopiovesan/Traverse/issues/53) or [#122](https://github.com/enricopiovesan/Traverse/issues/122) gets the next delivery slot
4. Complete [#122](https://github.com/enricopiovesan/Traverse/issues/122) and [#123](https://github.com/enricopiovesan/Traverse/issues/123)
5. Execute [#128](https://github.com/enricopiovesan/Traverse/issues/128)
6. Finish [#127](https://github.com/enricopiovesan/Traverse/issues/127) once the real integration evidence exists

## v0.1 Priority

### Must Have

- [#120](https://github.com/enricopiovesan/Traverse/issues/120)
- [#121](https://github.com/enricopiovesan/Traverse/issues/121)
- [#122](https://github.com/enricopiovesan/Traverse/issues/122)
- [#123](https://github.com/enricopiovesan/Traverse/issues/123)
- [#127](https://github.com/enricopiovesan/Traverse/issues/127)
- [#128](https://github.com/enricopiovesan/Traverse/issues/128)
- [#129](https://github.com/enricopiovesan/Traverse/issues/129)

### Soon After

- [#53](https://github.com/enricopiovesan/Traverse/issues/53)
- [#130](https://github.com/enricopiovesan/Traverse/issues/130)
- [#131](https://github.com/enricopiovesan/Traverse/issues/131)

### Later

- no additional first-consumer tickets are intentionally classified here yet

## Project 1

This planning board is mirrored into:

- [GitHub Project 1](https://github.com/users/enricopiovesan/projects/1/)

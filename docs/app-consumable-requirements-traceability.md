# App-Consumable Requirements Traceability

This document maps the first app-consumable Traverse requirements to the current GitHub issue and Project 1 state.

Project 1 is the canonical task board for this work. Every requirement area below must have one or more tickets in that project, and the ticket state must make the release picture obvious without having to reconcile old notes manually.

## Functional Requirements

| Requirement Area | Covered By Tickets | Current State |
|---|---|---|
| Root app-consumable onboarding | [#122](https://github.com/enricopiovesan/Traverse/issues/122), [#127](https://github.com/enricopiovesan/Traverse/issues/127), [#142](https://github.com/enricopiovesan/Traverse/issues/142), [#143](https://github.com/enricopiovesan/Traverse/issues/143) | `Done` / `In Progress` |
| Canonical docs entry path | [#142](https://github.com/enricopiovesan/Traverse/issues/142), [#144](https://github.com/enricopiovesan/Traverse/issues/144) | `In Progress` / `Ready` |
| Release checklist and release-readiness evidence | [#127](https://github.com/enricopiovesan/Traverse/issues/127), [#145](https://github.com/enricopiovesan/Traverse/issues/145), [#150](https://github.com/enricopiovesan/Traverse/issues/150) | `In Progress` / `In Progress` / `In Progress` |
| Versioned consumer bundle and installation steps | [#176](https://github.com/enricopiovesan/Traverse/issues/176) | `Ready` |
| Live browser-consumer path | [#120](https://github.com/enricopiovesan/Traverse/issues/120), [#121](https://github.com/enricopiovesan/Traverse/issues/121), [#123](https://github.com/enricopiovesan/Traverse/issues/123) | `Done` |
| Downstream consumer contract and app-facing validation | [#126](https://github.com/enricopiovesan/Traverse/issues/126), [#128](https://github.com/enricopiovesan/Traverse/issues/128), [#129](https://github.com/enricopiovesan/Traverse/issues/129) | `Done` |
| Real browser-hosted `youaskm3` shell validation | [#179](https://github.com/enricopiovesan/Traverse/issues/179) | `In Progress` |
| MCP WASM server model and validation | [#146](https://github.com/enricopiovesan/Traverse/issues/146), [#158](https://github.com/enricopiovesan/Traverse/issues/158), [#148](https://github.com/enricopiovesan/Traverse/issues/148) | `Done` / `In Progress` / `Blocked` |

## Non-Functional Requirements

| Requirement Area | Covered By Tickets | Current State |
|---|---|---|
| Documentation clarity for the first app-consumable path | [#142](https://github.com/enricopiovesan/Traverse/issues/142), [#143](https://github.com/enricopiovesan/Traverse/issues/143), [#145](https://github.com/enricopiovesan/Traverse/issues/145) | `In Progress` / `Done` / `In Progress` |
| Traceability from requirements to release artifacts | [#145](https://github.com/enricopiovesan/Traverse/issues/145), [#150](https://github.com/enricopiovesan/Traverse/issues/150), [#195](https://github.com/enricopiovesan/Traverse/issues/195) | `In Progress` / `Done` / `In Progress` |
| Operational safety boundary for app consumers | [#131](https://github.com/enricopiovesan/Traverse/issues/131) | `Blocked` |
| First app-consumable performance baseline | [#130](https://github.com/enricopiovesan/Traverse/issues/130) | `Blocked` |

## Current Open First-Release Ticket Set

- [#127](https://github.com/enricopiovesan/Traverse/issues/127) `Prepare the Traverse v0.1 release checklist for app consumers` - `In Progress`
- [#142](https://github.com/enricopiovesan/Traverse/issues/142) `Refresh README for v0.1 release-candidate state` - `In Progress`
- [#144](https://github.com/enricopiovesan/Traverse/issues/144) `Establish one canonical documentation entry path for humans and agents` - `Ready`
- [#145](https://github.com/enricopiovesan/Traverse/issues/145) `Refresh release and requirements traceability docs for current v0.1 state` - `In Progress`
- [#158](https://github.com/enricopiovesan/Traverse/issues/158) `Implement MCP stdio server package foundation` - `In Progress`
- [#179](https://github.com/enricopiovesan/Traverse/issues/179) `Validate the real browser-hosted youaskm3 shell against released Traverse consumer artifacts` - `In Progress`
- [#150](https://github.com/enricopiovesan/Traverse/issues/150) `Prepare and validate the first Traverse v0.1 GitHub release artifact` - `Done`
- [#195](https://github.com/enricopiovesan/Traverse/issues/195) `Publish the first governed Traverse package artifact` - `In Progress`
- [#176](https://github.com/enricopiovesan/Traverse/issues/176) `Publish versioned Traverse consumer bundle for downstream app integration` - `Ready`
- [#130](https://github.com/enricopiovesan/Traverse/issues/130) `Define first app-consumable performance baseline` - `Blocked`
- [#131](https://github.com/enricopiovesan/Traverse/issues/131) `Define app-facing security and safety boundary for browser and MCP consumers` - `Blocked`

## v0.1 Release Ordering

### Must Have For v0.1

- [#120](https://github.com/enricopiovesan/Traverse/issues/120) local browser adapter transport
- [#121](https://github.com/enricopiovesan/Traverse/issues/121) live browser demo over the adapter
- [#122](https://github.com/enricopiovesan/Traverse/issues/122) app-consumable quickstart
- [#123](https://github.com/enricopiovesan/Traverse/issues/123) end-to-end acceptance validation
- [#126](https://github.com/enricopiovesan/Traverse/issues/126) downstream-consumer contract
- [#127](https://github.com/enricopiovesan/Traverse/issues/127) release checklist
- [#128](https://github.com/enricopiovesan/Traverse/issues/128) real `youaskm3` integration validation
- [#129](https://github.com/enricopiovesan/Traverse/issues/129) app-facing MCP consumption validation

### Should Have Soon After v0.1

- [#142](https://github.com/enricopiovesan/Traverse/issues/142) README release-candidate refresh
- [#144](https://github.com/enricopiovesan/Traverse/issues/144) canonical documentation entry path
- [#145](https://github.com/enricopiovesan/Traverse/issues/145) requirements traceability refresh
- [#158](https://github.com/enricopiovesan/Traverse/issues/158) dedicated MCP stdio server package foundation
- [#150](https://github.com/enricopiovesan/Traverse/issues/150) release artifact and publication bundle
- [#195](https://github.com/enricopiovesan/Traverse/issues/195) package release pointer
- [#176](https://github.com/enricopiovesan/Traverse/issues/176) versioned consumer bundle and installation steps

### Later

- [#130](https://github.com/enricopiovesan/Traverse/issues/130) first app-consumable performance baseline
- [#131](https://github.com/enricopiovesan/Traverse/issues/131) app-facing security and safety boundary
- [#148](https://github.com/enricopiovesan/Traverse/issues/148) downstream validation for the dedicated MCP WASM server

## Rule

If a new app-consumable requirement appears and cannot be mapped to one or more tickets above, create the missing issue first and add it to Project 1 before calling the release backlog complete.

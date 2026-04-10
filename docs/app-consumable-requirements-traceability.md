# App-Consumable Requirements Traceability

This document maps the first app-consumable Traverse requirements to GitHub issues on [Project 1](https://github.com/users/enricopiovesan/projects/1/).

Project 1 is the canonical task board for this work. Every requirement area below must have one or more tickets in that project, with explicit indication of whether more governing spec work is needed.

## Functional Requirements

| Requirement Area | Covered By Tickets | Spec Signal |
|---|---|---|
| Public integration surface for downstream apps | [#126](https://github.com/enricopiovesan/Traverse/issues/126), [#128](https://github.com/enricopiovesan/Traverse/issues/128) | `spec`, `needs-spec` |
| Runtime execution for app consumers | [#120](https://github.com/enricopiovesan/Traverse/issues/120), [#121](https://github.com/enricopiovesan/Traverse/issues/121), [#123](https://github.com/enricopiovesan/Traverse/issues/123), [#128](https://github.com/enricopiovesan/Traverse/issues/128) | `no-spec-needed`, `needs-spec` |
| Eventing and subscriptions for the app | [#120](https://github.com/enricopiovesan/Traverse/issues/120), [#121](https://github.com/enricopiovesan/Traverse/issues/121), [#123](https://github.com/enricopiovesan/Traverse/issues/123) | `no-spec-needed` |
| Browser adapter transport and live browser path | [#120](https://github.com/enricopiovesan/Traverse/issues/120), [#121](https://github.com/enricopiovesan/Traverse/issues/121) | `no-spec-needed` |
| MCP consumption path for downstream apps | [#129](https://github.com/enricopiovesan/Traverse/issues/129) | `needs-spec` |
| Contracts and compatibility boundary for downstream apps | [#126](https://github.com/enricopiovesan/Traverse/issues/126), [#127](https://github.com/enricopiovesan/Traverse/issues/127) | `spec`, `needs-spec` |
| Portable packaging for executable agents and capabilities | [#109](https://github.com/enricopiovesan/Traverse/pull/109), [#53](https://github.com/enricopiovesan/Traverse/issues/53), [#111](https://github.com/enricopiovesan/Traverse/issues/111) | implemented / `no-spec-needed` |
| Developer workflow and app-consumable quickstart | [#122](https://github.com/enricopiovesan/Traverse/issues/122), [#123](https://github.com/enricopiovesan/Traverse/issues/123) | `no-spec-needed` |
| Release/governance path for first external consumer use | [#126](https://github.com/enricopiovesan/Traverse/issues/126), [#127](https://github.com/enricopiovesan/Traverse/issues/127), [#128](https://github.com/enricopiovesan/Traverse/issues/128), [#129](https://github.com/enricopiovesan/Traverse/issues/129) | `spec`, `needs-spec` |

## Non-Functional Requirements

| Requirement Area | Covered By Tickets | Spec Signal |
|---|---|---|
| Stability of public consumer surfaces | [#126](https://github.com/enricopiovesan/Traverse/issues/126), [#127](https://github.com/enricopiovesan/Traverse/issues/127) | `spec`, `needs-spec` |
| Determinism of runtime updates and outcomes | [#120](https://github.com/enricopiovesan/Traverse/issues/120), [#123](https://github.com/enricopiovesan/Traverse/issues/123), [#128](https://github.com/enricopiovesan/Traverse/issues/128), [#129](https://github.com/enricopiovesan/Traverse/issues/129) | `no-spec-needed`, `needs-spec` |
| Portability across browser, CLI, and future hosts | [#126](https://github.com/enricopiovesan/Traverse/issues/126), [#129](https://github.com/enricopiovesan/Traverse/issues/129), [#109](https://github.com/enricopiovesan/Traverse/pull/109) | `spec`, `needs-spec` |
| Explainability of runtime, trace, and failures | [#123](https://github.com/enricopiovesan/Traverse/issues/123), [#128](https://github.com/enricopiovesan/Traverse/issues/128), [#129](https://github.com/enricopiovesan/Traverse/issues/129) | `no-spec-needed`, `needs-spec` |
| Performance for the first app-consumable path | [#130](https://github.com/enricopiovesan/Traverse/issues/130) | `needs-spec` |
| Reliability and repeatability of the supported flow | [#122](https://github.com/enricopiovesan/Traverse/issues/122), [#123](https://github.com/enricopiovesan/Traverse/issues/123), [#128](https://github.com/enricopiovesan/Traverse/issues/128) | `no-spec-needed`, `needs-spec` |
| Testability under CI and protected gates | [#123](https://github.com/enricopiovesan/Traverse/issues/123), [#127](https://github.com/enricopiovesan/Traverse/issues/127), [#128](https://github.com/enricopiovesan/Traverse/issues/128), [#129](https://github.com/enricopiovesan/Traverse/issues/129) | `no-spec-needed`, `needs-spec` |
| Maintainability and public/internal boundary discipline | [#126](https://github.com/enricopiovesan/Traverse/issues/126), [#127](https://github.com/enricopiovesan/Traverse/issues/127) | `spec`, `needs-spec` |
| Security and safety of browser/MCP consumer paths | [#131](https://github.com/enricopiovesan/Traverse/issues/131) | `needs-spec` |
| Documentation quality for external app consumption | [#122](https://github.com/enricopiovesan/Traverse/issues/122), [#127](https://github.com/enricopiovesan/Traverse/issues/127) | `no-spec-needed`, `needs-spec` |

## Current Open First-Release Ticket Set

- [#120](https://github.com/enricopiovesan/Traverse/issues/120) `Implement local browser adapter transport for runtime subscriptions` - `no-spec-needed`
- [#121](https://github.com/enricopiovesan/Traverse/issues/121) `Upgrade React browser demo to consume the live local browser adapter` - `no-spec-needed`
- [#122](https://github.com/enricopiovesan/Traverse/issues/122) `Write the first app-consumable quickstart for Traverse v0.1` - `no-spec-needed`
- [#123](https://github.com/enricopiovesan/Traverse/issues/123) `Add end-to-end acceptance validation for the first app-consumable flow` - `no-spec-needed`
- [#126](https://github.com/enricopiovesan/Traverse/issues/126) `Define Traverse v0.1 downstream-consumer contract for youaskm3` - `spec`
- [#127](https://github.com/enricopiovesan/Traverse/issues/127) `Prepare the Traverse v0.1 release checklist for app consumers` - `needs-spec`
- [#128](https://github.com/enricopiovesan/Traverse/issues/128) `Validate the first real youaskm3 integration path against Traverse` - `needs-spec`
- [#129](https://github.com/enricopiovesan/Traverse/issues/129) `Validate the first app-facing MCP consumption path for downstream apps` - `needs-spec`
- [#130](https://github.com/enricopiovesan/Traverse/issues/130) `Define first app-consumable performance baseline` - `needs-spec`
- [#131](https://github.com/enricopiovesan/Traverse/issues/131) `Define app-facing security and safety boundary for browser and MCP consumers` - `needs-spec`

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

- [#53](https://github.com/enricopiovesan/Traverse/issues/53) second WASM AI agent example
- [#130](https://github.com/enricopiovesan/Traverse/issues/130) app-consumable performance baseline
- [#131](https://github.com/enricopiovesan/Traverse/issues/131) app-facing security and safety boundary

### Later

- no additional app-consumable release tickets are intentionally placed here yet; future additions should be added only when they are outside the first release and the near-follow-up set above

## Rule

If a new app-consumable requirement appears and cannot be mapped to one or more tickets above, create the missing issue first and add it to Project 1 before calling the release backlog complete.

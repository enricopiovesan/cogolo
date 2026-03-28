# Multi-Thread Workflow

Traverse can support parallel execution, but only when parallel work is real.

One Codex thread is one active worker. If we want true parallel work, we should run multiple Codex threads, each with a separate issue, branch, and pull request.

## Thread Roles

### PM Thread

The PM thread:

- keeps the backlog, labels, blockers, and Project 1 current
- talks with Enrico about product and governance decisions
- decides when work is `ready`, `blocked`, `in-progress`, or `future`
- does not mark a ticket `in-progress` unless a real worker has started

### Dev Threads

Each dev thread:

- owns exactly one active issue at a time
- works on exactly one `codex/...` branch at a time
- opens exactly one PR for that slice
- updates the issue and PR with validation evidence

Recommended rule:

- one dev thread per issue
- if two issues touch the same files heavily, do not start them in parallel

### Review / Integration Thread

The review thread:

- checks spec alignment
- checks contract and workflow drift
- checks merge conflicts and integration risk
- ensures must-fix findings are fixed in the active PR
- turns non-blocking follow-up work into `future` tickets

## Status Rules

Use statuses this way:

- `Ready`: approved and available to start
- `In Progress`: a real dev thread is actively working the ticket now
- `Blocked`: the ticket cannot continue and the blocker is visible in both the issue body and Project 1 note
- `Future`: valid work that is tracked but intentionally not active now

Do not move work to `In Progress` merely because it is a candidate for parallel execution.

## Required Parallel Work Rules

For parallel work to be valid:

- each active issue must have a dedicated dev thread
- each active issue must have its own branch
- each active issue must have its own PR
- Project 1 `Status` must match reality
- Project 1 `Note` should identify the worker, branch, or workstream when useful

## Recommended Current Split

For the current expedition artifact work, the cleanest split is:

- Workstream 1: [#42](https://github.com/enricopiovesan/Traverse/issues/42) event contracts
- Workstream 2: [#44](https://github.com/enricopiovesan/Traverse/issues/44) atomic capability contracts
- Workstream 3: [#43](https://github.com/enricopiovesan/Traverse/issues/43) composed capability contract
- Workstream 4: [#45](https://github.com/enricopiovesan/Traverse/issues/45) workflow artifact

If we only have one active dev thread, these should remain `Ready`.

If we have four active dev threads, then all four can honestly be `In Progress`.

## Starter Prompts

Use this PM thread prompt:

```text
Act as the Traverse PM / scrum master thread.
Your job is to keep GitHub issues, Project 1, labels, blockers, notes, and PR flow accurate.
Do not mark a ticket in progress unless a real dev thread has started it.
When a problem is must-fix for the active slice, it must be fixed in the active PR.
When a problem is non-blocking, create a future ticket.
Keep all work aligned to approved specs and project-management rules.
```

Use this dev thread prompt:

```text
Act as a Traverse dev thread for issue #NN.
Only work on this issue.
Use a dedicated codex branch and open a dedicated PR.
Keep implementation strictly aligned with the governing spec.
If you find a must-fix issue for this slice, fix it in the same PR.
If you find a non-blocking improvement, create or request a future ticket instead of expanding scope.
Do not change ticket or project status unless the PM thread asks for it.
```

Use this review thread prompt:

```text
Act as the Traverse review / integration thread.
Your job is to review active PRs for spec alignment, contract drift, workflow drift, missing tests, merge risk, and governance gaps.
Must-fix findings should stay in the active PR.
Nice-to-have follow-ups should become future tickets.
Keep the repo and board consistent with the approved process.
```

# Ticket Standard

This document defines the minimum quality bar for Traverse issues.

## Core Rule

Every meaningful ticket must be explicit enough that a developer can tell:

- whether the work is available to start
- whether it is blocked
- whether it needs a governing spec
- what "done" means
- how to validate that it is complete

## Required Labels

Every active or future ticket should use the relevant labels from this set:

- `in-progress`
- `blocked`
- `needs-spec`
- `needs-enrico`
- `future`
- `contracts`
- `workflow`
- `runtime`
- `quality`
- `spec`

Use `no-spec-needed` when the work can proceed under existing approved specs or is non-governing support work.

Use labels this way:

- `in-progress`: currently being worked on right now
- `blocked`: started or selected, but cannot continue until the blocker is removed
- `future`: valid backlog work that is intentionally not active now

Use Project 1 status for availability:

- `Ready`: approved and not started yet
- `In Progress`: currently being worked on right now
- `Blocked`: cannot continue until the blocker is removed

Do not leave newly created tickets in `Todo`. If work cannot start yet, set the Project 1 item to `Blocked` and add a short blocker note. If it can start, set it to `Ready`.

Do not move work to `in-progress` just because it is a candidate for parallel execution. Use `in-progress` only when there is real active execution, typically with an active branch, PR, or an explicitly assigned developer currently working the ticket.

If a ticket has an open PR, `in-progress` should remain until that PR merges or is closed. Once the PR merges, the ticket should be closed or moved out of `in-progress` in the same cleanup pass.

Open-PR rule:

- an open PR and a `Ready` Project 1 status must never coexist on the same ticket
- when a PR is opened, the ticket-state handoff must happen immediately in the same operational pass
- if this invariant is violated, fixing the ticket state takes priority over additional backlog cleanup

## Required Ticket Sections

Every meaningful work ticket should include:

- `Summary`
- `Why`
- `Depends on`
- `Blocked by`
- `Definition of Done`
- `Validation`

## Definition of Done Rule

Definition of done must be specific enough that completion is unambiguous.

It should say exactly what artifacts, files, behavior, or checks must exist when the ticket is complete.

Avoid vague completion text such as:

- "implement feature"
- "support workflow"
- "improve runtime"

Prefer explicit done criteria such as:

- which files or artifact types must exist
- which commands must pass
- which CI checks must be green
- which contracts, examples, or workflow ids must be present

## Validation Rule

Validation instructions must be concrete and reproducible.

Each ticket should identify:

- exact commands to run, when known
- exact checks expected to pass
- exact outputs or artifacts expected to exist

If a ticket is spec-only, validation should say how we know the spec is complete and merge-valid.

If a ticket is implementation work, validation should include test and quality-gate expectations.

## Blocked Rule

If a ticket is blocked, the ticket must say why.

Use:

- label: `blocked`
- section: `Blocked by`

The `Blocked by` section should name the missing dependency clearly, for example:

- missing governing spec
- waiting on Enrico decision
- depends on issue `#NN`
- depends on merged PR `#NN`

When the ticket is on Project 1 with status `Blocked`, the project `Note` field should also contain a short blocker summary so the reason is visible directly on the board.

## Must-Fix vs Future Rule

When a real problem is found during active work:

- if it is required for correctness, governance, mergeability, or stated acceptance criteria, fix it in the same PR
- if it is not required to complete the current slice, create a `future` ticket instead of silently dropping it

This keeps the active PR clean without losing useful follow-up work.

## Merge Candidate Rule

When a ticket's PR is the next candidate to merge into a protected base branch:

- do not start unrelated side work ahead of merging it
- update the PR to the latest base immediately after any prior merge changes `main`
- keep the ticket and project item clearly marked as active until the merge finishes
- if the PR is green but `BEHIND`, update it immediately rather than waiting on stale checks

This rule prevents "green but behind base" drift from silently stalling delivery.

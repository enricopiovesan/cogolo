# Ticket Standard

This document defines the minimum quality bar for Traverse issues.

## Core Rule

Every meaningful ticket must be explicit enough that a developer can tell:

- whether the work is ready to start
- whether it is blocked
- whether it needs a governing spec
- what "done" means
- how to validate that it is complete

## Required Labels

Every active or future ticket should use the relevant labels from this set:

- `ready`
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

Use status labels this way:

- `ready`: approved and not started yet
- `in-progress`: currently being worked on
- `blocked`: started or selected, but cannot continue until the blocker is removed
- `future`: valid backlog work that is intentionally not active now

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

## Must-Fix vs Future Rule

When a real problem is found during active work:

- if it is required for correctness, governance, mergeability, or stated acceptance criteria, fix it in the same PR
- if it is not required to complete the current slice, create a `future` ticket instead of silently dropping it

This keeps the active PR clean without losing useful follow-up work.

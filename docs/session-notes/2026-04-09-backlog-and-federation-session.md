# Backlog and Federation Session Note

Date: 2026-04-09

## Purpose

Capture the operating model and the federation-spec direction established in this session so we do not have to rebuild the same context later.

## Operating Model

- Backlog steward: owns ticket state, labels, blocker notes, and Project 1 truth.
- PR steward: owns open PRs, reruns, mergeability, and merge completion.
- Delivery lanes: own one implementation ticket at a time each, and only from the actionable backlog.

## Backlog Rules

- A ticket with an open PR must be `In Progress`, not `Ready`.
- A closed ticket must be `Done` on the issue and on Project 1.
- A ticket with `needs-spec` must not be shown as `Ready`.
- Closed tickets must not keep stale workflow labels like `ready`, `in-progress`, or `blocked`.
- Project 1 is the actionability source of truth, but it must match the issue state and labels.

## Federation Design Direction

- Use an end-to-end vertical-slice approach from the beginning.
- Split future work into separate future tickets instead of widening the current spec until it becomes vague.
- Prefer the long-term solution, even if it means more than one ticket.
- Treat each spec as a real implementable slice, not a vague umbrella.
- The federation governing spec was approved as `026-federation-registry-routing`.

## Session Emphasis

- The user specifically wanted the backlog to stay truthful without manual chasing.
- The user also wanted federation work to be designed as a long-term end-to-end path, with extra behaviors moved into future tickets.
- The current blocked federation chain should be spec'd in a way that preserves the full long-term architecture while still being split into clean ticket-sized steps.

## Follow-Up

- Continue using the backlog/PR/delivery lane split.
- Keep `needs-spec` work blocked until the governing spec exists.
- Keep future architectural extensions in separate tickets instead of absorbing them into the first slice.

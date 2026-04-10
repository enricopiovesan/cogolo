# Project Management

Traverse uses this GitHub Project as the canonical task board:

- [GitHub Project 1](https://github.com/users/enricopiovesan/projects/1/)

## Working Rule

All meaningful work must be traceable through all three of these artifacts:

- a GitHub issue
- a Project 1 item
- a pull request

This is the default Traverse operating rule for spec slices, implementation slices, governance work, and material documentation changes.

Ticket quality rules are defined in:

- [docs/ticket-standard.md](/Users/piovese/Documents/cogolo/docs/ticket-standard.md)
- [docs/multi-thread-workflow.md](/Users/piovese/Documents/cogolo/docs/multi-thread-workflow.md)

## Preferred Flow

1. Start from the governing spec or approved design discussion.
2. Create or link the GitHub issue.
3. Ensure the issue is represented on Project 1.
4. Open a pull request that links the issue or project item.
5. Keep implementation, contracts, and tests aligned with the governing spec.

## Issue Guidance

Issues should describe:

- problem or goal
- affected spec or capability/workflow area
- expected outcome
- any compatibility or governance concerns
- explicit definition of done
- explicit validation steps
- explicit blocker note when blocked

## Pull Request Guidance

Pull requests should include:

- linked issue or project item
- governing spec version
- contract changes, if any
- validation evidence
- ADR reference, if required

Implementation and spec pull requests must declare their governing specs in the PR body under a `## Governing Spec` section. Those declarations are validated against:

- `specs/governance/approved-specs.json`

## Required Traceability

The expected day-to-day rule is:

- one issue per meaningful slice of work
- that issue represented on Project 1
- one pull request implementing or codifying that slice

Exceptions should be rare and should be called out explicitly in the PR notes.

## Board Discipline

Recommended workflow labels:

- `in-progress`
- `blocked`
- `needs-spec`
- `needs-enrico`
- `future`
- `no-spec-needed`

Recommended categories for task tracking:

- specs and architecture
- runtime and contracts
- registries and workflows
- capabilities and examples
- browser demo
- quality and CI

The exact board columns can evolve, but the project board should remain the primary planning surface and the issue should remain the durable record of intent.

Status intent should stay simple:

- Project 1 status is the only actionability signal.
- `Ready` means the ticket can be started now
- `In Progress` means someone is actively working it right now
- `Blocked` means work cannot continue until the blocker named in the ticket is cleared
- `Todo` should not be used for newly created work items; new tickets should be moved directly to `Ready` or `Blocked`

Project 1 status is the only actionability signal.

When a Project 1 item is marked `Blocked`, the project `Note` field should summarize the blocker in one short sentence so the reason is visible on the board without opening the issue.

Potential parallel candidates should stay `Ready` until they are actually picked up. We should not use `In Progress` as a placeholder for work that is merely available to start.

Open PR-backed tickets must be reflected as `In Progress` in both the issue labels and Project 1. The PM thread should treat any mismatch as a board-drift bug and fix it immediately. The backlog-sync workflow now automates the normal PR open and merge handoff so the issue and Project 1 row move to `In Progress` and then `Done` without waiting for a manual PM pass.

Only tickets with real active execution should appear on Project 1 as `In Progress`.

For true parallel execution, use separate Codex threads with separate issues, branches, and PRs. The operating model is documented in:

- [docs/multi-thread-workflow.md](/Users/piovese/Documents/cogolo/docs/multi-thread-workflow.md)

Run the board audit when you change issue labels, Project 1 status, or PR state:

```bash
bash scripts/ci/project_board_audit.sh
```

The board audit logic lives in [scripts/ci/project_board_audit.sh](/Users/piovese/Documents/cogolo/scripts/ci/project_board_audit.sh).

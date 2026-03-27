# Project Management

Traverse uses this GitHub Project as the canonical task board:

- [GitHub Project 1](https://github.com/users/enricopiovesan/projects/1/)

## Working Rule

All meaningful work should be traceable to a project item.

That can be:

- an issue
- a pull request
- a draft item on the project board

## Preferred Flow

1. Start from the governing spec or approved design discussion.
2. Create or link an issue.
3. Ensure the issue is represented on the project board.
4. Reference the project item in the pull request.
5. Keep implementation, contracts, and tests aligned with the governing spec.

## Issue Guidance

Issues should describe:

- problem or goal
- affected spec or capability/workflow area
- expected outcome
- any compatibility or governance concerns

## Pull Request Guidance

Pull requests should include:

- linked issue or project item
- governing spec version
- contract changes, if any
- validation evidence
- ADR reference, if required

Implementation pull requests must declare their governing specs in the PR body under a `## Governing Spec` section. Those declarations are validated against:

- `specs/governance/approved-specs.json`

## Board Discipline

Recommended categories for task tracking:

- specs and architecture
- runtime and contracts
- registries and workflows
- capabilities and examples
- browser demo
- quality and CI

The exact board columns can evolve, but the project board should remain the primary planning surface.

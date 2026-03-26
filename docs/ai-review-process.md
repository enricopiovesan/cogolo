# AI Review Process

This document defines how AI-assisted review should work in Cogolo.

## Purpose

AI review is intended to improve:

- spec alignment
- contract alignment
- compatibility awareness
- test completeness
- architectural consistency

It does not replace human review.

## When AI Review Should Happen

AI review should run on pull requests after basic validation passes.

The AI review should focus on:

- drift from the governing spec
- contract mismatches
- compatibility risks
- missing tests
- hidden bypasses of contract, policy, constraint, or trace paths

Repository automation:

- workflow: `.github/workflows/ai-pr-review.yml`
- script: `scripts/ci/ai_pr_review.sh`
- required secret: `OPENAI_API_KEY`
- optional repo variable: `OPENAI_MODEL` (defaults to `gpt-4o-mini`)

The workflow updates a single sticky PR comment with the current AI review summary and findings.

## How Review Comments Should Be Handled

For each actionable review comment:

1. Respond in the PR thread.
2. Either:
   - fix the issue in the branch, or
   - create a linked issue/project item for follow-up if it is intentionally deferred.
3. Resolve the thread only after the fix or explicit follow-up exists.

## Comment Actioning Rule

No significant AI or human review concern should disappear silently.

Each important comment should end in one of these states:

- fixed in code
- rejected with written rationale
- deferred with linked task and owner

## Merge Expectations

Pull requests should not merge when:

- required review threads are unresolved
- review findings identify unresolved spec or contract drift
- follow-up work is required but not captured in the project board

If the AI workflow reports blocking issues, the PR should be treated as not ready even if the workflow is not yet a required branch-protection check.

## Task Tracking

Follow-up work should be represented in:

- GitHub issues
- GitHub Project 1

Project board:

- [GitHub Project 1](https://github.com/users/enricopiovesan/projects/1/)

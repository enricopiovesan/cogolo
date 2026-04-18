# Traverse Development Antipatterns

This guide collects the main repo-specific shortcuts and habits that make Traverse harder to govern, harder to merge, or harder to trust.

These are not generic style preferences. They are the patterns most likely to create spec drift, CI failures, hidden coupling, or confusing contributor experience.

## 1. Writing Code Before The Governing Slice Is Clear

Antipattern:

- implementing behavior first and hoping the spec or ticket can be updated later

Why it is a problem:

- Traverse is spec-governed
- CI checks expect the PR body and changed files to align with approved spec ids
- post-hoc governance almost always produces drift or misleading PR metadata

Do instead:

- start from the approved governing spec
- if the slice truly has no missing governance work, label it `no-spec-needed`
- keep the PR `## Governing Spec` section narrow and accurate

## 2. Declaring Stale Or Extra Specs In A PR Body

Antipattern:

- copying an old PR body and leaving unrelated spec ids in place

Why it is a problem:

- `spec-alignment` validates the declared specs
- stale declarations create false governance claims and can fail CI even when the code is fine

Do instead:

- declare only the approved spec ids actually needed for the files you changed
- update the PR body after rebases or scope changes

## 3. Using `in-progress` As A Placeholder

Antipattern:

- marking a ticket `in-progress` just because it looks like the next good candidate

Why it is a problem:

- the Project 1 status is the repo’s actionability signal
- fake `in-progress` state creates board drift and hides what is truly ready

Do instead:

- keep a ticket `Ready` until someone is actively executing it
- add a blocker note when something is blocked instead of leaving the state ambiguous

## 4. Leaving Hidden Placeholder Paths In Docs

Antipattern:

- letting docs imply a path is supported when it is still partial, inert, or only test scaffolding

Why it is a problem:

- developers and agents will follow the wrong path first
- this wastes time and creates incorrect assumptions about the supported surface

Do instead:

- point to the supported path explicitly
- make unsupported commands or bootstrap ideas fail loudly
- distinguish normative runtime behavior from fixtures and examples

## 5. Depending On Internal Knowledge Instead Of Public Docs

Antipattern:

- assuming contributors will infer the right command, file, or release surface from repo familiarity

Why it is a problem:

- Traverse is trying to be consumable by both humans and coding agents
- “just know where to look” is not a stable interface

Do instead:

- add docs for public entry paths
- link new docs from the README, quickstart, tutorial index, or getting-started flow when they matter to first-time users

## 6. Using `unwrap`, `panic!`, Or TODO-Driven Control Flow

Antipattern:

- relying on crashes, unchecked assumptions, or TODO comments in production code

Why it is a problem:

- Traverse expects predictable failure behavior and actionable error states
- panic-driven code undermines runtime trust and makes automation harder to reason about

Do instead:

- return explicit errors
- document the failure mode
- capture follow-up work as a real issue instead of a lingering TODO comment

## 7. Sneaking In Demo-Only Hacks

Antipattern:

- adding one-off behavior that only works for an example but lives in foundation code

Why it is a problem:

- foundation crates are supposed to stay portable and governed
- demo shortcuts become accidental product behavior if they survive one merge

Do instead:

- keep example-specific logic in examples, fixtures, or dedicated docs
- keep core runtime and registry behavior generic and explainable

## 8. Treating Tests As Optional Around Core Runtime Logic

Antipattern:

- landing core runtime, registry, or contract logic without exhaustive automated validation

Why it is a problem:

- Traverse keeps `100%` coverage expectations for core business and runtime logic
- missing tests usually mean missing determinism guarantees too

Do instead:

- add or update the tests in the same slice
- use the repository checks and coverage gate as design feedback, not as an afterthought

## 9. Opening A PR Without The Full Traceability Chain

Antipattern:

- code change without a durable issue, Project 1 item, and PR linkage

Why it is a problem:

- the repo’s governance model depends on issue + project item + PR traceability
- missing links make later review, release prep, and audit work harder

Do instead:

- keep every meaningful slice tied to:
  - a GitHub issue
  - a Project 1 item
  - a pull request

## 10. Letting “Behind Main” Sit Unresolved

Antipattern:

- waiting too long to restack a green PR after another docs or onboarding PR merges

Why it is a problem:

- GitHub branch protection blocks merges on stale heads
- queue latency grows fast when docs slices all touch the same top-level files

Do instead:

- rebase promptly when the PR becomes `BEHIND`
- rerun the checks on the updated head instead of trying to merge around the branch protection

## Related Docs

- [docs/quality-standards.md](quality-standards.md)
- [docs/project-management.md](project-management.md)
- [docs/multi-thread-workflow.md](multi-thread-workflow.md)
- [CONTRIBUTING.md](../CONTRIBUTING.md)

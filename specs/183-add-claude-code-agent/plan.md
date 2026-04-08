# Implementation Plan: Add Claude Code as parallel AI development agent

**Branch**: `183-add-claude-code-agent` | **Date**: 2026-04-07 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/183-add-claude-code-agent/spec.md`

## Summary

Register Claude Code as a first-class parallel AI agent in the Traverse Speckit governance framework. Three files change: CLAUDE.md is created at the repo root as Claude Code's entry point; init-options.json gains `"claude"` as a supported `"ai"` value; speckit-plan SKILL.md gains a second `update-agent-context.sh claude` invocation in Phase 1. No Rust code, no contract, no CI gate changes.

## Technical Context

**Language/Version**: Rust 1.94+ (workspace, unchanged); bash scripting; Markdown
**Primary Dependencies**: `.specify/scripts/bash/update-agent-context.sh` (already supports `claude`); `.specify/templates/agent-file-template.md`
**Storage**: N/A
**Testing**: Manual — open Claude Code at repo root, confirm CLAUDE.md loaded; run speckit-plan, confirm CLAUDE.md updated
**Target Platform**: Developer workstation; Claude Code CLI
**Project Type**: Dev tooling configuration
**Performance Goals**: N/A
**Constraints**: Must not break existing Codex flows; must not touch any Rust crate, contract, or CI gate
**Scale/Scope**: Three files modified or created; zero new runtime capabilities

## Constitution Check

This feature is **dev tooling only**. No constitution gates block this work.

| Gate | Status | Note |
|------|--------|------|
| Capability-First Boundaries | N/A | No business capability defined |
| Contracts Are Source of Truth | Pass | No contracts touched |
| Specs Are Versioned, Immutable, Merge-Gating | Pass | This spec governs the work |
| Portability Over Host Coupling | N/A | Dev tooling; outside runtime portability boundary |
| Discoverability and Governance by Default | Pass | CLAUDE.md improves governance discoverability |
| Runtime Decisions Must Be Explainable | N/A | No runtime behavior changes |
| Small, Verifiable v0.1 | Pass | Smallest possible change; traverse-mcp deferred |

## Project Structure

### Documentation (this feature)

```text
specs/183-add-claude-code-agent/
├── spec.md
└── plan.md              # This file
```

### Files touched

```text
CLAUDE.md                                    # CREATED — Claude Code entry point
.specify/init-options.json                   # MODIFIED — add "claude" as valid ai value
.agents/skills/speckit-plan/SKILL.md         # MODIFIED — add update-agent-context.sh claude call
```

## Phase 0: Research

No unknowns. All inputs are resolved:

- `update-agent-context.sh claude` is implemented at line 619; writes to `$REPO_ROOT/CLAUDE.md` using the agent-file-template.md. No script changes required.
- speckit-plan SKILL.md Phase 1 step 3 currently only calls `codex`. Adding `claude` is the entire change.
- init-options.json has no enum validation blocking a new `"ai"` value.
- CLAUDE.md initial content: use the agent-file-template.md structure with a hand-authored `## Governance` section inside the manual additions markers.

## Phase 1: Design & Contracts

No external contracts. This feature has no runtime-facing interfaces.

### CLAUDE.md structure

Must satisfy two purposes: (1) human-readable governance reference and (2) machine-updatable via update-agent-context.sh. The script looks for `## Active Technologies`, `## Recent Changes`, and `<!-- MANUAL ADDITIONS START/END -->` markers.

```text
# Traverse Development Guidelines
Auto-generated. Last updated: <date>

## Active Technologies        ← auto-updated
## Project Structure          ← auto-updated
## Commands                   ← auto-updated
## Code Style                 ← auto-updated
## Recent Changes             ← auto-updated

<!-- MANUAL ADDITIONS START -->
## Governance                 ← hand-authored, preserved across updates
<!-- MANUAL ADDITIONS END -->
```

### speckit-plan SKILL.md change

Phase 1, step 3 — add immediately after existing codex line:
```
- Run `.specify/scripts/bash/update-agent-context.sh claude`
```

### init-options.json change

Change `"ai": "codex"` → `"ai": ["codex", "claude"]` to declare both agents active.

## Implementation Sequence

1. Create `CLAUDE.md` at repo root
2. Edit `.agents/skills/speckit-plan/SKILL.md` — add claude invocation
3. Edit `.specify/init-options.json` — declare both agents
4. Run `update-agent-context.sh claude` to verify script path works end-to-end (requires active plan.md; manual verification)
5. Verify `cargo test && cargo clippy` unchanged
6. Commit, push, open PR referencing #183

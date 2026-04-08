# Feature Specification: Add Claude Code as parallel AI development agent

**Feature Branch**: `183-add-claude-code-agent`
**Created**: 2026-04-07
**Status**: Draft
**Input**: GitHub issue #183 — Add Claude Code as parallel AI development agent

> **Governance note**: This is dev tooling, not a runtime capability. The constitution's Capability-First Boundaries rule (Principle I) does not apply here. No new business capability is being introduced, no contract is being created or amended, and no runtime behavior is changing. This spec governs tooling and developer workflow configuration only.

## User Scenarios & Testing

### User Story 1 - Start work with Claude Code and receive project context (Priority: P1)

As a developer using Claude Code on the Traverse project, I want Claude Code to automatically load the project's governance rules, constitution, and active spec when I open a session, so that I can plan and implement features under the same governance framework as Codex without manually locating those documents.

**Why this priority**: CLAUDE.md is the entry point Claude Code reads at session start. Without it, Claude Code has no awareness of the Speckit framework, the constitution, or the active branch's spec. Every other story depends on this being in place first.

**Independent Test**: Open Claude Code at the repo root. Verify that the session context includes references to the constitution and the active spec directory without any manual prompting.

**Acceptance Scenarios**:

1. **Given** CLAUDE.md exists at the project root, **When** Claude Code initializes a session, **Then** Claude Code's context includes the constitution location, the specs directory convention, and the governance rules governing feature work.
2. **Given** a developer is on a feature branch with a corresponding `specs/<branch>/` directory, **When** Claude Code starts, **Then** CLAUDE.md directs Claude Code to load the relevant spec artifacts for that branch.
3. **Given** CLAUDE.md is present, **When** a developer asks Claude Code to begin planning or implementing a feature, **Then** Claude Code applies the spec-first, contract-first governance rules from the constitution rather than inventing its own process.

---

### User Story 2 - Run speckit-plan and have Claude Code context updated automatically (Priority: P2)

As a developer using Claude Code, I want the speckit-plan skill to update CLAUDE.md with the current plan's technical context alongside AGENTS.md, so that both Codex and Claude Code receive consistent, up-to-date project information after every planning cycle.

**Why this priority**: Without this, CLAUDE.md would become stale after new plans are created. The update-agent-context.sh script already supports the `claude` agent type; the speckit-plan SKILL.md just needs to invoke it.

**Independent Test**: Run the speckit-plan skill on any feature branch that has a plan.md. Confirm that CLAUDE.md is created or updated with the current plan's language, framework, and branch entry.

**Acceptance Scenarios**:

1. **Given** a completed plan.md on a feature branch, **When** speckit-plan Phase 1 executes, **Then** `update-agent-context.sh claude` is invoked and CLAUDE.md reflects the plan's technical context.
2. **Given** CLAUDE.md already exists from a prior run, **When** speckit-plan is re-run on a new branch, **Then** CLAUDE.md is updated with the new branch's technology additions while preserving manual additions between the designated markers.
3. **Given** speckit-plan runs successfully, **When** both agent context updates complete, **Then** AGENTS.md and CLAUDE.md are consistent in their technology stack and recent changes entries for the same plan.

---

### User Story 3 - Declare claude as a supported init AI agent type (Priority: P3)

As a developer setting up a new Speckit project, I want to choose `claude` as the AI agent type in init-options.json so that project initialization targets Claude Code's entry point file.

**Why this priority**: Supporting `claude` in init-options.json makes the choice declarative. Lower priority because the update script already works independently.

**Independent Test**: Set `"ai": "claude"` in init-options.json. Run any agent context update flow. Confirm CLAUDE.md is created at the root level.

**Acceptance Scenarios**:

1. **Given** `"ai": "claude"` in init-options.json, **When** the agent context update runs, **Then** CLAUDE.md is the target output file.
2. **Given** `"ai": "codex"` in init-options.json, **When** the agent context update runs, **Then** AGENTS.md remains the output, unchanged from current behavior.
3. **Given** both agent files already exist in a repo with mixed agent usage, **When** the update-all path runs, **Then** both CLAUDE.md and AGENTS.md are updated correctly.

---

### Edge Cases

- What happens if CLAUDE.md exists but was authored manually without the auto-update markers? The update script must not overwrite manual content outside the designated markers.
- What happens if speckit-plan is run when CLAUDE.md already references a different branch? The script appends the new branch entry and caps the recent changes list at three entries.
- What happens if both `"ai": "codex"` and CLAUDE.md already exist? The update-all path updates both without duplication.

## Requirements

### Functional Requirements

- **FR-001**: The project MUST have a CLAUDE.md at the repository root serving as Claude Code's entry point, referencing the constitution, specs directory, and Speckit governance workflow.
- **FR-002**: init-options.json MUST support `"claude"` as a valid value for the `"ai"` field alongside the existing `"codex"` value.
- **FR-003**: The speckit-plan SKILL.md MUST invoke `update-agent-context.sh claude` during Phase 1, in addition to the existing `update-agent-context.sh codex` invocation.
- **FR-004**: CLAUDE.md MUST be maintained by update-agent-context.sh, preserving manual additions between the existing marker comments.

## Success Criteria

- **SC-001**: After merging, a developer can open Claude Code at the Traverse repo root and immediately read the governance rules, constitution reference, and active spec guidance from CLAUDE.md with no manual setup.
- **SC-002**: Running speckit-plan on any feature branch results in both AGENTS.md and CLAUDE.md being updated with consistent technical context from that branch's plan.md.
- **SC-003**: Existing CI gate (`cargo test`, `cargo clippy`, spec-alignment checks) passes without modification.
- **SC-004**: init-options.json accepts `"claude"` as a valid `"ai"` value without breaking existing Codex-based flows.

## Assumptions

- Claude Code reads CLAUDE.md from the repository root when initializing a session.
- The existing update-agent-context.sh `claude` case (line 619) is already correct — only the call site in speckit-plan SKILL.md is missing.
- traverse-mcp expansion for Claude Code's MCP surface is out of scope; it requires a separate spec.
- Developers using Codex and Claude Code may coexist; both AGENTS.md and CLAUDE.md can be committed together.

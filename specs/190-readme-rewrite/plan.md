# Implementation Plan: Rewrite README for best-in-class open source

**Branch**: `190-readme-rewrite` | **Date**: 2026-04-07 | **Spec**: [spec.md](spec.md)

## Summary

Rewrite README.md with hero badges, dual human/agent paths, and architecture overview. Update GitHub repo description and topics. Governed by `001-foundation-v0-1`.

## Technical Context

**Language/Version**: Markdown
**Primary Dependencies**: GitHub Actions badge URLs, shields.io
**Project Type**: Documentation
**Constraints**: Must declare `001-foundation-v0-1` in PR body; must not break CI

## Constitution Check

Documentation update governed by approved spec. All gates pass.

## Files touched

```text
README.md                    # MODIFIED — governed by 001-foundation-v0-1
```

GitHub repo metadata (description, topics) updated via `gh repo edit`.

## Phase 0: Research

- CI workflow: `.github/workflows/ci.yml` — single workflow with 4 jobs: `repository-checks`, `coverage-gate`, `pr-hygiene`, `spec-alignment`
- Badge URL pattern: `https://github.com/enricopiovesan/Traverse/actions/workflows/ci.yml/badge.svg`
- License: Apache-2.0
- Rust: 1.94+
- Spec `001-foundation-v0-1` governs README.md per `approved-specs.json`

## Phase 1: README structure

```
badges row
title + one-liner
vision (2–3 sentences)
---
For Humans: quick start (build / test / run)
For Agents: entry points (CLAUDE.md, AGENTS.md, constitution, speckit)
---
Architecture overview (crates table)
Approved specs table
Contributing + links
License
```

## Implementation Sequence

1. Rewrite README.md
2. Update repo metadata via gh repo edit
3. Verify cargo test passes
4. Commit + push + PR declaring spec 001-foundation-v0-1

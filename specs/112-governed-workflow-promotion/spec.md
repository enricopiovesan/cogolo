# Governed Workflow Promotion and Versioning (P4)

**Status**: Approved
**Canonical governing ID**: `112-governed-workflow-promotion`  
**Version**: 0.1.0  
**Depends on**: approved `109-runtime-workflow-proposals`, workflow registry
and application-bundle publication governance.

## Purpose

Define the non-mutating export and human-reviewed promotion path from an
ephemeral proposal to a reusable versioned workflow artifact.

## Requirements

- **FR-001**: Export MUST create a candidate artifact, not a registry or app
  manifest mutation, and MUST retain source proposal/trace provenance without
  secret values, prompts, or private host identifiers.
- **FR-002**: Promotion MUST require explicit review, a new workflow version,
  canonical workflow validation, registry/bundle publication gates, and a new
  application-manifest version where applicable.
- **FR-003**: A promoted workflow MUST have its own immutable identity and
  lifecycle; its source proposal does not confer ongoing execution authority.
- **FR-004**: Yank, rollback, deprecation, and compatibility behavior MUST
  follow existing registry and app-version governance.

## Acceptance scenarios

1. An authorized user exports a completed proposal; no reusable workflow is
   discoverable until review/publication completes.
2. A candidate containing secret/prompt material is rejected or redacted.
3. A published promoted workflow has a new versioned identity and can be
   discovered/executed through existing workflow paths.

## Out of scope

Direct runtime publication, automatic promotion, or bypassing human review.

# Quality Standards

This document defines the operational quality standards for Traverse.

These standards work together with the constitution and feature specs. If there is a conflict, the constitution and approved governing spec take precedence.

## Core Rule

Code is not considered mergeable unless it is:

- aligned with the approved governing spec
- aligned with capability, event, and workflow contracts
- validated by the required automated checks
- maintainable at production quality

## Engineering Standards

All in-scope code must meet these standards:

- Clear module boundaries
- Clear ownership of responsibilities
- Deterministic behavior where practical
- Actionable error handling
- Structured runtime and validation evidence
- Testability by design
- No hidden contract bypasses
- No demo-only hacks in foundation code

## Required Validation Gates

The default validation flow should include:

- spec-alignment validation
- contract validation
- formatting
- linting
- tests
- coverage checks for core logic
- dependency/security checks

Spec-alignment gate implementation:

- approved spec registry: `specs/governance/approved-specs.json`
- workflow job: `spec-alignment`
- script: `scripts/ci/spec_alignment_check.sh`

## Coverage Standard

Required:

- `100%` automated coverage for core business and runtime logic

Core logic includes:

- contract validation
- semver enforcement
- registry behavior
- discovery logic
- ambiguity handling
- workflow traversal
- runtime state machine
- trace generation

Coverage outside core logic should remain appropriate for risk and maintainability.

Coverage gate implementation:

- workflow job: `coverage-gate`
- script: `scripts/ci/coverage_gate.sh`
- protected crate list: `ci/coverage-targets.txt`

The coverage gate is merge-safe even before core logic exists. It passes when no protected crates are configured, and becomes enforcing as soon as core crates are added to `ci/coverage-targets.txt`.

## Reproducibility Standard

Build and validation flows should be reproducible from pinned inputs:

- pinned toolchain
- pinned dependencies
- documented commands
- CI using the same default validation flow expected locally

## Documentation Standard

Public modules and runtime surfaces should document:

- purpose
- inputs and outputs
- major constraints
- failure modes
- examples when useful

## Merge Blocking Conditions

A change must not merge when any of the following are true:

- spec drift is detected
- contract drift is detected
- tests fail
- required validation gates fail
- required coverage for core logic fails
- an unreviewed portability exception exists
- a material architecture change lacks a required ADR

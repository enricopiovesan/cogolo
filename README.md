[<img width="1357" height="440" alt="Screenshot 2026-03-27 at 9 22 54 AM" src="https://github.com/user-attachments/assets/aeafaaf8-650d-4489-bf5e-bd386f0bcaf0" />](https://enricopiovesan.com/)

# Traverse

Traverse is a contract-driven runtime for discovering, validating, and composing portable business capabilities through events, policies, constraints, and graph-based workflows.

## Status

This repository is in v0.1 release-candidate preparation for the first app-consumable Traverse flow.

This project is personal research and development work by Enrico Piovesan,
created on personal time, outside of work, and is not affiliated with,
sponsored by, or related to Autodesk.

The project is currently focused on:

- the first app-consumable browser flow
- the live local browser adapter consumer path
- downstream consumer contracts and validation
- release readiness, checklist, and publication shape
- Rust-first runtime foundations
- WASM-first portable capabilities
- capability, event, and workflow contracts
- registries and deterministic workflows
- structured traces and runtime state
- a React browser demo

## Vision

Traverse treats business capabilities as the primary unit of software.

The long-term goal is to make business logic:

- portable across browser, edge, cloud, and device environments
- governed through contracts and specs
- composable through events and graph-based workflows
- explainable through structured runtime traces
- usable by humans, runtimes, and AI systems safely

## Source of Truth

Traverse is spec-governed.

The governing artifacts are:

- approved versioned specs in [specs](/Users/piovese/Documents/cogolo/specs)
- capability and event contracts
- project constitution in [.specify/memory/constitution.md](/Users/piovese/Documents/cogolo/.specify/memory/constitution.md)

Code, generated artifacts, and tests must align with the approved governing spec version. Pull requests should fail when implementation drifts from spec.

## Key Docs

- Start here: [quickstart.md](/Users/piovese/Documents/cogolo/quickstart.md)
- App-consumable release checklist: [docs/app-consumable-release-checklist.md](/Users/piovese/Documents/cogolo/docs/app-consumable-release-checklist.md)
- App-consumable acceptance: [docs/app-consumable-acceptance.md](/Users/piovese/Documents/cogolo/docs/app-consumable-acceptance.md)
- Release and requirements traceability: [docs/app-consumable-requirements-traceability.md](/Users/piovese/Documents/cogolo/docs/app-consumable-requirements-traceability.md)
- Project direction: [draft.md](/Users/piovese/Documents/cogolo/draft.md)
- Brainstorming decisions: [brainstorming.md](/Users/piovese/Documents/cogolo/brainstorming.md)
- First real youaskm3 integration validation: [docs/youaskm3-integration-validation.md](/Users/piovese/Documents/cogolo/docs/youaskm3-integration-validation.md)
- Quality standards: [docs/quality-standards.md](/Users/piovese/Documents/cogolo/docs/quality-standards.md)
- Compatibility policy: [docs/compatibility-policy.md](/Users/piovese/Documents/cogolo/docs/compatibility-policy.md)
- Adapter boundaries: [docs/adapter-boundaries.md](/Users/piovese/Documents/cogolo/docs/adapter-boundaries.md)
- Contract publication policy: [docs/contract-publication-policy.md](/Users/piovese/Documents/cogolo/docs/contract-publication-policy.md)
- Local runtime home: [docs/local-runtime-home.md](/Users/piovese/Documents/cogolo/docs/local-runtime-home.md)
- Exception process: [docs/exception-process.md](/Users/piovese/Documents/cogolo/docs/exception-process.md)
- ADR guidance: [docs/adr/README.md](/Users/piovese/Documents/cogolo/docs/adr/README.md)

Foundation `v0.1` planning docs:

- Spec: [specs/001-foundation-v0-1/spec.md](/Users/piovese/Documents/cogolo/specs/001-foundation-v0-1/spec.md)
- Plan: [specs/001-foundation-v0-1/plan.md](/Users/piovese/Documents/cogolo/specs/001-foundation-v0-1/plan.md)
- Research: [specs/001-foundation-v0-1/research.md](/Users/piovese/Documents/cogolo/specs/001-foundation-v0-1/research.md)

## Task Management

The canonical task board for this project is:

- [GitHub Project 1](https://github.com/users/enricopiovesan/projects/1/)

Issues and pull requests should be written so they can be tracked there cleanly. See [docs/project-management.md](/Users/piovese/Documents/cogolo/docs/project-management.md).

## Open Source Collaboration

Please read before contributing:

- [CONTRIBUTING.md](/Users/piovese/Documents/cogolo/CONTRIBUTING.md)
- [CODE_OF_CONDUCT.md](/Users/piovese/Documents/cogolo/CODE_OF_CONDUCT.md)
- [SECURITY.md](/Users/piovese/Documents/cogolo/SECURITY.md)
- [SUPPORT.md](/Users/piovese/Documents/cogolo/SUPPORT.md)

## Repository Principles

- Specs are versioned, immutable, and merge-gating once approved.
- Contracts are the source of truth for runtime behavior.
- Portability matters more than host-specific shortcuts.
- Core logic must maintain production-grade quality and full automated coverage.
- Runtime behavior must be explainable through explicit traces and evidence.

## License

This project is licensed under Apache-2.0. See [LICENSE](/Users/piovese/Documents/cogolo/LICENSE).

See [NOTICE](/Users/piovese/Documents/cogolo/NOTICE) for attribution and project disclaimer text.

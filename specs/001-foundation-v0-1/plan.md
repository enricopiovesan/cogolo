# Implementation Plan: Cogolo Foundation v0.1

**Branch**: `001-foundation-v0-1` | **Date**: 2026-03-26 | **Spec**: [spec.md](/Users/piovese/Documents/cogolo/specs/001-foundation-v0-1/spec.md)
**Input**: Feature specification from `/specs/001-foundation-v0-1/spec.md`

## Summary

Build the first production-grade Cogolo foundation as a Rust-first, WASM-centered portable capability runtime. The implementation will establish capability contracts, event contracts, initial registries, deterministic graph-based workflows, a local-only placement abstraction, structured traces, and a runtime state machine, plus a React browser demo that subscribes to runtime events and state transitions.

The plan keeps `Foundation v0.1` intentionally narrow: one runtime model, one local executor, one browser demo, five real capabilities, and no real AI agents yet. The architecture must still remain agent-ready, placement-ready, and portable for future browser, edge, cloud, Android, macOS, and MCP expansion. Approved specs are versioned, immutable implementation-governing artifacts, so the delivery plan must also include spec-alignment validation in the normal build and review flow.

## Technical Context

**Language/Version**: Rust stable toolchain for core runtime and capabilities; TypeScript for React demo  
**Primary Dependencies**: Rust workspace crates, WASM toolchain (`wasm32-wasip1` or equivalent stable WASI target), React, browser event/state subscription layer  
**Storage**: File-based registries and workflow definitions for `v0.1`; no external database  
**Testing**: `cargo test`, browser/frontend tests for React demo, contract and integration tests, CLI integration tests  
**Target Platform**: Local runtime on macOS/Linux-class developer environments plus browser runtime for demo  
**Project Type**: Multi-package runtime/cli/library project with browser demo  
**Performance Goals**: Local capability execution should feel interactive for demo use; runtime state updates should be visible in browser without page reload; structured traces should be produced for every in-scope execution path  
**Constraints**: Portable WASM capability model, local-only executor in `v0.1`, no OS-specific capability bindings by default, 100% coverage for core runtime/business logic, production-grade quality across all code  
**Scale/Scope**: One foundation runtime, three contract/registry types (capability, event, workflow), at least five capabilities, one composed workflow using three or more capabilities, one React browser demo

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- `Capability-First Boundaries`: Pass. Work is centered on capability contracts, workflows, and runtime execution rather than UI-first or CRUD-first design.
- `Contracts Are the Source of Truth`: Pass. Plan treats capability and event contracts as the primary runtime artifacts, with validation before registration and execution.
- `Specs Are Versioned, Immutable, and Merge-Gating`: Pass. Plan includes spec-alignment validation as part of the implementation and review flow.
- `Portability Over Host Coupling`: Pass. Rust + WASM is the default implementation path, and the runtime uses a placement abstraction with `local` executor only for `v0.1`.
- `Discoverability and Governance by Default`: Pass. Capability, event, and workflow registries are part of the core implementation.
- `Runtime Decisions Must Be Explainable`: Pass. Runtime state machine, ambiguity failure handling, and structured traces are core deliverables.
- `Small, Verifiable v0.1`: Pass. Scope is limited to local execution, deterministic workflows, one browser demo, and no real AI agents or distributed orchestration.

No constitution violations currently require exception tracking.

## Project Structure

### Documentation (this feature)

```text
specs/001-foundation-v0-1/
├── plan.md
├── spec.md
├── research.md
├── data-model.md
├── quickstart.md
└── tasks.md
```

### Source Code (repository root)

```text
crates/
├── cogollo-contracts/
│   ├── src/
│   └── tests/
├── cogollo-registry/
│   ├── src/
│   └── tests/
├── cogollo-runtime/
│   ├── src/
│   └── tests/
├── cogollo-mcp/
│   ├── src/
│   └── tests/
├── cogollo-cli/
│   ├── src/
│   └── tests/
└── cogollo-capabilities/
    ├── comment-draft/
    ├── permissions/
    ├── text-improve/
    ├── resource-attach/
    └── notification/

contracts/
├── capabilities/
├── events/
└── workflows/

examples/
├── capabilities/
├── workflows/
└── traces/

apps/
└── browser-demo/
    ├── src/
    └── tests/

tests/
├── integration/
├── contract/
└── e2e/
```

**Structure Decision**: Use a Rust workspace with dedicated crates for contracts, registries, runtime, MCP surface, CLI, and example capabilities. Keep contract artifacts and example workflow definitions in repo-level directories so they remain inspectable and reusable by tests, CLI flows, and the browser demo.

## Delivery Phases

### Phase 0 - Foundations and Research Closure

Goals:

- Confirm Rust workspace strategy and WASM target strategy
- Define the minimum file-based registry persistence approach
- Lock the `v0.1` contract artifact boundaries
- Define the initial spec-alignment validation approach for CI and PR gating
- Define the initial static-analysis, dependency, and reproducibility baseline
- Identify the five initial capabilities and one composed workflow

Outputs:

- `research.md`
- initial repository skeleton
- documented WASM target/toolchain decision

### Phase 1 - Data Model and Contract Definitions

Goals:

- Define capability contract schema
- Define event contract schema
- Define workflow definition and metadata schema
- Define structured trace schema
- Define runtime state machine model
- Define semver and duplicate-version validation rules
- Define compatibility policy, evidence identifiers, and exception record shape

Outputs:

- `data-model.md`
- initial contract JSON schema or equivalent validation model
- example `contract.json` artifacts

### Phase 2 - Core Runtime and Registries

Goals:

- Implement contract validation crate
- Implement spec-alignment validation hooks or checks for CI usage
- Implement capability, event, and workflow registries
- Implement metadata loading and discovery support
- Implement local placement abstraction
- Implement runtime state machine
- Implement ambiguity failure handling and trace generation

Outputs:

- core runtime crates compiling and tested
- CLI registration/list/validate/run flows
- structured trace artifacts for success and failure cases

### Phase 3 - Capabilities, Events, and Workflow Composition

Goals:

- Implement at least five portable WASM capabilities
- Register associated event contracts
- Define and execute at least one deterministic workflow using three or more capabilities
- Validate event-driven capability communication end to end

Outputs:

- five working capabilities
- workflow and event examples
- integration tests for composed flow

### Phase 4 - Browser Demo and Developer Experience

Goals:

- Expose runtime state and event subscriptions for browser use
- Build React browser demo
- Demonstrate runtime loading, execution, event emission, completion, and error handling in UI
- Document quickstart flow

Outputs:

- `quickstart.md`
- working browser demo
- end-to-end validation path from CLI/runtime to browser demo

## Module Responsibilities

### `cogollo-contracts`

Responsible for:

- capability contract parsing and validation
- event contract parsing and validation
- workflow contract/definition parsing and validation
- semantic version rules
- duplicate identity/version rejection

Must have full coverage because it is core logic.

### `cogollo-registry`

Responsible for:

- file-based storage and lookup of capability contracts
- file-based storage and lookup of event contracts
- file-based storage and lookup of workflow definitions
- metadata indexing for runtime discovery

Must have full coverage for registry behavior and duplicate handling.

### `cogollo-runtime`

Responsible for:

- runtime state machine
- discovery and candidate collection
- constraint evaluation
- deterministic selection flow
- ambiguity rejection
- local placement abstraction
- WASM capability execution
- event emission orchestration
- trace generation

Must have full coverage for decision, state, workflow traversal, and trace behavior.

### `cogollo-mcp`

Responsible for:

- initial MCP-friendly runtime surface
- discoverability and invocation-facing interfaces intended for future agent use

Must remain narrow in `v0.1`, but stable enough to semver and validate.

### `cogollo-cli`

Responsible for:

- `register`
- `list`
- `validate`
- `run`
- inspect workflows and event contracts

CLI glue should be well tested, with full coverage where logic is nontrivial.

### `cogollo-capabilities`

Responsible for:

- initial five Rust + WASM capabilities
- example event-driven composition
- workflow-ready sample assets

Capabilities must remain portable and free from direct OS bindings unless explicitly justified.

## Initial Capability Set

Recommended initial set:

- `create-comment-draft`
- `validate-permissions`
- `improve-text-with-ai-ready-interface` or a non-agent placeholder capability shaped for future AI replacement
- `attach-comment-to-resource`
- `send-notification`

Notes:

- The “AI-ready” capability in `v0.1` does not need a real agent implementation.
- It should still be contract-shaped in a way that makes replacement by a real WASM AI capability straightforward in `v0.2`.

## Initial Workflow Shape

Recommended workflow:

- `comment-creation-flow`

Likely traversal:

1. `validate-permissions`
2. `create-comment-draft`
3. `attach-comment-to-resource`
4. `send-notification`

Optional fifth capability can participate directly or via emitted events depending on final contract shapes.

The workflow must be deterministic and graph-based, with explicit capability and event references stored in the workflow registry.

## Verification Strategy

### Unit Coverage

100% coverage required for:

- contract validation
- semver enforcement
- registry behavior
- discovery logic
- ambiguity handling
- workflow traversal logic
- runtime state machine
- trace generation

### Integration Coverage

Required integration validation:

- reject implementation drift from approved governing spec
- register then execute single capability
- register then execute workflow
- emit and validate event contract usage
- reject duplicate contract identity/version mismatch
- reject ambiguous intent-based selection
- reject invalid event/workflow references

### End-to-End Coverage

Required end-to-end paths:

- CLI contract validation and registration
- runtime local WASM execution
- workflow execution with events
- React browser demo subscribing to runtime state and events

### Non-Functional Verification

Required verification of non-functional requirements:

- deterministic behavior for validation, selection, workflow traversal, and trace generation under repeated identical inputs
- actionable failure outputs for invalid contracts, duplicate versions, ambiguity, and runtime execution errors
- preservation of runtime/module boundary separation in crate structure and tests
- browser demo responsiveness sufficient for interactive use during runtime state and event updates
- CI validation that blocks merge on spec drift, failed tests, or missing required evidence
- reproducible build and validation commands from pinned toolchain/dependency inputs
- structured evidence emission suitable for runtime diagnosis and CI review
- compatibility validation for versioned surfaces when changed
- static-analysis and dependency/security checks in the default validation flow

### Non-Negotiable Merge Gates

The default validation flow must block merge when any of the following fail:

- governing spec alignment
- contract alignment
- 100% coverage for core business and runtime logic
- automated validation/test suite
- explicit ambiguity failure behavior
- reviewed portability exceptions
- required decision records for material architectural changes
- reproducibility or static-analysis baseline checks

## Risk Areas

- WASM execution model details may introduce toolchain friction early, especially around stable WASI target choices and browser/runtime parity.
- The browser runtime and React demo may expose assumptions that are safe locally but not yet stable for broader placement targets.
- “Intent-based lookup” must stay simple enough to avoid accidental design drift into a full planner or policy engine.
- The initial AI-ready capability must be carefully framed so it does not silently pull real agent scope into `v0.1`.

## Out of Scope for This Plan

- Real AI agents running as WASM capabilities
- Android demo
- macOS demo
- Edge/cloud executors
- Distributed orchestration
- Full policy marketplace
- Full planner-driven workflow selection
- Federated registries

## Complexity Tracking

No constitution exceptions currently identified.

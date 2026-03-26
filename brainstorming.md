# Cogollo Brainstorming Decisions

This document captures the outcome of the initial product and architecture brainstorming used to shape the first formal specifications.

It is not the final spec. It is the working decision record that narrows the MVP and protects the architecture from unnecessary ambiguity.

## Scope Strategy

Cogollo will be delivered in two stages:

- `Foundation v0.1`
- `MVP v0.2`

`Foundation v0.1` focuses on the runtime and contract foundation.

`MVP v0.2` expands that foundation with richer agent behavior and additional client applications.

### Foundation v0.1 Focus

- Rust-first implementation
- WASM as the portable binary model
- Portable capabilities with no target or OS-specific bindings unless clearly justified
- Capability contracts
- Event contracts
- Runtime and MCP surface
- Basic registries
- Deterministic graph-based workflows
- Local execution through a placement abstraction
- Structured traces
- React browser demo

### MVP v0.2 Focus

- Real AI agents running on top of the stable foundation
- Android demo
- macOS demo
- Richer workflow and agent behavior

## Contract Decisions

### Contract Format

For `v0.1`, the primary capability contract format is:

- required `contract.json`
- optional companion `README.md`

Reasoning:

- machine-first contract truth
- better fit for validation, runtime use, and AI tooling
- still leaves room for human-readable explanation

### Spec Governance

Formal specs for approved work must be:

- versioned
- immutable once approved for implementation
- treated as the source of truth for code generation, manual implementation, and validation

Merge rule:

- pull requests must fail if implementation, contracts, generated code, or tests are not aligned with the governing approved spec

### Semantic Versioning Scope

Semantic versioning applies in `v0.1` to:

- capability contracts
- event contracts
- runtime and MCP surface

## Capability Decisions

### Capability Implementation Model

Capabilities in `v0.1` are implemented as portable WASM binaries.

Primary implementation language:

- `Rust`

Rule:

- capabilities must remain portable
- capabilities must not bind directly to target-specific or OS-specific APIs unless there is a clearly justified exception

### Capability Count

`Foundation v0.1` should include:

- at least `5` real capabilities

These should be small, meaningful business actions and should be sufficient to demonstrate:

- registry value
- event-driven interaction
- composition
- workflow traversal

## Runtime Decisions

### Runtime Selection Model

The runtime should support intent-based lookup.

If multiple capabilities match and the runtime cannot resolve the ambiguity safely in `v0.1`, it must:

- fail explicitly
- emit a structured trace explaining the ambiguity

This keeps discovery real without introducing premature ranking complexity.

### Runtime Placement Model

`Foundation v0.1` should include:

- a real placement abstraction
- contracts that can declare execution constraints and preferred targets

But only this executor is implemented in `v0.1`:

- `local`

This preserves the architecture while keeping the first milestone small.

### Runtime Interface Boundary

The capability execution boundary should follow a WASM-compatible, contract-first runtime model rather than a Rust trait as the architectural source of truth.

Structured input and output should be contract-driven and runtime-facing.

### Runtime State Machine

The initial runtime state machine should cover both lifecycle and decision phases.

Expected state families include:

- `idle`
- `loading_registry`
- `ready`
- `discovering`
- `evaluating_constraints`
- `selecting`
- `executing`
- `emitting_events`
- `completed`
- `error`

This supports explainability, trace generation, debugging, and UI subscriptions.

## Workflow and Graph Decisions

### Workflow Model

`Foundation v0.1` uses:

- graph-based workflow model
- deterministic traversal

It does not yet include full runtime planning or AI-driven workflow selection.

### Workflow Registry

The initial workflow registry should store:

- workflow definitions
- workflow metadata

Metadata should include at least:

- id
- version
- owner
- participating capabilities
- input and output shape
- events used
- tags
- lifecycle or status

## Agent Decisions

### AI Agent Scope

`Foundation v0.1` is:

- agent-ready

`Foundation v0.1` is not:

- agent-heavy

Real AI agents are deferred to `MVP v0.2`.

This means `v0.1` should still prepare for agents by supporting:

- discoverable contracts
- MCP-friendly runtime surfaces
- graph-based workflows
- structured traces

## Demo Decisions

### UI Demos

`Foundation v0.1` should include:

- one React browser demo

This demo should:

- connect to the runtime running in the browser
- subscribe to runtime events
- update the UI from runtime state and events

Deferred to `v0.2`:

- Android demo
- macOS demo

## Quality Decisions

### Coverage Policy

`Foundation v0.1` requires:

- `100%` test coverage for core business and runtime logic

This includes at minimum:

- contract validation
- registry behavior
- decision logic
- workflow traversal
- runtime state machine
- trace generation

Coverage outside core logic may be pragmatic, but should still be appropriate for the feature’s risk.

### Quality Policy

All code must be production-level in quality, including:

- clean architecture
- maintainable code
- deterministic behavior where practical
- robust error handling
- clear boundaries
- testability
- no throwaway demo hacks in the foundation

In addition:

- no code should be merged if it is not validated against the governing approved spec

## Foundation v0.1 Summary

`Foundation v0.1` should prove the following:

- a portable Rust + WASM runtime foundation
- capability contracts as runtime truth
- event-driven capability communication
- initial capability registry
- initial event registry
- initial workflow registry
- graph-based composition
- structured runtime decisions and traces
- runtime state machine and event subscriptions
- one React browser demo connected to the runtime
- at least five real capabilities

## Next Spec Target

The first formal spec should define `Foundation v0.1` around:

- capability contract format
- event contract format
- runtime and MCP surface
- capability registry behavior
- event registry behavior
- workflow registry behavior
- local runtime decision flow
- state machine
- trace output format
- CLI and runtime interactions

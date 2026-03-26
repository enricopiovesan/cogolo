# Research: Cogolo Foundation v0.1

## Purpose

This document captures the implementation-critical decisions for `Foundation v0.1` before task breakdown begins.

The goal is not to explore every possible architecture. It is to choose the smallest durable set of technical decisions that preserves Cogolo’s long-term direction:

- Rust-first
- WASM-first
- portable capabilities
- contract-driven runtime
- event-driven composition
- explainable runtime behavior
- spec-governed implementation and merge validation

## Decision 1: Rust Workspace as the Primary Repository Structure

### Decision

Use a Rust workspace with multiple focused crates:

- `cogollo-contracts`
- `cogollo-registry`
- `cogollo-runtime`
- `cogollo-mcp`
- `cogollo-cli`
- example capability crates

### Rationale

This is the cleanest match for:

- production-grade code quality
- test isolation
- explicit boundaries between contract, registry, runtime, and interface logic
- Rust-first implementation across the platform

It also aligns with the intended architecture from the plan and keeps core logic out of the browser demo.

### Alternatives Considered

#### Single crate

Rejected because:

- boundaries between runtime, registry, and contracts would blur quickly
- testing and ownership would become harder as the system grows

#### Separate repos per major component

Rejected because:

- too much overhead for `v0.1`
- would slow iteration on the foundational contract/runtime model

## Decision 2: WASM as the Default Capability Binary Format

### Decision

Use WASM as the only capability binary format for `Foundation v0.1`.

Rust is the default authoring language for all example capabilities and core runtime components.

### Rationale

This directly supports the architecture we chose:

- portable business capabilities
- no target or OS-specific bindings by default
- future browser, edge, and cloud execution
- alignment with UMA’s portability model

Using one binary model in `v0.1` avoids fragmentation and keeps the runtime honest.

### Alternatives Considered

#### Native binaries plus later WASM support

Rejected because:

- it would weaken the portability promise from the beginning
- the contract and runtime model might accidentally optimize for local process execution instead of portable capability execution

#### Script-based local execution

Rejected because:

- useful for prototyping, but too weak for the architecture we want to validate

## Decision 3: Stable WASI Target for Foundation v0.1

### Decision

Target a stable Rust WASI target for capability binaries in `v0.1`, preferring the current stable target family available in the toolchain rather than inventing a custom execution format.

Current local toolchain:

- `rustc 1.94.0`
- `cargo 1.94.0`

The implementation should use the stable WASI target supported by this toolchain and lock it explicitly in the workspace tooling and documentation.

### Rationale

The exact WASI target naming may evolve over time, but `v0.1` should optimize for:

- stable builds
- reproducible CI
- low friction for contributors
- compatibility with local execution and browser-facing packaging work

The architectural requirement is “portable WASM capability binaries,” not “adopt the newest experimental target name.”

### Alternatives Considered

#### Experimental component-model-first target for all capabilities

Rejected for `v0.1` because:

- it may introduce unnecessary toolchain and integration friction
- the runtime foundation, contracts, and registries are the higher-risk items right now

This can be revisited once the core runtime is stable.

## Decision 4: File-Based Registries for v0.1

### Decision

Use file-based registries for capabilities, events, and workflows in `v0.1`.

Registry state should be inspectable in-repo and test-friendly.

Likely shape:

- `contracts/capabilities/...`
- `contracts/events/...`
- `contracts/workflows/...`

The runtime may build indexed in-memory views over these files at load time.

### Rationale

This is the best fit for `v0.1` because it is:

- simple
- transparent
- easy to diff and review
- easy to test
- compatible with contract-as-artifact thinking

It also avoids adding a database before the contract and runtime model is stable.

### Alternatives Considered

#### Embedded database

Rejected because:

- unnecessary operational complexity for the first milestone
- would make contracts less inspectable during early design and testing

#### External service-backed registry

Rejected because:

- far beyond `v0.1` needs
- would blur the focus on the local foundation

## Decision 5: Contract-First Artifact Model

### Decision

Use required `contract.json` files as the source of truth for capability contracts, with optional human-readable companion Markdown.

Apply the same machine-first approach to event contracts and workflow definitions.

### Rationale

This best matches:

- C-DAD’s machine-first contract model
- runtime discoverability
- CLI validation
- AI-friendly structured artifacts
- future provenance and governance

### Alternatives Considered

#### YAML contracts

Rejected because:

- easier for hand-authoring, but weaker as a strict manifest artifact
- more room for formatting errors and looser conventions

#### Rust traits as the real contract

Rejected because:

- would make Rust source the true boundary instead of the contract
- would reduce portability and weaken runtime discoverability

## Decision 5a: Approved Specs Are Merge-Gating Artifacts

### Decision

Treat approved formal specs as:

- versioned artifacts
- immutable once approved for implementation
- merge-gating sources of truth

Implementation, generated code, contracts, and tests must be validated against the governing approved spec in the normal validation flow.

### Rationale

This matches the product intent that:

- code should not drift from specification
- generated code should not become a second source of truth
- tests should prove alignment with intended behavior rather than only internal implementation assumptions

### Alternatives Considered

#### Specs as advisory documentation only

Rejected because:

- it would allow architecture drift
- it would weaken contract-first development
- it would make spec-driven generation and CI enforcement unreliable

## Decision 6: Deterministic Runtime Selection with Explicit Ambiguity Failure

### Decision

The runtime supports intent-based lookup in `v0.1`, but it does not implement full ranking or planning.

If multiple valid capabilities match an intent and no deterministic resolution rule exists, the runtime must:

- fail explicitly
- emit a structured ambiguity trace

### Rationale

This allows Cogolo to prove:

- discovery is real
- the runtime is a control plane
- traces are meaningful

without prematurely expanding into a full policy-driven decision engine.

### Alternatives Considered

#### Require exact capability IDs only

Rejected because:

- too weak for validating discoverability and runtime reasoning

#### Full scoring and ranking in `v0.1`

Rejected because:

- higher complexity than needed for the first foundation
- risks pulling in broader placement/policy work too early

## Decision 7: Real Placement Abstraction, Local Executor Only

### Decision

Implement a real placement abstraction in `v0.1`, but only ship a `local` executor.

Contracts may still declare:

- execution constraints
- preferred targets
- placement metadata

### Rationale

This preserves the architectural direction toward browser, edge, cloud, and device-aware execution without expanding the first milestone into distributed runtime work.

It also keeps the contract model future-proof.

### Alternatives Considered

#### Local execution with no placement model

Rejected because:

- it would undercut one of the central ideas of Cogolo and UMA

#### Real browser/edge/cloud executors in `v0.1`

Rejected because:

- too much scope and operational complexity for the first milestone

## Decision 8: Event-Driven Capability Communication

### Decision

Capabilities communicate through registered event contracts, and workflows may use those events as part of deterministic graph traversal.

The runtime is responsible for:

- validating event references
- managing event emission in execution flows
- surfacing event activity in traces and browser subscriptions

### Rationale

This is essential to proving Cogolo is more than isolated WASM invocation.

It also connects directly to:

- ECCA-style event governance
- workflow composition
- browser UI subscriptions
- future MCP/agent interaction

### Alternatives Considered

#### Pure direct invocation model in `v0.1`

Rejected because:

- too weak for validating composition and event registry value

## Decision 9: Graph-Based Deterministic Workflows

### Decision

Use graph-based workflow definitions with deterministic traversal in `v0.1`.

Workflow definitions should be explicit artifacts stored in the workflow registry with metadata.

### Rationale

This proves:

- the concept of capability graph
- workflow composition
- discoverability across capabilities and events

without taking on planner-driven behavior yet.

### Alternatives Considered

#### Static ad hoc code paths

Rejected because:

- would hide workflow behavior in implementation instead of making it a first-class artifact

#### Dynamic planning engine

Rejected because:

- more appropriate for later agent-enabled milestones

## Decision 10: Structured Trace as a First-Class Artifact

### Decision

Every in-scope execution path must produce a structured trace artifact.

This includes:

- successful single capability execution
- validation failure
- ambiguity failure
- workflow execution

### Rationale

Traceability is one of the key differentiators of Cogolo’s runtime model.

The trace should serve:

- explainability
- testing
- debugging
- future governance
- future agent reasoning
- browser UI updates

### Alternatives Considered

#### Plain logs only

Rejected because:

- logs are not strong enough as runtime artifacts
- harder to validate and replay

## Decision 11: Explicit Runtime State Machine

### Decision

Define an explicit runtime state machine for `v0.1` covering both lifecycle and execution phases.

Minimum states:

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

### Rationale

This supports:

- predictable runtime behavior
- stronger tests
- browser subscriptions
- explainable execution

It also keeps runtime state from becoming implicit and inconsistent across integrations.

## Decision 12: React Browser Demo as the Only UI in v0.1

### Decision

Ship one React browser demo in `v0.1`.

Defer Android and macOS demo apps to `v0.2`.

### Rationale

The browser demo gives the best early proof that:

- Cogolo can run in the browser context
- runtime state and events can drive UI
- the architecture is truly runtime-centered rather than app-centered

This keeps demo scope manageable while still proving a core UMA idea.

### Alternatives Considered

#### No UI demos in `v0.1`

Rejected because:

- we would lose an important validation of runtime events and browser portability

#### Browser + Android + macOS in `v0.1`

Rejected because:

- too large for the foundation milestone

## Decision 13: Agent-Ready, Not Agent-Heavy

### Decision

`Foundation v0.1` must be agent-ready but will not include real AI agents.

The runtime and MCP surface must still support:

- discoverable contracts
- structured invocation surfaces
- graph-based reasoning inputs
- structured traces

### Rationale

This keeps the architecture aligned with the long-term vision without allowing agent work to dominate the foundation milestone.

### Alternatives Considered

#### Include real agents in `v0.1`

Rejected because:

- too much additional scope and integration risk

## Decision 14: Production-Level Quality Everywhere, 100% Coverage for Core Logic

### Decision

Adopt two separate but complementary quality rules:

- production-level code quality across the whole repository
- 100% automated test coverage for core business and runtime logic

Core logic includes:

- contract validation
- semver enforcement
- registry behavior
- discovery logic
- ambiguity handling
- workflow traversal
- runtime state machine
- trace generation

### Rationale

This preserves rigor without forcing artificial coverage targets on thin glue code or demo-only presentation layers.

## Open Items to Resolve in `data-model.md`

The following should be finalized next:

- exact capability contract field set
- exact event contract field set
- workflow definition structure
- trace JSON structure
- duplicate version comparison rules
- runtime state event payload shape
- browser subscription API shape

## Result

The research supports a clear `Foundation v0.1` direction:

- Rust workspace
- WASM-first portable capabilities
- stable WASI target
- file-based registries
- contract-first machine-readable artifacts
- event-driven composition
- graph-based deterministic workflows
- real placement abstraction with local executor only
- structured traces
- explicit runtime state machine
- one React browser demo

# Cogollo

Cogollo is a portable runtime and control-plane model for business capabilities where contracts define boundaries, events connect capabilities, and workflows are formed by traversing a governed capability graph.

The goal of Cogollo is to make business logic portable, composable, discoverable, and executable across environments such as browser, edge, cloud, and on-device, instead of locking that logic inside applications or platform-specific stacks.

Cogollo is not an application framework. It is a runtime, registry, and composition model for capabilities, contracts, events, and runtime decisions.

## One-Sentence Summary

Cogollo is a contract-driven runtime for discovering, validating, and composing portable business capabilities through events, policies, constraints, and graph-based workflows.

## Architectural Lineage

Cogollo builds on three complementary ideas:

- `UMA`: portable capability execution across environments
- `ECCA`: event contracts as first-class, discoverable, governable assets
- `C-DAD`: contracts as immutable, AI-native, validated units of record

Formal specs are also first-class artifacts in Cogollo. They should be versioned, immutable once approved, and used as merge-gating sources of truth for implementation, generated code, and validation.

Cogollo applies these ideas to business capabilities as the primary unit of software.

## Core Ideas

### Capability

A capability is a portable unit of business logic representing one meaningful business action.

A capability:

- Has a clear business responsibility
- Is defined by a contract
- Declares inputs and outputs
- Declares side effects
- Declares events it emits and consumes
- Declares execution constraints
- Can run in different environments
- Can be composed with other capabilities
- Has a clear owner

Examples:

- `create-comment-draft`
- `improve-text-with-ai`
- `attach-comment-to-resource`
- `send-notification`
- `validate-permissions`

A capability is not a microservice, not a CRUD endpoint, and not a utility function. It is a portable business unit.

### Capability Contract

Contracts define the boundary of a capability and act as the source of truth for humans, AI systems, registries, and runtimes.

A capability contract should include:

- Capability identity and namespace
- Version and lifecycle state
- Inputs and outputs
- Preconditions and postconditions
- Side effects
- Events emitted
- Events consumed
- Permissions required
- Execution constraints
- Policies or policy references
- Owner
- Dependencies
- Validation evidence references
- Provenance metadata
- Human-readable rationale or linked decision records

Contracts are used for:

- Spec-driven development
- AI-assisted development
- Runtime validation
- Capability discovery
- Safe composition
- Governance
- Brownfield extraction
- Provenance and auditability
- Spec-aligned code generation and pipeline enforcement

Contracts define the boundary, and published contracts should be treated as immutable records.

### Event Contract

Events are also first-class contracts, not just payload schemas.

An event contract should include:

- Event identity and namespace
- Schema
- Semantic meaning
- Producer expectations
- Consumer expectations
- Ownership
- Classification and sensitivity
- Version and lifecycle state
- Publication and subscription rules
- Governance metadata

Events enable:

- Loose coupling
- Composition
- Reactive workflows
- Discoverability
- Runtime observability
- Impact analysis

Events connect capabilities, and event contracts should be cataloged and governed like APIs or capabilities.

### Runtime Control Plane

Cogollo’s runtime is not only an execution engine. It is a control plane responsible for turning contracts and metadata into executable decisions.

The runtime should:

- Discover capabilities through the registry and metadata graph
- Evaluate constraints against the current context
- Apply policies to guide selection among valid options
- Select a capability or workflow path
- Execute through the appropriate environment adapter
- Emit a trace explaining how the decision was formed

The runtime decides where and how a capability runs based on factors such as:

- Latency
- Cost
- Data locality
- Security
- Trust level
- Device capabilities
- Execution constraints
- Policy preferences
- Recent runtime conditions or failures

Possible execution environments:

- Browser
- Edge
- Cloud
- On-device
- Worker
- Server

The developer should describe constraints and intent. The runtime should evaluate the best valid execution path.

### Metadata Graph

All capabilities and events form a metadata graph.

- Nodes = capabilities, events, policies, constraints
- Edges = dependencies, event flows, compatibility, ownership, lifecycle, and governance relationships
- Workflows = graph traversal across capabilities and events
- AI agents = planners that reason over the graph
- UIs = consumers of the graph
- APIs = entry points into the graph

Applications become governed graph traversals, not hardcoded flows.

### Policies, Constraints, and Traces

Policies and constraints are first-class artifacts.

- Constraints determine what is valid
- Policies determine what is preferable
- Traces record what was discovered, rejected, selected, executed, and why

Traces are not just logs. They are runtime artifacts that support:

- Explainability
- Auditing
- Reproducibility
- Drift detection
- Observability
- AI reasoning support

## System Layers

Cogollo is composed of several core parts:

| Component | Responsibility |
| --- | --- |
| `runtime-core` | Shared control-plane logic for discovery, evaluation, decision, execution, and tracing |
| `contract-engine` | Validate contracts, schemas, lifecycle, policy conformance, and boundaries |
| `capability-registry` | Store, discover, and govern capability contracts and metadata |
| `event-registry` | Store, catalog, and govern event contracts and metadata |
| `metadata-graph` | Queryable graph of capabilities, events, dependencies, policies, and constraints |
| `policy-engine` | Evaluate preferences and governance rules |
| `constraint-evaluator` | Eliminate invalid options based on context and contract requirements |
| `decision-engine` | Select a valid capability or workflow path deterministically |
| `execution-adapters` | Execute capabilities in browser, edge, cloud, or local environments |
| `trace-engine` | Emit structured traces for decisions and execution |
| `mcp-gateway` | Expose capabilities and reasoning surfaces to agents and external systems |
| `agent` | AI planner that composes workflows from contracts and graph data |

## Capability Granularity Rules

The registry must prevent bad capability boundaries.

A valid capability:

- Represents a business action
- Has clear inputs and outputs
- Can be used in multiple workflows
- Is not just a CRUD operation
- Is not just a utility function
- Is not a full application
- Has clear side effects
- Has a clear owner
- Has a contract
- Can be validated independently

### Capability Size Guide

| Too Small | Good | Too Large |
| --- | --- | --- |
| `sanitize-text` | `improve-comment-text` | `comment-system` |
| `validate-json` | `validate-comment` | `content-platform` |
| `db-insert` | `create-comment` | `user-management-system` |

Rule of thumb: a capability should represent one meaningful business action.

## Brownfield vs Greenfield

### Brownfield

Existing systems can be integrated by:

- Extracting business logic
- Wrapping it in a capability contract
- Capturing dependencies and runtime behavior
- Registering the capability
- Adding validation evidence and provenance
- Letting the runtime execute it where possible

### Greenfield

New systems can be built by:

- Defining capabilities first
- Defining capability and event contracts
- Registering contracts in the registry
- Defining policies and constraints
- Composing workflows from the metadata graph

## v0.1 Scope

Cogollo `v0.1` should stay intentionally small.

### v0.1 Components

- Capability contract spec
- Basic event contract spec
- Contract validator
- Capability registry (basic)
- Event registry (basic)
- Metadata graph (minimal)
- Runtime core (local execution only)
- Constraint evaluation for local execution decisions
- Structured trace output for local execution
- CLI to register a capability
- CLI to list capabilities
- CLI to run a capability through the runtime

### v0.1 Non-Goals

- No distributed orchestration
- No edge/cloud placement engine
- No full policy marketplace or advanced governance workflows
- No full AI planner
- No multi-cloud runtime
- No full workflow engine
- No full UI
- No federated registry mesh

`v0.1` goal: define a capability contract, register it, validate it, discover it, and execute it locally through the runtime with a structured trace.

## Example Flow: Comment System

Goal: a user wants to post a comment.

Possible graph traversal:

1. `authenticate-user`
2. `validate-permissions`
3. `create-comment-draft`
4. `improve-text-with-ai`
5. `attach-comment-to-resource`
6. `persist-comment`
7. `emit-comment-created-event`
8. `send-notifications`

This workflow can be:

- Predefined
- Generated by an AI planner
- Triggered by an event
- Called from UI
- Called from API
- Called from chat

The same capabilities should support different entry points because the graph and contracts are the source of truth.

## Principles

- Business logic should be portable
- Capability and event contracts define boundaries
- Specs are versioned, immutable, and merge-gating sources of truth
- Contracts are the source of truth for humans, AI, and runtimes
- Published contracts should be immutable records
- Events enable composition and observability
- The runtime decides placement and execution path
- Constraints define validity
- Policies define preference
- Traces make runtime behavior explainable
- Capabilities are the unit of software
- Workflows are graph traversal
- UI is a consumer, not the source of truth
- Governance is required for capability and event quality
- AI can plan, but contracts and policies define what is allowed
- Code and generated artifacts must fail CI when they drift from the approved governing spec

## Long-Term Vision

In the long term, Cogollo enables:

- Portable business logic
- Capability and event marketplaces
- AI-generated workflows
- Cross-company capability composition
- Runtime placement optimization
- Traceable and explainable runtime decisions
- Fully inspectable systems
- Contract-driven governance
- Event-driven architecture with structure
- Agents that can understand and use real systems safely
- Federated registries and trust-aware contract exchange

## Initial Repository Direction

```text
cogolo/
├── README.md
├── draft.md
├── .specify/
├── runtime-core/
│   ├── decision-engine/
│   ├── constraint-evaluator/
│   ├── policy-engine/
│   ├── execution-orchestrator/
│   └── trace-engine/
├── adapters/
│   └── local/
├── contracts/
│   ├── capabilities/
│   └── events/
├── registries/
│   ├── capability-registry/
│   └── event-registry/
├── metadata-graph/
├── policies/
├── constraints/
├── scenarios/
└── examples/
```

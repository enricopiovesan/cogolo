# Adapter Responsibilities Versus Core Runtime Responsibilities

Traverse keeps the core runtime narrow and topology-agnostic.

The core model governs:

- capability, event, and workflow contracts
- registry behavior and artifact registration
- runtime request validation and execution state
- trace and runtime evidence generation
- browser-subscription and MCP-facing message contracts

Adapters and sidecar-like helpers may exist around that core, but they are optional integration surfaces, not part of the mandatory runtime shape.

## Core Runtime Responsibilities

The core runtime owns the governed semantic model.

That includes:

- validating governed requests, contracts, and workflow traversal
- selecting approved capabilities and workflows
- applying placement and execution rules
- producing runtime state, trace, and terminal artifacts
- exposing transport-agnostic browser-subscription and MCP-facing payload contracts

These responsibilities must remain stable regardless of whether Traverse is embedded directly in a process, wrapped by a browser adapter, surfaced through MCP, or accompanied by local helper processes.

## Adapter Responsibilities

Adapters exist to connect governed Traverse behavior to a concrete environment.

Adapters may own:

- transport bindings such as HTTP, browser, IPC, or other host-specific delivery paths
- local configuration overlays and environment-specific defaults
- sidecar-like helper behavior for integration convenience
- host-specific bootstrapping, discovery, or packaging glue
- UI- or tool-facing presentation layers that consume governed runtime artifacts

Adapters must not redefine governed runtime semantics. They may shape transport and host integration, but they should reuse the already-governed runtime, trace, placement, browser-subscription, and MCP surfaces.

## Optional Sidecar Behavior

Traverse is not adopting a mandatory sidecar topology.

Why:

- the core runtime is meant to stay portable across browser, edge, cloud, and device environments
- a required sidecar would overcommit Traverse to one deployment shape too early
- mandatory sidecars would blur the boundary between core governed semantics and optional infrastructure glue

Sidecar-style helpers are allowed when useful, but only as optional adapter choices.

Examples:

- a local browser helper that exposes the governed browser-subscription contract over a concrete transport
- an MCP-serving process that wraps the governed MCP surface
- a device or desktop helper that manages local configuration overlays or host integration details

## What Must Stay Transport-Agnostic

The following belong to the core governed surface and should not become adapter-specific inventions:

- runtime request meaning
- state-machine meanings and transition evidence
- trace artifact structure
- browser-subscription message ordering and payload shape
- MCP discovery, get, execute, and observe semantics

An adapter may wrap these with a protocol or deployment model, but it should not rename or redefine them.

## Practical Rule

If a concern changes the meaning of runtime behavior, it belongs to the core runtime.

If a concern only changes how governed behavior is exposed to a host environment, it belongs to an adapter.

## Related Docs

- `docs/oss-pattern-extraction.md`
- `docs/local-runtime-home.md`
- `docs/compatibility-policy.md`
- `specs/013-browser-runtime-subscription/spec.md`
- `specs/014-mcp-surface/spec.md`

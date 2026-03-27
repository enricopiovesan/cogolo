# ADR-0001: Rust and WASM as the Foundation Direction

- Status: Accepted
- Date: 2026-03-26

## Context

Traverse is intended to be a portable runtime for business capabilities that can evolve across browser, edge, cloud, and device environments.

The project needs an implementation foundation that:

- preserves portability
- supports strong contract boundaries
- fits enterprise-grade quality standards
- is suitable for runtime and capability execution
- avoids target-specific lock-in in the first milestone

## Decision

Traverse `Foundation v0.1` will use:

- Rust as the default implementation language
- WASM as the default capability binary format

This applies to:

- core runtime crates
- example capabilities
- runtime-facing execution model

Exceptions are allowed only through the documented exception process and must be explicitly justified and reviewed.

## Consequences

Positive:

- strong portability story from the beginning
- clear alignment with UMA-style runtime goals
- safer and more disciplined systems programming foundation
- easier long-term consistency between runtime and capability implementations

Trade-offs:

- higher early toolchain complexity than script-based execution
- browser/runtime packaging details may require extra care
- some integrations may need adapters instead of direct host bindings

## Alternatives Considered

### Native binaries first

Rejected because it weakens the portability model too early.

### Script or local-command execution first

Rejected because it is too weak for validating the real Traverse architecture.

### Polyglot capability model from day one

Rejected because it adds complexity before the contract/runtime model is stable.

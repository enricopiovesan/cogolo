# ADR-0032: Adopt a No-Std WASI Guest Profile for Expedition Execution

- Status: Accepted
- Date: 2026-08-04
- Governing spec: `091-no-std-wasi-guest-profile`
- Owner: Traverse maintainers
- Review by: 2026-11-04

## Context

The audited FFI exception in ADR-0031 correctly limits application I/O to
`fd_read`, `fd_write`, and `proc_exit`. Compiled evidence nevertheless showed
that Rust `std` startup imports `environ_get`, which ABI v1 denies before guest
logic begins.

## Decision

Use a guest-local `no_std` plus `alloc` profile for the expedition component.
The guest owns a bounded allocator and deterministic panic/termination path.
It retains only the three ADR-0031 WASI imports. No Host ABI v1 change is made.

## Consequences

The expedition guest can remain least-authority compatible, but its allocator,
panic behavior, memory limit, and artifact import evidence are now mandatory
review and test subjects. A reusable no-std runtime or any further import needs
a successor ADR and security review.

## Alternatives Considered

- Add `environ_get`: rejected; it broadens ambient authority.
- Keep `std` and accept import validation failure: rejected; it cannot execute.
- Build a shared guest framework: deferred; it expands scope beyond #921.

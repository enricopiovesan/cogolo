# Production Readiness Boundary Contracts

- `prepare`: host supplies network-capable source and cache writer; returns a
  verified immutable lock/cache generation plus evidence.
- `init` and `execute`: accept only active verified local generation; perform no
  network access.
- `activate` and `rollback`: host-directed, atomic generation state changes.
- `migrate` and `restore`: host-directed v1-to-v2 state transitions with
  verified backup and stable safe errors.
- `trace.list`, `trace.get`, and `trace.export`: workspace-authorized safe
  projections only; exports are explicit and redacted.

Exact public API shapes belong to the bounded successor specs.

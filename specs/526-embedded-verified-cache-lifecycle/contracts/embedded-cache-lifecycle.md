# Embedded Cache Lifecycle Boundary Contracts

- `prepare(input, host_writer)` returns a complete verified candidate or a
  stable error; only this operation may receive a network-capable host source.
- `activate(candidate_id)` and `rollback(generation_id)` are explicit,
  host-directed atomic transitions.
- `init(active_generation, host_reader)` and execution are offline-only and
  consume no source other than the active verified generation.
- `evaluate_lifecycle(lock, locally_known_state)` returns deterministic stale
  or security-yank evidence without network access.

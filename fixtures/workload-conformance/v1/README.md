# Governed workload-conformance fixture v1

This repository-controlled, deterministic fixture is the bounded validation
corpus for issue #1234. It exercises a native, stateless echo capability under
sequential and bounded-parallel load, checks a declared concurrency overload,
and records terminal-success trace evidence through the durable trace journal.

The fixture pins its own version, governed spec identities, artifact and
contract identities, iteration bounds, concurrency bounds, expected overload
code, and safe projection version. It intentionally contains no input payloads,
credentials, host paths, or performance SLOs.

Run it with:

```bash
cargo test -p traverse-runtime --test workload_conformance -- --nocapture
```

The runner emits a single aggregate JSON evidence line containing latency
distribution, peak RSS when the host can provide it, queued/rejected work,
terminal outcomes, recovery projection, and host/engine context. Latency and
memory are reported for comparison; correctness, bounded concurrency, overload
rejection, and restart recovery are the pass/fail checks.

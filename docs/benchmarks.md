# Traverse Benchmarks

Traverse makes a portability trade-off: the same WASM capability runs on macOS, Linux, Android, and edge without a container runtime. Portability has a cost. This page documents what we measure and how to interpret it.

## How to Run

```bash
bash benchmarks/run.sh
```

Results are written to `benchmarks/results/summary.json`. See [`benchmarks/README.md`](../benchmarks/README.md) for full options.

## What Is Measured

### Cold Start

Time from `traverse-cli expedition execute <request>` invocation to process exit (success path).

This includes:
- CLI binary startup
- Bundle manifest load and contract validation
- Capability registry lookup
- WASM module initialization
- Capability execution (stdin/stdout JSON round-trip)
- `RuntimeTrace` serialization

**Interpretation**:
- Under 500ms: acceptable for tool invocations and autonomous agent pipelines
- Over 1000ms: check that you are running a release build (`cargo build --release`)

### Steady State

The same command run 20 times in sequence. Measures repeated execution latency with the binary already warm in the OS page cache.

**Interpretation**:
- Under 100ms: comparable to a local HTTP service call
- Over 500ms: likely the WASM executor or JSON trace serialization — file an issue

## Hardware Assumptions

Results vary significantly across hardware. The benchmark records platform and git SHA in `summary.json`. Always include this metadata when sharing numbers.

Do not compare results across:
- Apple Silicon vs Intel Mac
- macOS vs Linux
- Debug vs release builds

## Docker Comparison

Traverse is not a container replacement. The comparison is asymmetric by design — see [`benchmarks/results/baseline-reference.md`](../benchmarks/results/baseline-reference.md) for the full methodology.

Key asymmetry: the benchmark measures CLI cold start (new process per invocation). A Docker microservice stays warm between requests, so Docker wins on steady-state latency for server workloads. Traverse wins when:
- No container daemon is available (mobile, edge, offline)
- The same binary must run on multiple OS/arch targets
- Governed immutable contracts are required per execution

See [`docs/why-not-docker.md`](why-not-docker.md) for the full decision matrix.

## Reproducibility

- Input: `benchmarks/fixtures/benchmark-request.json` — pinned, deterministic
- No network calls during measurement
- `benchmarks/results/summary.json` is gitignored — check it in manually when publishing
- Re-run with `bash benchmarks/run.sh` to regenerate

## Regression Gate

`benchmarks/check-regression.sh` reads the checked-in baseline (`benchmarks/results/baseline.json`) and compares it against the current `summary.json` produced by `run.sh`.

- Default threshold: 15% mean increase per metric
- Override threshold: `TRAVERSE_BENCH_THRESHOLD_PCT=10 bash benchmarks/check-regression.sh`
- To update the baseline after an intentional performance change: `bash benchmarks/update-baseline.sh`

The gate is currently optional (not a required CI check). Run it manually before merging changes to the runtime:

```bash
bash benchmarks/run.sh
bash benchmarks/check-regression.sh
```

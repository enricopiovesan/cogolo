# Traverse Benchmarks

Measures cold-start latency and steady-state execution latency for the Traverse CLI.

## Quick Start

```bash
bash benchmarks/run.sh
```

Results are written to `benchmarks/results/summary.json`.

## What Is Measured

| Scenario | Command | What it captures |
|---|---|---|
| Cold start | `traverse-cli expedition execute` (first run) | Capability resolution + WASM load + execution |
| Steady state | `traverse-cli expedition execute` (repeated) | Repeated execution latency with trace enabled |

## Hardware / OS Assumptions

- macOS (Apple Silicon or Intel) or Linux x86-64
- No network calls — all inputs are local fixtures
- No background compilation — binary must be pre-built before measuring
- Results vary with hardware; always record the platform in `summary.json`

## Reproducibility

- Input fixtures are pinned in `benchmarks/fixtures/`
- `run.sh` records the git SHA at measurement time
- Results are gitignored — check in `summary.json` manually when publishing benchmark data

## Baseline Comparison

See `benchmarks/results/baseline-reference.md` for the Docker comparison methodology.

## Regression Gate

After running `benchmarks/run.sh`, check for latency regressions against the checked-in baseline:

```bash
bash benchmarks/check-regression.sh
```

To update the baseline after an intentional change: `bash benchmarks/update-baseline.sh`

## See Also

- [`docs/benchmarks.md`](../docs/benchmarks.md) — interpretation guide
- [`docs/why-not-docker.md`](../docs/why-not-docker.md) — when to use Traverse vs Docker

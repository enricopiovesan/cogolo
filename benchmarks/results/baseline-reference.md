# Baseline Comparison Reference

This document explains the Docker/container comparison methodology for Traverse benchmarks.

## Comparison Scenario

We compare Traverse's `expedition execute` cold-start latency against the equivalent overhead of running a minimal containerized service:

| Step | Traverse | Docker equivalent |
|---|---|---|
| Load binary | `target/release/traverse-cli` startup | `docker run` + image pull (cached) |
| Load registry | Parse + validate JSON bundle | N/A (runtime config) |
| Resolve capability | Registry lookup + placement | Service discovery / DNS |
| Execute | WASM module load + stdin/stdout | HTTP request to container |
| Trace | Serialize `RuntimeTrace` to JSON | Application logging |

## Why This Comparison Is Not Apples-to-Apples

Traverse and Docker solve different problems. The comparison is intentionally asymmetric:

- Docker `cold start` includes container daemon overhead (already running) but **not** image build time.
- Traverse `cold start` includes registry load and WASM module initialization but **not** Cargo build time.
- Docker services stay warm between requests; Traverse CLI spawns a new process per invocation in the benchmark (worst case for Traverse).

## Methodology

Live Docker numbers are not included in this file — running them requires a Docker daemon and network access, violating the "no network" reproducibility constraint. Reference numbers from community benchmarks:

- Minimal `docker run hello-world` (cached): ~50–200ms on macOS (varies with Docker Desktop overhead)
- Minimal `docker run` + HTTP request to a FastAPI echo service: ~100–400ms cold, ~5–15ms steady-state

## What the Traverse Numbers Mean

- **Cold start < 500ms**: acceptable for tool invocations, autonomous agent pipelines
- **Cold start > 1000ms**: investigate — likely caused by debug build or large WASM module
- **Steady-state < 100ms**: comparable to a local HTTP microservice call
- **Steady-state > 500ms**: investigate — likely the WASM executor or JSON serialization

## Hardware Assumptions

Record the platform string from `summary.json` alongside any published results. Results on Apple M-series differ significantly from Intel Macs and Linux x86. Do not compare numbers across platforms.

# Killer Use Case: Portable Knowledge Workflow (Browser + Cloud)

This repository is easiest to understand when you start from a real constraint:

You want one governed knowledge workflow to run:

- offline in the browser (fast, private, resilient)
- and also in cloud or edge environments (automation, sharing, federation)

Most teams solve this by splitting the implementation:

- one code path in the web app (often JS/TS + bespoke storage)
- a separate service in the cloud (often a different language/runtime)
- glue code that drifts over time, plus duplicated validation logic

Traverse targets a different outcome: keep the business capability portable and governed, then swap the host around it.

## What “Success” Means (Measurable)

We consider this use case successful when we can demonstrate:

- One capability contract and one workflow definition that run in at least two hosts.
- Deterministic validation: the same artifacts either pass or fail in both hosts.
- Traceability: a runtime trace that is consistent across hosts and can be consumed by downstream apps.

Performance claims are tracked separately under benchmarks (see: #326).

## Where to Start in This Repo

- Downstream validation path: [docs/youaskm3-integration-validation.md](docs/youaskm3-integration-validation.md)
- Consumer release surface: [docs/app-consumable-consumer-bundle.md](docs/app-consumable-consumer-bundle.md)
- First app-consumable entry path: [docs/app-consumable-entry-path.md](docs/app-consumable-entry-path.md)

## What This Use Case Forces Us to Build Next

This use case is the forcing function for several “industry standard” requirements:

- Observability that works in non-traditional hosts (OpenTelemetry traces/logs) (#329)
- A stable insulation layer as WASI/component model evolves (#330)
- Contract enforcement that blocks invalid artifacts early (#332)


# Why Not Docker? — When Traverse Is (and Isn't) Worth It

This is a blunt decision guide. Traverse has real costs. Use this page to self-select quickly.

---

## The Short Answer

**Use Docker when:** you have a standard server environment and don't need portability beyond Linux containers.

**Use Traverse when:** you need the same governed computation to run on macOS, Android, cloud, and offline edge — without a container runtime — and you need autonomous agents or LLMs to discover and compose those computations safely.

---

## Decision Matrix

| Scenario | Docker / Containers | Traverse |
|---|---|---|
| Standard cloud microservice | ✅ Simpler, well-understood | ❌ Overkill |
| Already running in Kubernetes | ✅ Stay there | ❌ Adds complexity without payoff |
| Offline / edge / device (no container runtime) | ❌ Can't run without daemon | ✅ WASM runs natively |
| macOS + Android + Linux from one artifact | ❌ Separate builds required | ✅ Single WASM binary |
| Immutable governed contracts per version | ❌ No built-in contract system | ✅ Core feature |
| LLM/agent discovers and calls capabilities | ❌ No governed discovery surface | ✅ MCP + registry |
| Offline-first portable knowledge app | ❌ Requires daemon | ✅ Designed for this |
| Regulated environment needing provenance | ⚠️ Possible with extra tooling | ✅ Built-in digest + immutability |
| Team already on Docker Compose | ✅ Don't change it | ❌ Migration cost not worth it |
| Simple CRUD service | ✅ Rails / FastAPI / Express wins | ❌ Way too much governance overhead |
| CI/CD pipeline steps | ✅ Docker is standard here | ⚠️ Possible but unusual |

---

## Two Concrete Examples

### Example 1: Docker is the right answer

**Scenario**: A team building a REST API backend for a SaaS product running on AWS ECS.

- Single target runtime: Linux x86 containers
- No portability requirement beyond AWS regions
- Standard HTTP/JSON traffic
- Team already knows Docker Compose and ECR

**Decision: Use Docker.** Traverse adds contract governance and WASM portability overhead that provides zero benefit here. Ship faster with what you know.

---

### Example 2: Traverse is uniquely valuable

**Scenario**: A knowledge app that runs AI-powered reasoning on a user's local macOS machine, syncs to a cloud backend, and has a read-only mode on Android when offline.

- **Same reasoning capability must run on**: macOS (native), cloud (Linux), Android (offline, no daemon)
- **Autonomous agent** (LLM) must discover which capability handles "summarize this document" without knowing the deployment target
- **Governance**: each reasoning step must be auditable — which version ran, on what input, with what output
- **Immutability**: a v1.2.0 run must be reproducible 6 months later (same digest, same output given same input)

**Decision: Traverse.** This is the exact use case it was designed for. A WASM capability registered at v1.2.0 runs identically on all three targets. The MCP surface lets the LLM discover it. The trace artifact records the full governed execution.

---

## Non-Goals

Traverse is **not**:

- A microservices platform or container replacement for standard server workloads
- A Kubernetes alternative
- A general-purpose RPC framework
- A database or persistence layer (it defines contracts for data access, but does not implement storage)
- A build system or CI/CD tool
- A performance-first runtime (portability has overhead — see [`docs/benchmarks.md`](benchmarks.md))

---

## The Portability Cost

Portability is not free. Traverse adds:

- **Cognitive overhead**: governed contracts, manifests, digests, spec alignment
- **Toolchain overhead**: WASM compilation target, WASI host bindings
- **Runtime overhead**: WASM startup vs native binary (see [`docs/benchmarks.md`](benchmarks.md) for measured numbers)

If you don't need what portability buys, don't pay the cost.

---

## Related Docs

- [`docs/benchmarks.md`](benchmarks.md) — measured latency numbers
- [`docs/what-can-i-build.md`](what-can-i-build.md) — concrete patterns Traverse supports today
- [`docs/architecture-execution-models.md`](architecture-execution-models.md) — the three execution surfaces
- [`quickstart.md`](../quickstart.md) — first runnable flow

# OSS Pattern Extraction for Traverse

This note captures reusable implementation patterns from the checked-out open-source references under `references/open-source/` and translates them into concrete guidance for Traverse.

Reviewed references:

- Spin:
  - `references/open-source/spin/README.md`
  - `references/open-source/spin/templates/redis-go/content/spin.toml`
- Dapr:
  - `references/open-source/dapr/README.md`
  - `references/open-source/dapr/docs/decision_records/cli/CLI-002-self-hosted-init-and-uninstall-behaviors.md`
- Flogo Core:
  - `references/open-source/flogo-core/docs/model.md`
  - `references/open-source/flogo-core/trigger/config_schema.json`
- Polyglot microservices:
  - `references/open-source/polyglot-microservices/README.md`
  - `references/open-source/polyglot-microservices/docker/docker-compose.yml`

## Adopt

### 1. Package manifest plus colocated build command

Spin keeps the runnable application shape compact:

- application metadata at the top
- trigger declaration separate from component declaration
- each component points at a concrete WASM source artifact
- build instructions live next to the component manifest

Traverse should adopt this discipline for executable capability packages:

- one checked-in package manifest per executable capability implementation
- one deterministic local build command next to the package
- one explicit binary path and digest in the governed package metadata

This is already proving useful in `#54`, and should become the standard pattern for future executable capability examples.

### 2. Platform-owned local runtime home

Dapr’s self-hosted CLI note is valuable because it standardizes where local runtime binaries, default components, and config live instead of scattering them across arbitrary paths.

Traverse should adopt the same principle:

- one default local runtime home for generated runtime helpers
- one predictable location for local packaged-binary caches or generated fixtures
- one predictable location for local runtime config overlays

This keeps developer workflows reproducible and makes smoke paths easier to script.

### 3. Pluggable infrastructure behind stable app-facing contracts

Dapr’s strongest reusable idea is not “use a sidecar everywhere,” but “hide infrastructure choice behind stable contracts and pluggable components.”

Traverse should adopt:

- stable runtime-facing capability, event, and workflow contracts
- separately pluggable adapters for infrastructure concerns
- explicit governance around what is pluggable and what is part of the core runtime model

This aligns well with Traverse’s contract-first direction without copying Dapr’s full sidecar model.

## Adapt

### 1. Trigger-to-action composition should become event-edge-to-capability progression

Flogo models triggers, handlers, and actions cleanly, including:

- shared actions
- inline actions
- conditional handler action selection
- application-level shared properties and schemas

Traverse should adapt this idea, not copy it literally:

- keep event-driven workflow edges as the primary composition model
- allow reusable capability references the way Flogo allows shared actions
- keep conditional branching governed and explicit, similar to handler action choice
- prefer shared schemas and typed payload contracts over ad hoc per-edge payload logic

The right Traverse equivalent is governed workflow/event composition, not a generic trigger/action engine.

### 2. Polyglot boundaries matter more than polyglot repos

The polyglot microservices reference is useful because it shows common boundary expectations repeated across languages:

- documented service responsibilities
- Dockerized execution
- health and tracing assumptions
- gateway and service-discovery boundaries

Traverse should adapt the boundary discipline, not the repo shape:

- keep portable contract boundaries language-neutral
- require consistent packaging and validation expectations for every implementation
- keep deployment/runtime concerns separate from governed artifact identity

Traverse should not mirror the “many services in one giant repo” structure as a product pattern.

### 3. Sidecar behavior should become optional adapter behavior

Dapr proves the value of keeping application code simple by moving infrastructure mechanics into a companion runtime.

Traverse should adapt this carefully:

- browser, MCP, device, and local adapters can play a sidecar-like role
- the governing contract should remain transport-agnostic
- sidecar-style helpers should stay optional deployment choices, not the core model

This keeps portability while avoiding a premature hard dependency on one deployment topology.

## Reject

### 1. Do not copy Dapr’s sidecar-everywhere runtime shape

Traverse already has a narrower portability and governance goal. A mandatory sidecar model would add too much operational shape too early and would blur the runtime-vs-adapter boundary we have been trying to keep clean.

### 2. Do not copy Flogo’s contribution/import model directly

Flogo’s trigger and action contribution system is useful as inspiration, but Traverse should not adopt free-form runtime imports as a primary extension mechanism. Traverse should stay centered on governed artifact registration, not code-import extensibility.

### 3. Do not copy the polyglot reference repo’s deployment sprawl

The polyglot example is helpful for spotting cross-language boundary conventions, but its Docker Compose sprawl and many-framework layout are not a good direct pattern for Traverse MVP implementation work.

## Immediate recommendations

1. Standardize one governed executable-package layout for future capability implementations.
2. Add one documented local runtime home for generated helper artifacts and adapter config.
3. Keep future adapter work focused on optional integration surfaces rather than mandatory sidecars.
4. Reuse the event-edge composition model instead of introducing a second generic trigger/action runtime model.

## Follow-up ticket recommendations

1. Add a ticket to standardize a local Traverse runtime home for generated binaries, fixtures, and adapter config.
2. Add a ticket to template executable capability package manifests and deterministic build commands.
3. Add a ticket to document adapter responsibilities versus core runtime responsibilities, explicitly calling out why Traverse is not adopting a mandatory sidecar model.

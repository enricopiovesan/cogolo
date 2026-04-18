# Capability Contract Authoring Guide

This guide covers how to author a valid capability contract for Traverse and explains every governed field in `execution.constraints`.

Use the checked-in examples as living references:

- [`contracts/examples/expedition/capabilities/`](../contracts/examples/expedition/capabilities/)
- [`contracts/examples/hello-world/capabilities/say-hello/contract.json`](../contracts/examples/hello-world/capabilities/say-hello/contract.json)
- [`specs/002-capability-contracts/data-model.md`](../specs/002-capability-contracts/data-model.md)

## Contract Structure

A capability contract is a `contract.json` artifact placed under `contracts/`. The top-level shape must include all required fields defined in spec `002-capability-contracts`. The key governed sections are:

- `kind` — must be `capability_contract`
- `schema_version` — must be `1.0.0` for v0.1
- `id`, `namespace`, `name` — identity triple; `id` must equal `namespace.name`
- `version` — semantic version `MAJOR.MINOR.PATCH`
- `lifecycle` — see lifecycle enum below
- `execution` — binary format, entrypoint, preferred targets, and constraints

## Lifecycle Values

| Value        | Meaning                                              |
|--------------|------------------------------------------------------|
| `draft`      | Not publishable for runtime use                      |
| `active`     | Eligible for runtime use                             |
| `deprecated` | Still valid but discouraged for new composition      |
| `retired`    | No longer eligible for new runtime selection         |
| `archived`   | Retained as historical record only                   |

Only `active` and `deprecated` are considered runtime-eligible.

## Constraint Reference

Every capability contract's `execution` block must include a `constraints` object with exactly three fields. These fields describe the security and portability posture of the capability at runtime.

```json
"constraints": {
  "host_api_access": "none",
  "network_access": "forbidden",
  "filesystem_access": "none"
}
```

The following tables document all valid values, their runtime meaning, and whether the runtime enforces the constraint or treats it as a declaration.

### `host_api_access`

Controls whether the WASM module may call host-provided APIs beyond standard WASI.

| Value                | Description                                                                                              | Runtime enforcement |
|----------------------|----------------------------------------------------------------------------------------------------------|---------------------|
| `none`               | The capability makes no calls to host-specific APIs. Fully portable across all execution targets.        | Documentation-only in v0.1. The runtime does not inspect WASM imports at execution time; authors are responsible for keeping the binary portable. |
| `exception_required` | The capability requires host API access for a justified reason. An approved portability exception reference must be present in `provenance.exception_refs`. | Structurally enforced: validation rejects any contract that declares `exception_required` without at least one entry in `provenance.exception_refs`. |

**Source**: Formally defined in spec `002-capability-contracts` (`data-model.md`, field `execution.constraints.host_api_access`) and implemented as `enum HostApiAccess` in `crates/traverse-contracts/src/lib.rs`.

**Design note**: `none` is the default and required value for all portable capabilities. `exception_required` triggers a governance checkpoint — the approved exception reference must identify the reason and scope of the host dependency.

---

### `network_access`

Controls whether the WASM module may open outbound network connections.

| Value       | Description                                                                                          | Runtime enforcement |
|-------------|------------------------------------------------------------------------------------------------------|---------------------|
| `forbidden` | The capability must not make outbound network calls. Required for all portability-first capabilities. | Documentation-only in v0.1. The runtime does not apply a WASI network sandbox at execution time; the binary must not call network APIs. |
| `required`  | The capability explicitly requires outbound network access to function.                               | Documentation-only in v0.1. Authors must justify this in `description` and avoid co-locating with `host_api_access: none` without deliberate design intent. |

**Source**: Formally defined in spec `002-capability-contracts` (`data-model.md`, field `execution.constraints.network_access`) and implemented as `enum NetworkAccess` in `crates/traverse-contracts/src/lib.rs`.

**Design note**: All existing expedition and hello-world example contracts use `"network_access": "forbidden"`. This is the expected value for pure computation capabilities that receive all needed data through their JSON input schema.

---

### `filesystem_access`

Controls whether the WASM module may access the host filesystem.

| Value          | Description                                                                                                           | Runtime enforcement |
|----------------|-----------------------------------------------------------------------------------------------------------------------|---------------------|
| `none`         | The capability does not access the filesystem. Fully portable across execution targets with no filesystem assumption.   | Documentation-only in v0.1. The runtime does not pre-open directories or restrict filesystem WASI imports; authors must ensure the binary does not call filesystem APIs. |
| `sandbox_only` | The capability may access a sandboxed directory provided by the host runtime, scoped to the current execution context. | Documentation-only in v0.1. The specific sandbox path or directory pre-open policy is defined by the host executing environment, not by the contract itself. |

**Source**: Formally defined in spec `002-capability-contracts` (`data-model.md`, field `execution.constraints.filesystem_access`) and implemented as `enum FilesystemAccess` in `crates/traverse-contracts/src/lib.rs`.

**Design note**: All existing example contracts use `"filesystem_access": "none"`. Authors declaring `sandbox_only` must document the expected sandbox layout in the capability's `description` or a companion `README.md`.

---

### Summary table

| Field               | Valid values                          | Required | Formally specified in |
|---------------------|---------------------------------------|----------|-----------------------|
| `host_api_access`   | `none`, `exception_required`          | Yes      | `002-capability-contracts` spec + Rust enum |
| `network_access`    | `forbidden`, `required`               | Yes      | `002-capability-contracts` spec + Rust enum |
| `filesystem_access` | `none`, `sandbox_only`                | Yes      | `002-capability-contracts` spec + Rust enum |

All three fields were inferred from both the spec data model and the Rust implementation in `crates/traverse-contracts/src/lib.rs`. No constraint values in v0.1 were inferred from examples alone; all are formally enumerated in the governing spec.

### Portability rule

If `host_api_access` is set to `exception_required`, validation will reject the contract unless `provenance.exception_refs` contains at least one non-empty reference string. This is the only constraint field that produces a hard validation error at authoring time rather than a documentation-only declaration.

### v0.1 enforcement status

In v0.1, constraint fields are structural declarations — the runtime does not apply WASM import filtering, network sandboxing, or filesystem directory restrictions automatically. Authors are responsible for ensuring their compiled WASM binary matches its declared constraints. Full runtime enforcement is deferred to a future governed slice.

## Authoring Steps

1. Start from the identity triple: choose `namespace`, `name`, and compute `id = namespace.name`.
2. Set `lifecycle: draft` until the contract passes validation.
3. Fill in `inputs`, `outputs`, `preconditions`, `postconditions`, and `side_effects` to describe the full business boundary.
4. Set `execution.binary_format: wasm` (only supported value in v0.1).
5. Set `execution.entrypoint.kind: wasi-command` and `command: run`.
6. Choose `execution.preferred_targets` — at minimum `["local"]`.
7. Set all three `execution.constraints` fields using the values documented above.
8. If `host_api_access: exception_required`, add the exception reference to `provenance.exception_refs`.
9. Run `cargo run -p traverse-cli -- validate contracts/path/to/contract.json` to confirm the contract passes structural and semantic validation.
10. Run `bash scripts/ci/spec_alignment_check.sh` before opening a PR.

## Validation

Run the spec-alignment gate before merging any contract change:

```bash
bash scripts/ci/spec_alignment_check.sh
bash scripts/ci/repository_checks.sh
```

## Related Documents

- [`specs/002-capability-contracts/spec.md`](../specs/002-capability-contracts/spec.md)
- [`specs/002-capability-contracts/data-model.md`](../specs/002-capability-contracts/data-model.md)
- [`docs/wasm-io-contract.md`](wasm-io-contract.md)
- [`docs/wasm-agent-authoring-guide.md`](wasm-agent-authoring-guide.md)
- [`docs/wasm-microservice-authoring-guide.md`](wasm-microservice-authoring-guide.md)

# Feature Specification: Portable Authority Contracts for Audio and Model Runtime

**Status**: Draft
**Canonical governing ID**: `1259-portable-authority-contracts`
**Version**: 0.1.0
**Extends**: `039-connector-plugin-architecture`, `103-application-connector-binding`, and `104-mediated-connector-invocation`.
**Decision evidence**: Traverse #1259 decision records (2026-09-07).

## Purpose and boundary

Define portable connector contracts for unavoidable physical audio input and
model-runtime authority. The contracts name generic operations, not vendors,
models, devices, codecs, applications, or user interfaces. Applications own
workflow and binding selection; hosts own implementations, credentials,
endpoints, permissions, discovery, device selection, and raw bytes.

Portable audio/DSP algorithms and model request/policy preparation remain WASM
capabilities. Advisory/review exchange composes through `traverse.http` until
cross-application evidence justifies a distinct typed authority.

## Requirements

- **FR-001**: `traverse.audio-input` MUST be a native-only connector with
  bounded generic capture requests. Its request/response envelope MUST declare
  duration/size limits, correlation, cancellation, and a content reference or
  host-mediated result; it MUST NOT contain device IDs, microphone names,
  codec vendors, paths, credentials, or application workflow fields.
- **FR-002**: Audio-input discovery, permission prompts, device selection, and
  byte capture MUST remain host-owned. Browser activation of a native-only
  binding MUST reject deterministically before invocation with a stable,
  secret-free target-incompatible error.
- **FR-003**: `traverse.model-runtime` MUST be a vendor-neutral connector for
  activating and executing a declared model artifact. Its typed envelope MUST
  cover artifact identity, licence/policy reference, resource limits, label
  schema, target, correlation, cancellation, quotas, and trace constraints.
  It MUST NOT contain provider choice, model ID, endpoint, credential, or
  application-specific taxonomy.
- **FR-004**: A portable capability MAY prepare model requests or evaluate
  model policy, but it MUST invoke an activated model-runtime binding for
  provider/runtime authority. Connector activation MUST verify compatible
  contract version, configured host implementation, policy/permission, and
  target before execution.
- **FR-005**: Portable codec/DSP algorithms MUST publish as WASM capabilities.
  A media-generic acceleration connector MAY be proposed only for an
  unavoidable host/hardware operation and must expose bounded abstract media
  operations, targets, limits, cancellation, and redacted evidence.
- **FR-006**: Connector operations MUST define stable secret-free failure
  classes for unbound, incompatible, unconfigured, policy-denied,
  target-incompatible, quota-limited, cancelled, and replay/idempotency cases
  that apply to the operation.
- **FR-007**: Activation and invocation evidence MUST record connector ID and
  version, abstract operation, target, binding/configuration reference name,
  decision outcome, and correlation identifier. It MUST NOT record values,
  credentials, endpoints, paths, device identities, raw audio, or provider
  identifiers.
- **FR-008**: Each approved connector contract MUST include
  implementation-independent fixtures for success, missing/incompatible or
  unconfigured binding, policy denial, limits, cancellation, replay or
  idempotency where relevant, redaction, and browser/native behavior.

## Acceptance scenarios

1. A native host activates `traverse.audio-input` with a compatible private
   binding and processes a bounded capture request; a browser rejects that
   binding before invocation without exposing host details.
2. A declared model artifact runs through a compatible `traverse.model-runtime`
   binding under policy and resource limits; an untrusted or incompatible
   binding fails closed with a stable redacted error.
3. A WASM capability prepares an inference request and invokes the activated
   model-runtime connector without embedding provider or credential data.
4. A portable DSP algorithm runs as WASM; a proposed hardware acceleration
   binding supplies only media-generic operation evidence.
5. An advisory workflow uses `traverse.http`; no review UI, taxonomy, or
   provider-specific advisory connector becomes a public contract.

## Compatibility and non-goals

This is additive to the existing connector architecture. It does not design
production drivers, choose codecs/models/vendors/databases/microphones/clouds,
add browser emulation of native authority, standardize advisory/review UI or
workflow, or publish a media acceleration connector without a separately
approved unavoidable-host proposal.

## Validation

Parser and activation validation must reject invalid envelopes and incompatible
bindings deterministically. Contract, activation, and fixture tests must cover
every requirement and prove that trace/evidence output is redacted.

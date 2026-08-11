# ADR-0039: Extend Connector Architecture With App Bindings and Mediated Invocation

- Status: Accepted
- Governing specs: `039-connector-plugin-architecture`, `103-application-connector-binding`, `104-mediated-connector-invocation`
- Related issues: #826, #1050, #1051, #1052

## Context

Spec 039 defines connector contracts, abstract capability requirements, semver
resolution, host-owned configuration, runtime wiring, and trace evidence. Its
implementation is incomplete, and it does not define portable application
bindings or the concrete guest-to-host call surface. WASM execution currently
correctly grants no ambient filesystem, network, environment, or device access.

## Decision

Keep Spec 039 as the sole connector architecture. Capabilities declare abstract
requirements; applications bind them by connector id, compatible version, and a
non-secret configuration reference; hosts own implementations, private config,
credentials, paths, devices, and network policy. Static bundle validation checks
binding shape and compatibility. Host activation checks an installed compatible
implementation and private configuration. A versioned, bounded `connector_invoke`
host ABI is the only WASM route to an activated binding. Every decision and
failure emits deterministic, non-secret evidence.

## Consequences

Two successor specs govern the new fields and ABI. Implementations fail closed
for unbound, incompatible, unavailable, invalid-config, or unauthorized calls.
No secret or host identifier enters a public bundle, WASM guest, output, or
public trace. Existing ambient-authority denial remains unchanged.

## Alternatives Considered

- A new independent connector model: rejected because it duplicates Spec 039.
- Host-only undeclared mapping: rejected because portable bundles cannot be
  validated or explained.
- Ambient WASI or sidecar access: rejected because it bypasses contract and
  host-policy enforcement.

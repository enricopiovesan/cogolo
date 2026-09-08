# Feature Specification: Registry Genericity, Naming, and Configuration Policy

**Status**: Approved (2026-09-08)
**Canonical governing ID**: `1256-registry-genericity-policy`
**Version**: 0.1.0
**Decision evidence**: Traverse #1256 decision record (2026-09-07); ADR-0064.

## Purpose

Define the admission policy for public Registry capabilities. The policy keeps
portable business authority publishable while preventing public records from
embedding application workflows, user interfaces, vendors, hosts, secrets, or
environment-specific bindings.

## Boundary and ownership

A Registry capability owns one stable domain operation, its typed input/output
contract, and portable configuration semantics. A runtime primitive or
connector contract owns an unavoidable host authority and its request/response
envelope. An application owns workflow composition, product policy, UI, and
the selection of a concrete connector. A host connector implementation owns
credentials, endpoints, device IDs, filesystem paths, and provider-specific
bindings. A public capability MUST NOT move host or application ownership into
its name, configuration, contract, or evidence.

Audio-window planning and inference-request preparation are portable
capabilities when their schemas express only domain inputs and outputs.
Model-policy values are portable configuration when they are typed and do not
select a provider or secret. Microphone capture and a provider binding are
host connector concerns; a review screen and a customer workflow are
application concerns.

## Requirements

- **FR-001**: A public Registry capability MUST describe a stable domain
  operation with typed inputs, outputs, side effects, dependencies, and
  execution constraints. Its behavior MUST be deterministic or explicitly
  mediated through a declared connector/runtime authority.
- **FR-002**: A public capability `id`, namespace, and name MUST describe the
  domain operation. They MUST NOT encode an application, workflow, UI,
  customer, location, vendor, model identifier, database, microphone, host
  target, credential, endpoint, filesystem path, or device identity.
- **FR-003**: Persisted configuration MUST have a versioned typed schema. The
  schema MUST distinguish portable defaults from host-required references;
  records MAY retain only reference names and redacted provenance, never
  configuration values, credentials, endpoints, paths, or device IDs.
- **FR-004**: Configuration-schema evolution MAY add optional fields, enum
  members, or ranges only when existing configurations retain their meaning.
  Removing or reinterpreting a required field, enum member, range, or default
  requires a major schema version and explicit migration/deprecation guidance.
  Published identity/version pairs remain immutable.
- **FR-005**: Publication MUST include at least two materially distinct,
  portable fixture/configuration scenarios. A genuinely new primitive MAY use
  one scenario only with a documented publication-review exception explaining
  why a second scenario is not yet meaningful.
- **FR-006**: Fixture evidence MUST identify the contract/configuration schema
  version, target or target limitation, expected outcome, and redacted
  provenance. It MUST contain no private value, credential, endpoint, local
  path, device identity, or customer-specific data.
- **FR-007**: Publication validation MUST reject objectively detectable
  leakage in governed identity fields and evidence fields, including malformed
  secret-like values and explicitly disallowed host/application identifiers.
  It MUST emit stable, secret-free failures. It MUST NOT attempt to infer
  genericity from arbitrary source code or prose.
- **FR-008**: Human publication review MUST assess semantic genericity,
  ownership boundary, fixture distinctness, and any exception rationale. A
  successful structural validator MUST NOT by itself approve publication.

## Acceptance scenarios

1. Audio-window planning publishes with typed window parameters and portable
   browser/native fixtures; it does not name a microphone or application.
2. Inference-request preparation publishes with typed request and model-policy
   configuration while a model-runtime connector owns provider activation.
3. A contract whose identity contains a vendor, UI screen, host target, or
   secret-like evidence value fails structural publication validation with a
   stable, redacted error.
4. A configuration schema adds an optional policy field without changing an
   existing meaning; the publication is compatible. Reinterpreting a required
   field requires a new major schema version and migration guidance.
5. A new primitive with one fixture is rejected unless its reviewed exception
   rationale is recorded; a normal capability with two distinct fixtures is
   reviewable without evidence from two shipping customers.

## Compatibility and non-goals

This specification is additive. It does not rename existing public records
solely for mentioning a legitimate domain, design concrete connector contracts
or a runtime ABI, introduce a semantic-code classifier, or expose host-owned
configuration values in Registry evidence. Existing records are audited and
migration issues are created only for confirmed violations.

## Validation

Implement deterministic schema/structure tests for the rejectable fields,
fixture evidence redaction, and compatible versus breaking configuration
evolution. Keep semantic admission decisions and exceptions in the human
publication checklist with inspectable decision evidence.

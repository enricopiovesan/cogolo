# Feature Specification: ECCA Event Products

**Feature Branch**: `534-ecca-event-products`  
**Created**: 2026-07-29  
**Status**: Approved
**Version**: 1.0.0  
**Input**: Traverse #894 and its recorded architecture decisions; ECCA.

## Purpose

Define the mandatory, portable event-product standard for Traverse. This is
not a visual `emits`/`consumes` enhancement. A meaningful asynchronous domain
fact is a first-class governed product whose contract, catalog record, runtime
behavior, and operational evidence agree.

The standard governs registry, runtime, host adapters, generated capabilities,
and reference applications. It keeps generic runtime lifecycle telemetry
separate from domain events.

## Scope

In scope:

- the canonical event envelope, descriptor, payload schema, and contract kinds
- ownership, lifecycle, classification, exposure, and compatibility policy
- publication, runtime validation, audit, quarantine, lineage, and telemetry
- catalog discovery and the required reference-app conformance journey
- existing-catalog classification and migration rules

Out of scope:

- selecting a broker vendor or transport topology
- exactly-once delivery guarantees
- central ownership of domain semantics
- treating a command, request, or runtime lifecycle record as a domain event
- releasing a complete historical migration in this specification alone

## Normative Contract Standard

### Contract kinds and source of truth

- **FR-001**: A governed domain event MUST represent an externally meaningful,
  past-tense domain fact. Commands, requests, and runtime lifecycle telemetry
  MUST use distinct contract kinds and MUST NOT satisfy this standard.
- **FR-002**: A governed event MUST use a CloudEvents-compatible envelope plus
  a strict Traverse event-product descriptor. The descriptor and JSON Schema
  payload are authoritative. Generated bindings and AsyncAPI are derived views.
- **FR-003**: JSON Schema is the sole canonical payload-schema language. The
  selected JSON Schema profile, its schema identifier, and its event-version
  linkage MUST be validated.
- **FR-004**: An event type MUST use the canonical past-tense fact grammar
  `domain.entity.occurred`, or an approved equivalent fact verb. It MUST have a
  concise purpose statement and semantic description.

### Required descriptor fields

- **FR-005**: The descriptor MUST contain: immutable event identity; semantic
  version; CloudEvents-compatible type, source, subject, and time mapping;
  JSON Schema reference; purpose; domain; owner; stable support route;
  lifecycle; exposure; field classifications; compatibility policy; producer;
  declared consumers; retention/deprecation information; and validation
  evidence/provenance.
- **FR-006**: Owner is the producing capability's owning domain/team. An
  individual developer identifier alone is insufficient. Traverse platform
  owns shared standards and tooling, not the domain event's semantics.
- **FR-007**: Every event MUST declare exactly one exposure class: `public`,
  `partner`, `internal`, or `restricted`. Each payload field MUST declare the
  applicable controlled data class: `none`, `personal`, `sensitive`, or
  `regulated`. Free-form class names are invalid.
- **FR-008**: Lifecycle MUST be one of `draft`, `approved`, `deprecated`, or
  `retired`. A deprecated version MUST declare a replacement and retirement
  date. Only an approved contract may be published for discovery or execution.

### Versioning and delivery

- **FR-009**: Published contract versions and their payload schemas are
  immutable. Backward-compatible additions require a new minor version;
  breaking schema or semantic changes require a new major version. A published
  version MUST never be overwritten.
- **FR-010**: Every event MUST declare at-least-once delivery, an ordering
  scope, an event/deduplication identity, correlation ID, and causation ID.
  Consumers MUST be idempotent. Exactly-once is not a Traverse guarantee.

## Validation and Enforcement

- **FR-011**: Authoring and registry publication MUST machine-validate the
  descriptor, semantic naming, JSON Schema, classifications, ownership,
  lifecycle, compatibility, and declared relationships. A non-compliant new
  contract MUST be rejected before registry publication.
- **FR-012**: Validators MUST produce deterministic machine-readable
  diagnostics with a stable code, path, severity, remediation, contract ID,
  version, and governing-spec reference.
- **FR-013**: Runtime producer and consumer boundaries MUST validate the
  envelope, contract version, payload, and policy. Migration mode MUST report
  violations but not reject documented legacy gaps. Enforcement mode MUST reject
  invalid traffic.
- **FR-014**: In enforcement mode, invalid traffic MUST create immutable audit
  evidence and a governed quarantine record containing only sanitized failure
  metadata. Rejected payload data MUST NOT be exposed by default.
- **FR-015**: The enforcement cutover is permitted only when every published
  governed contract validates; all declared producers/consumers emit required
  telemetry; and two consecutive release conformance runs have zero unresolved
  undeclared-relationship or invalid-event findings.

## Catalog and Runtime Evidence

- **FR-016**: The catalog MUST store declared producer/consumer relationships
  separately from observed runtime lineage and show drift between them.
- **FR-017**: Each capability page MUST expose governed **Publishes** and
  **Consumes** relationships. Each event MUST have a dedicated searchable
  contract page containing schema/version, purpose, owner/support, lifecycle,
  exposure/classification, compatibility, declarations, observed lineage, and
  deprecation/replacement information.
- **FR-018**: The catalog MUST provide navigable lineage by event, producer,
  consumer, domain, owner, lifecycle, classification, and payload-field
  metadata. It MUST expose AsyncAPI as a generated discovery/export view.
- **FR-019**: Every host runtime MUST emit OpenTelemetry-compatible traces and
  metrics for publication, delivery, validation result, consumer outcome,
  contract version, correlation/causation, latency, retries, and observed
  lineage. Logs alone do not satisfy this requirement.

## Acceptance Scenarios

1. Given a new event descriptor missing its domain owner, controlled field
   classification, or canonical fact type, when publication is attempted, then
   validation rejects it with deterministic diagnostics and no catalog record.
2. Given an approved event product and a compatible minor payload addition,
   when the successor is published, then the prior version remains immutable,
   the successor is discoverable, and lineage preserves both versions.
3. Given a producer and two independent consumer capabilities, when the
   producer publishes a valid domain fact, then each idempotent consumer
   receives it according to the declared ordering scope and catalog evidence
   records both declarations and observed lineage.
4. Given a non-compliant payload after enforcement cutover, when it reaches a
   boundary, then it is rejected, audit and sanitized quarantine evidence exist,
   and consumers receive no invalid payload.
5. Given a valid reference application, when its catalog is inspected, then it
   shows a producer, state-changing consumer, and audit/notification observer;
   their declared and observed relations are distinguishable and navigable.

## Quality Gates

- **QG-001**: No new event product may be published without the mandatory
  descriptor, JSON Schema, and passing machine validation.
- **QG-002**: No new capability or reference-app generation/publication may
  resume until this spec and ADR are approved, the validator/conformance
  fixtures are merged and passing, and the three-capability reference journey
  passes end to end.
- **QG-003**: Contract tests MUST cover structural/semantic rejection,
  immutable evolution, compatibility, classifications, exposure, lifecycle,
  declared/observed drift, runtime rejection, quarantine privacy, and host
  telemetry.
- **QG-004**: An implementation MUST not substitute generic lifecycle telemetry
  for a meaningful domain event, or fabricate a domain event where no externally
  meaningful asynchronous effect exists.

## Existing Catalog Migration

- **FR-020**: Before its next publication, every currently published capability
  MUST receive a validator-backed classification: either it has no externally
  meaningful asynchronous effect with documented evidence, or it MUST add
  governed event products. Silent grandfathering is prohibited.

## Governing Relationship

This approved specification extends `003-event-contracts`, `018-event-driven-composition`,
`036-event-subscription-replay`, `066-durable-identity-event-delivery`,
`070-runtime-event-sink-boundary`, and `207-event-broker`. It does not amend
their approved status; the #894 decision log is its recorded approval evidence.

## Implementation Tickets

- Traverse #896 — registry catalog and capability discovery
- Traverse #897 — runtime validation, lineage, and conformance
- Traverse #898 — three-capability reference-app proof
- Traverse #899 — published-capability inventory and migration

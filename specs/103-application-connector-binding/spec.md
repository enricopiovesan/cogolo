# Feature Specification: Application Connector Binding and Activation

**Status**: Approved
**Canonical governing ID**: `103-application-connector-binding`
**Version**: 1.0.0
**Extends**: `039-connector-plugin-architecture`, `044-application-bundle-manifest`
**Input**: #826, #1050, #1051; ADR-0039.

## Purpose

Make an application bundle prove that its abstract Spec 039 connector
requirements have portable, compatible bindings without exposing private host
configuration.

## Requirements

- **FR-001**: An application manifest MUST bind each required connector by
  connector id, semver range, and non-secret configuration reference.
- **FR-002**: Static bundle validation MUST reject missing, duplicate,
  malformed, or Spec-039-incompatible bindings without reading private config.
- **FR-003**: Host activation MUST resolve an installed compatible connector,
  validate its placement target and private config against its connector
  contract, and fail closed if any check fails.
- **FR-004**: Validation and activation MUST emit immutable non-secret evidence
  and stable errors for unbound, incompatible, unavailable, and unconfigured
  connectors.
- **FR-005**: Capabilities MUST NOT select concrete host instances or receive
  config, credentials, paths, device identifiers, or provider endpoints.
- **FR-006**: Binding evolution MUST preserve Spec 039 semver compatibility and
  support explicit replacement/deprecation evidence.

## Acceptance Scenarios

1. A valid static binding validates without host configuration.
2. An incompatible or missing binding fails static validation with a stable error.
3. Invalid private configuration fails host activation without leaking a value.
4. Multiple capabilities may share one compatible activated connector.

## Quality Gates

- Fixtures cover each acceptance scenario and post-activation host drift.
- Bundle, activation, and evidence validation are deterministic.
- The spec-alignment gate covers manifest, contracts, runtime activation, and
  ADR-0039 paths.

## Out of Scope

Connector invocation ABI, secrets schema, concrete connectors, scheduler
semantics, and local-model artifact governance.

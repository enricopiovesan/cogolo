# Feature Specification: Activation-Time Executable Artifact Resolution

**Status**: Approved
**Canonical governing ID**: `106-activation-artifact-resolution`
**Version**: 1.0.0
**Extends**: `041-workflow-composition-api`, `044-application-bundle-manifest`, `103-application-connector-binding`
**Input**: #1057, #1058, #1060, #1061; ADR-0040.

## Purpose

Make host activation prove that every capability contract required by an
application or workflow has a compatible, active, verified executable package
in that host environment, while retaining portable workflow registration.

## Requirements

- **FR-001**: Workflow registration MUST continue to validate its graph and
  capability contract references without requiring host package inventory.
- **FR-002**: Host activation MUST resolve every required contract to an active
  executable package whose version, digest, ABI, placement, lifecycle, and
  declared execution constraints are compatible.
- **FR-003**: An explicit application package id/version pin MUST be honored.
- **FR-004**: Without a pin, the resolver MUST select the highest compatible
  active package version and then lexicographically lowest package id as a
  stable tie-break.
- **FR-005**: Activation MUST validate applicable connector and model
  requirements without exposing secrets or host-private configuration.
- **FR-006**: Successful activation MUST persist immutable non-secret evidence:
  contract reference, selected package id/version/digest, resolver version,
  eligibility decisions, and configuration references.
- **FR-007**: Missing, incompatible, inactive, invalid, unsatisfied, or drifted
  artifacts MUST fail closed with stable structured errors and evidence.
- **FR-008**: Execution MUST consume the activation record and MUST NOT silently
  re-resolve an artifact. A changed digest, lifecycle, ABI, placement, or
  constraint MUST produce an activation-drift failure.
- **FR-009**: Advisory package composition metadata MUST NOT influence artifact
  eligibility, selection, or authorization.

## Acceptance Scenarios

1. A portable workflow registers where its contracts exist but no local package
   is installed; host activation then fails with `executable_artifact_unavailable`.
2. Activation selects one eligible package and records its digest; execution
   succeeds only using that record.
3. An exact pin selects the requested compatible package; an invalid pin fails
   without falling back to another package.
4. Multiple eligible packages resolve by highest compatible version and stable
   package-id tie-break.
5. Artifact or lifecycle drift after activation causes execution to fail closed
   with `activation_artifact_drift`.

## Quality Gates

- Unit, integration, activation, trace, and drift tests cover zero, one, and
  multiple candidates; pins; ties; constraints; connector/model requirements;
  and all stable failure classes.
- Activation evidence is deterministic, non-secret, and inspectable.
- Spec-alignment validation covers registry, application host, runtime, CLI,
  MCP discovery metadata, and ADR-0040 paths.

## Out of Scope

Automatic workflow creation, traversal-time package selection, remote loading,
secret storage, and connector invocation ABI implementation.

# Feature Specification: Standalone Executable Capability Packages

**Status**: Approved
**Canonical governing ID**: `105-standalone-capability-packages`
**Version**: 1.0.0
**Extends**: `017-ai-agent-packaging`, `100-capability-package-authoring`
**Input**: #1057, #1058, #1059; ADR-0040.

## Purpose

Allow one governed executable package to implement and directly execute one
capability contract without knowing an application workflow. This preserves
contract, digest, ABI, constraint, fixture, and sandbox governance.

## Requirements

- **FR-001**: A package manifest MUST declare one valid capability contract
  reference and executable artifact; it MUST NOT require a workflow reference.
- **FR-002**: `known_compositions` MAY be omitted or empty. When present, it
  records advisory known/tested workflow compositions only.
- **FR-003**: Schema v1 MUST accept `workflow_refs` as the deprecated alias for
  `known_compositions` for one supported minor release.
- **FR-004**: If both fields occur with different normalized values, validation
  MUST fail with `conflicting_composition_metadata`.
- **FR-005**: Schema v2 MUST reject `workflow_refs` and require the canonical
  field where composition metadata is supplied.
- **FR-006**: `capability-package inspect` and `capability-package execute`
  MUST accept a valid standalone package through the ordinary verified-artifact
  path.
- **FR-007**: Composition metadata MUST NOT grant authority, determine package
  execution eligibility, or become a registry `requires` dependency.
- **FR-008**: Existing validation for contracts, source/binary paths, digests,
  ABI, host access, network/filesystem policy, runtime fixtures, lifecycle, and
  execution evidence MUST remain unchanged.
- **FR-009**: Inspect output and documentation MUST distinguish a standalone
  package from advisory known compositions without implying a prerequisite.

## Acceptance Scenarios

1. A valid package with omitted or empty `known_compositions` inspects and
   executes successfully from its runtime request fixture.
2. A package using only legacy `workflow_refs` remains valid in schema v1 and
   emits deterministic migration guidance.
3. A package containing conflicting canonical and legacy values fails before
   artifact execution with `conflicting_composition_metadata`.
4. Existing non-empty workflow-reference packages remain valid without edits.
5. Invalid contract, digest, ABI, or sandbox declarations remain rejected even
   when the package is standalone.

## Quality Gates

- Unit and CLI tests cover omitted, empty, canonical, alias, equal dual-field,
  conflicting dual-field, and existing workflow-reference manifests.
- Regression tests prove direct execution uses the verified artifact router.
- Documentation specifies the v1 migration window and v2 removal behavior.

## Out of Scope

Workflow creation, dynamic traversal-time discovery, host connector bindings,
remote package loading, and changes to executable sandbox authority.

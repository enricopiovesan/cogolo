# Feature Specification: Privacy-Preserving Authoring Outcome Telemetry

**Status**: Draft — approval required before implementation
**Canonical governing ID**: `1183-privacy-preserving-authoring-telemetry`
**Extends**: `088-runtime-usage-telemetry` only for its default-off,
provider-neutral collection precedent; it does not extend runtime usage fields.
**Decision evidence**: Traverse #1169 decision records.

## Purpose

Define a voluntary, privacy-preserving aggregate signal for whether governed
capability authoring and review reach a publishable outcome. This specification
does not authorize collection of authoring content or individual performance.

## Scope

In scope: locally aggregated, default-off authoring outcome measurements;
explicit consent; a minimum remote-publication cohort; retention, deletion,
and access controls; normalized non-content review categories.

Out of scope: prompts, source, contracts, artifacts, repository paths,
personal identifiers, review text, contributor ranking, and any effect on
authoring, review, build, or publication.

## Requirements

- **FR-001**: Collection MUST be disabled by default and enablement MUST be
  explicit, persistent, and prompt-free outside that dedicated action.
- **FR-002**: A measurement record MAY contain only a reporting-window ID,
  voluntary non-identifying authoring-route category, normalized terminal
  outcome, revision-cycle count, elapsed-time bucket, and normalized finding
  category counts.
- **FR-003**: Records MUST NOT contain prompts, source, raw contracts,
  artifact payloads or digests, repository paths, personal identifiers,
  review text, device/network identifiers, or inferred author attributes.
- **FR-004**: Remote publication MUST occur only from a local aggregate that
  represents at least 20 distinct opted-in contributors in its reporting
  window. Under-threshold data MUST remain local and MUST emit no remote
  report.
- **FR-005**: Opt-out MUST stop future collection immediately and deletion
  MUST remove locally retained authoring telemetry before any later report.
- **FR-006**: Remote collector failure, disabled collection, malformed data,
  unavailable consent state, or an under-threshold cohort MUST fail closed and
  MUST NOT affect authoring, review, build, or publication outcomes.
- **FR-007**: Remote reports MUST be aggregate-only, access-controlled, and
  retained for no more than 90 days.
- **FR-008**: Remote reports MUST be accessible only to a named, least-privilege
  Traverse maintainer analytics role. Every access MUST be logged and the role
  membership MUST be reviewed at least quarterly.

## Acceptance Scenarios

1. A fresh installation performs no authoring telemetry collection or network
   activity.
2. An opted-in contributor produces only allowed local categories; a prohibited
   field rejects the record before aggregation.
3. Nineteen distinct opted-in contributors produce no remote report.
4. Twenty distinct opted-in contributors produce one aggregate-only report.
5. Opt-out and deletion remove local data and prevent later remote publication.
6. A collector outage leaves the authoring and publication path unchanged.

## Quality Gates

- Deterministic conformance coverage for every acceptance scenario.
- Schema validation rejects every prohibited field.
- Review verifies the collector endpoint before approval; it cannot be inferred
  by implementation.

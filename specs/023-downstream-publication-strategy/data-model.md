# Data Model: Downstream Publication Strategy for Packaged Traverse Runtime and MCP Artifacts

This document defines the minimal data model for the governed downstream publication strategy used by Traverse release stewardship.

## Publication Strategy

Represents the governed publication plan for packaged runtime and MCP artifacts.

### Fields

- `strategy_id`: Stable identifier for the publication strategy slice.
- `scope`: The release path covered by the strategy.
- `supported_artifact_forms`: The artifact forms explicitly allowed for downstream publication.
- `release_critical_artifact_forms`: The subset required for the first consumer path.
- `downstream_consumer_targets`: The downstream apps or integrations expected to consume the artifacts.
- `linked_docs`: The release-facing docs that define how the strategy is used.

## Published Artifact Form

Represents one concrete published artifact form in the governed strategy.

### Fields

- `artifact_form_id`: Stable id for the artifact form.
- `artifact_type`: Runtime artifact or MCP artifact.
- `publication_shape`: How the artifact appears in the release path.
- `release_critical`: Whether the form is required for the first consumer path.
- `consumer_expectation`: How a downstream consumer is expected to use the artifact.

## Release-Critical Artifact

Represents a published artifact form that must exist for the first app-consumable release path.

### Fields

- `artifact_form_id`
- `required_for_first_release`
- `validation_reference`
- `consumer_bundle_reference`

## Downstream Consumer Target

Represents one downstream app or integration that consumes the published artifacts.

### Fields

- `consumer_id`: Stable identifier for the consumer.
- `consumer_type`: Browser-hosted app, MCP client, or other governed target.
- `required_artifact_forms`: The forms required by this consumer.
- `optional_artifact_forms`: The forms that are helpful but not required.

## Validation Evidence

Represents the evidence a reviewer uses to confirm the strategy is concrete and aligned.

### Fields

- `doc_reference`: Path to the governing spec or related release docs.
- `check_reference`: Repository check that verifies the required links and references exist.
- `release_reference`: The release or publication pointer that anchors the strategy.

# Traverse v0.1 App-Consumable Release Artifact and Publication Bundle

This document defines what the first Traverse v0.1 app-consumable release actually publishes.

The release checklist explains when the release is allowed. This document explains what the release artifact is once that checklist is satisfied.

## Publication Shape

The first app-consumable Traverse release is published as:

- one Git tag representing the v0.1 release point
- one GitHub release entry for that tag
- one release notes document that describes the shipped consumer surface
- one release bundle that links the release notes to the required validation evidence and supported consumer artifacts

The release bundle is a publication record, not a new runtime behavior layer.

## Bundle Contents

The first release bundle MUST include:

- the release notes scope
- the release checklist reference
- the governed consumer contract reference
- the downstream integration-validation reference
- the MCP consumption-validation reference
- the first real `youaskm3` validation reference
- the app-consumable acceptance reference
- the canonical quickstart reference
- the release-traceability reference
- the supported runnable consumer artifact reference

## Release Notes Scope

Release notes for the first app-consumable release SHOULD describe:

- what Traverse already supports for app consumers
- which public surfaces are approved for the first release
- which validation paths were used to support the release decision
- which known follow-up items are intentionally deferred
- which tickets remain outside the v0.1 release boundary

Release notes SHOULD avoid promising broader production hardening than the approved specs and tickets support.

## Required Validation Evidence

The release publication bundle MUST reference evidence for:

- the first app-consumable quickstart
- the live browser adapter smoke path
- the app-consumable acceptance path
- the downstream consumer contract
- the downstream integration validation path
- the MCP consumption validation path
- the first real `youaskm3` integration validation path

## Supported Runnable Artifact

The first release MUST point to at least one runnable consumer artifact that proves the app-consumable path exists.

For v0.1, the supported runnable artifact is the checked-in first app-consumable browser flow and its associated validation paths.

## Post-Release Policy

Items that do not belong in the first release bundle include:

- broader packaging polish beyond the first supported consumer path
- future host-model experiments
- broader performance baselines beyond the first app-consumable release needs
- future consumer templates and extra downstream apps

## Validation

A reviewer can tell from this document:

1. what the first app-consumable release publishes
2. what evidence accompanies the publication
3. what is intentionally left out of the first release bundle
4. what runnable artifact proves the release is consumable by an app

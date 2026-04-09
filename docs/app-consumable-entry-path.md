# App-Consumable Documentation Entry Path

This is the canonical documentation path for humans and coding agents working on the first app-consumable Traverse flow.

## Start Here

1. Read the repository root [README.md](/Users/piovese/Documents/cogolo/README.md)
2. Open [quickstart.md](/Users/piovese/Documents/cogolo/quickstart.md)
3. Use the relevant deeper docs only after the quickstart path is clear:
   - [docs/app-consumable-acceptance.md](/Users/piovese/Documents/cogolo/docs/app-consumable-acceptance.md)
   - [docs/app-consumable-release-checklist.md](/Users/piovese/Documents/cogolo/docs/app-consumable-release-checklist.md)
   - [docs/app-consumable-consumer-bundle.md](/Users/piovese/Documents/cogolo/docs/app-consumable-consumer-bundle.md)
   - [docs/app-consumable-package-release-pointer.md](/Users/piovese/Documents/cogolo/docs/app-consumable-package-release-pointer.md)
   - [docs/app-consumable-requirements-traceability.md](/Users/piovese/Documents/cogolo/docs/app-consumable-requirements-traceability.md)
   - [docs/youaskm3-integration-validation.md](/Users/piovese/Documents/cogolo/docs/youaskm3-integration-validation.md)
   - [docs/youaskm3-published-artifact-validation.md](/Users/piovese/Documents/cogolo/docs/youaskm3-published-artifact-validation.md)
   - [docs/youaskm3-compatibility-conformance-suite.md](/Users/piovese/Documents/cogolo/docs/youaskm3-compatibility-conformance-suite.md)
   - [docs/youaskm3-real-shell-validation.md](/Users/piovese/Documents/cogolo/docs/youaskm3-real-shell-validation.md)
   - [apps/browser-consumer/README.md](/Users/piovese/Documents/cogolo/apps/browser-consumer/README.md)

## Canonical Rule

If a new human or agent asks where to begin, point them to the README first and then to the root quickstart.

## Why This Exists

- The README is the front door.
- The quickstart is the first executable consumer path.
- The versioned consumer bundle explains what a downstream app installs and which released surfaces it may rely on.
- The package release pointer explains how the governed app-consumable package release is identified downstream.
- The published-artifact validation explains how `youaskm3` consumes the released runtime and MCP artifacts.
- The conformance suite explains how the released Traverse and `youaskm3` surfaces are proven together.
- The real-shell validation explains how the browser-hosted `youaskm3` shell is checked against the released Traverse consumer artifacts.
- The deeper docs explain validation, release, and traceability after the first path is understood.
- Competing entrypoints should be treated as references, not as the first recommended path.

## Validation

- The README links to the root quickstart.
- The quickstart links to the deeper app-consumable docs when needed.
- The canonical path is easy to describe without repository archaeology.

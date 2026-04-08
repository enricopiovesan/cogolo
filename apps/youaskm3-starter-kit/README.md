# youaskm3 Traverse Starter Kit

This starter kit is a reference integration package for browser-hosted downstream apps that want to adopt Traverse without repo archaeology.

It is intentionally small:

- it points to the versioned Traverse consumer bundle
- it points to the browser-targeted consumer package
- it points to the MCP consumption and compatibility validation paths
- it leaves downstream UI and product behavior to the consuming app

## What It Is For

Use this starter kit as the first place a downstream app developer looks when wiring Traverse into a browser-hosted app such as `youaskm3`.

It is not a replacement for the app-consumable docs.

## Included References

- [docs/app-consumable-consumer-bundle.md](/Users/piovese/Documents/cogolo/docs/app-consumable-consumer-bundle.md)
- [docs/youaskm3-starter-kit.md](/Users/piovese/Documents/cogolo/docs/youaskm3-starter-kit.md)
- [docs/youaskm3-integration-validation.md](/Users/piovese/Documents/cogolo/docs/youaskm3-integration-validation.md)
- [docs/youaskm3-compatibility-conformance-suite.md](/Users/piovese/Documents/cogolo/docs/youaskm3-compatibility-conformance-suite.md)

## Supported Assumptions

- The consuming app is browser-hosted.
- The consuming app uses the published Traverse consumer bundle.
- The consuming app depends on the browser-targeted consumer package rather than Traverse internals.
- The consuming app validates the browser-hosted and MCP-facing paths using the documented repo-local commands.


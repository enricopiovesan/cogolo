# Local Authoring Telemetry Pilot

Date: 2026-08-28

This bounded, non-exporting pilot exercises Spec 123’s aggregate-only threshold
path with 20 synthetic opaque contributor tickets. It uses no network
destination, real contributor, source, prompt, repository path, review text,
or personal identifier.

## Evidence

- Nineteen distinct ticket hashes are rejected for export.
- Twenty distinct ticket hashes produce only the bounded aggregate payload.
- Ticket hashes and raw events are absent from that payload.
- An unpublished bucket is removed on opt-out and expires at 90 days.

## Limits

This is a deterministic local integration pilot, not a production deployment.
It has complete selection bias because tickets are synthetic; it provides no
claim about authoring success rates, contributor behavior, or external
collector reliability. A later opt-in production pilot requires a named host,
explicit participant consent, and the analytics-role access configuration.

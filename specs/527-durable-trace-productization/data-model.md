# Data Model

| Entity | Required fields | Rule |
| --- | --- | --- |
| DurableTraceRecord | ids, timestamp, safe execution facts, redaction version | Never holds raw request/output payloads. |
| WorkspaceAuthorization | workspace, operation, allow/deny, policy ref | Host-issued and required for every operation. |
| RetentionPolicy | max age, max count, policy version | Defaults to 30 days and 10,000 records. |
| TracePruneEvidence | trigger, count, retained range, policy, outcome | Safe and deterministic. |
| TraceExportManifest | workspace, range, record ids, schema, digest | Produced only by explicit authorized export. |

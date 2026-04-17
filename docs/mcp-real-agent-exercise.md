# Traverse MCP Real-Agent Exercise

This document defines the first end-to-end exercise for a real AI agent consuming the Traverse MCP surface.

The goal is to prove that a real agent can connect to the governed `traverse-mcp` stdio server, discover the published surfaces, read contracts, and invoke a governed workflow without relying on private Traverse internals.

## Exercise Flow

1. Start the Traverse MCP stdio server.
2. Connect a real AI agent through the documented MCP stdio path.
3. Discover capabilities, events, and workflows through the public MCP entry points.
4. Read the relevant capability and event contracts.
5. Invoke the approved `plan-expedition` workflow through the governed MCP surface.
6. Observe the returned trace or execution report.

## Public Entry Points

The real-agent exercise should use the same public surfaces documented in:

- [docs/mcp-stdio-server.md](mcp-stdio-server.md)
- [docs/mcp-consumption-validation.md](mcp-consumption-validation.md)
- [docs/youaskm3-integration-validation.md](youaskm3-integration-validation.md)

The real-agent exercise should validate the following entry points:

- `discover_capabilities`
- `discover_events`
- `discover_workflows`
- `execute_entrypoint`
- `render_execution_report`

## Validation

Run the documented smoke path with:

```bash
bash scripts/ci/mcp_real_agent_exercise_smoke.sh
```

That smoke path confirms the guide exists, references the governed MCP entry points, and stays linked to the documented downstream validation flow.

## What Good Looks Like

- The real agent can discover governed Traverse surfaces without private repo knowledge.
- The real agent can read contracts through the documented MCP path.
- The real agent can invoke the approved workflow and receive a structured result.
- The output remains deterministic enough for a reviewer to trace.

## Known Limits

This exercise does not claim:

- full multi-agent coordination
- broad agent planning beyond the governed workflow
- automatic resolution of ambiguous downstream requirements

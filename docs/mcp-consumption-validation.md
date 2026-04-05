# App-Facing MCP Consumption Validation

Traverse uses `youaskm3` as the first proving downstream app for the app-facing MCP substrate.

This validation path is governed by:

- `specs/019-downstream-consumer-contract/spec.md`
- `specs/020-downstream-integration-validation/spec.md`
- `specs/021-app-facing-operational-constraints/spec.md`

## Validation Path

Run the deterministic smoke validation:

```bash
bash scripts/ci/mcp_consumption_validation.sh
```

The smoke path verifies that the public `traverse-mcp` surface can:

- discover a governed capability through the public MCP-facing API
- resolve that capability without internal-only coupling
- execute one governed request through the public MCP substrate
- observe ordered lifecycle, trace, and terminal messages

## Expected Evidence

The validation should prove:

- `consumer_name: youaskm3`
- `validated_flow_id: youaskm3_mcp_validation`
- `public_surface_id: traverse.mcp.downstream-consumer`
- at least one exposed tool or governed public surface is available
- the observed runtime outcome is completed
- the path uses only public Traverse surfaces

## Known Failure Modes

The validation is expected to fail deterministically when:

- the public capability cannot be discovered
- the capability cannot be resolved through the public surface
- runtime execution fails
- the observation stream does not include ordered lifecycle, trace, and terminal evidence

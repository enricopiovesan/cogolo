#![no_std]
#![no_main]

// Real, input-driven implementation of
// expedition.planning.validate-team-readiness (see
// contracts/examples/expedition/capabilities/validate-team-readiness/contract.json).
// Mirrors traverse-cli's own execute_validate_team_readiness exactly:
// readiness turns on team_profile.equipment_ready alone (conditions_summary
// is accepted per the contract but not itself a deciding factor, matching
// the CLI's own example logic), producing "ready" with no required actions,
// or "needs_action" with one.

include!("../../shared/expedition_json.rs");

fn process(input: &[u8], out: &mut Buf<'_>) {
    let objective = find_object(input, "objective").unwrap_or(b"{}");
    let objective_id = find_string(objective, "objective_id").unwrap_or(b"");
    let team_profile = find_object(input, "team_profile").unwrap_or(b"{}");
    let equipment_ready = find_bool(team_profile, "equipment_ready").unwrap_or(false);

    let status: &[u8] = if equipment_ready { b"ready" } else { b"needs_action" };

    let write_body = |out: &mut Buf<'_>| {
        out.push_str("\"readiness_result_id\":\"readiness-");
        out.push_bytes(objective_id);
        out.push_str("\",\"objective_id\":");
        out.push_json_string(objective_id);
        out.push_str(",\"status\":");
        out.push_json_string(status);
        out.push_str(",\"reasons\":[\"team profile satisfies baseline expedition requirements\"],\"required_actions\":[");
        if !equipment_ready {
            out.push_str("\"complete equipment verification\"");
        }
        out.push_str("]");
    };

    out.push_str("{");
    write_body(out);
    out.push_str(",\"readiness_result\":{");
    write_body(out);
    out.push_str("}}");
}

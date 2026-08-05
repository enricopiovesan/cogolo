#![no_std]
#![no_main]

// Real, input-driven implementation of
// expedition.planning.assemble-expedition-plan (see
// contracts/examples/expedition/capabilities/assemble-expedition-plan/contract.json).
// Mirrors traverse-cli's own execute_assemble_expedition_plan exactly:
// recommended_route_style is the first interpreted-intent route preference
// (falling back to a conservative default), constraints passes the
// interpreted intent's constraints through unchanged, readiness_notes is
// readiness_result.reasons followed by readiness_result.required_actions,
// and key_steps/summary are the same fixed planning-domain text the CLI's
// example implementation uses (this capability composes prior steps'
// outputs; it does not itself invent new route guidance).

include!("../../shared/expedition_json.rs");

fn process(input: &[u8], out: &mut Buf<'_>) {
    let objective = find_object(input, "objective").unwrap_or(b"{}");
    let objective_id = find_string(objective, "objective_id").unwrap_or(b"");
    let interpreted_intent = find_object(input, "interpreted_intent").unwrap_or(b"{}");
    let route_preferences = find_array(interpreted_intent, "route_preferences").unwrap_or(b"[]");
    let route_style =
        first_string_in_array(route_preferences).unwrap_or(b"conservative-alpine-push");
    let constraints = find_array(interpreted_intent, "constraints").unwrap_or(b"[]");
    let readiness_result = find_object(input, "readiness_result").unwrap_or(b"{}");
    let readiness_status = find_string(readiness_result, "status").unwrap_or(b"");
    let readiness_reasons = find_array(readiness_result, "reasons").unwrap_or(b"[]");
    let required_actions = find_array(readiness_result, "required_actions").unwrap_or(b"[]");

    let status: &[u8] = if readiness_status == b"ready" {
        b"ready"
    } else {
        b"requires_attention"
    };

    out.push_str("{\"plan_id\":\"plan-");
    out.push_bytes(objective_id);
    out.push_str("\",\"objective_id\":");
    out.push_json_string(objective_id);
    out.push_str(",\"status\":");
    out.push_json_string(status);
    out.push_str(",\"recommended_route_style\":");
    out.push_json_string(route_style);
    out.push_str(
        ",\"key_steps\":[\"depart before sunrise\",\"reassess winds at mid-route checkpoint\",\"apply conservative turnaround time\"],\"constraints\":",
    );
    out.push_bytes(constraints);
    out.push_str(",\"readiness_notes\":");
    push_joined_string_arrays(out, &[readiness_reasons, required_actions]);
    out.push_str(
        ",\"summary\":\"Proceed with a conservative same-day ascent plan under a limited morning weather window.\"}",
    );
}

#![no_std]
#![no_main]

// Real, input-driven implementation of
// expedition.planning.interpret-expedition-intent (see
// contracts/examples/expedition/capabilities/interpret-expedition-intent/contract.json).
// Mirrors traverse-cli's own execute_interpret_expedition_intent exactly:
// route_preferences is [style, priority] from the objective's preferences,
// constraints is a single "priority:<priority>" entry, assumptions echoes the
// free-form planning_intent, and confidence is a fixed 0.87 (this capability
// is documented as AI-assisted interpretation feeding deterministic
// downstream steps, not itself a confidence model).

include!("../../shared/expedition_json.rs");

fn process(input: &[u8], out: &mut Buf<'_>) {
    let objective = find_object(input, "objective").unwrap_or(b"{}");
    let objective_id = find_string(objective, "objective_id").unwrap_or(b"");
    let preferences = find_object(objective, "preferences").unwrap_or(b"{}");
    let style = find_string(preferences, "style").unwrap_or(b"");
    let priority = find_string(preferences, "priority").unwrap_or(b"");
    let planning_intent = find_string(input, "planning_intent").unwrap_or(b"");

    let write_body = |out: &mut Buf<'_>| {
        out.push_str("\"intent_id\":\"intent-");
        out.push_bytes(objective_id);
        out.push_str("\",\"objective_id\":");
        out.push_json_string(objective_id);
        out.push_str(",\"route_preferences\":[");
        out.push_json_string(style);
        out.push_str(",");
        out.push_json_string(priority);
        out.push_str("],\"constraints\":[\"priority:");
        out.push_bytes(priority);
        out.push_str("\"],\"assumptions\":[");
        out.push_json_string(planning_intent);
        out.push_str("],\"confidence\":0.87");
    };

    out.push_str("{");
    write_body(out);
    out.push_str(",\"interpreted_intent\":{");
    write_body(out);
    out.push_str("}}");
}

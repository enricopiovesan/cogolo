#![no_std]
#![no_main]

// Real, input-driven implementation of
// expedition.planning.assess-conditions-summary (see
// contracts/examples/expedition/capabilities/assess-conditions-summary/contract.json).
// Mirrors traverse-cli's own execute_assess_conditions_summary exactly: a
// fixed "watchful" overall_rating with two derived key_findings sentences and
// no blocking_concerns (this capability is documented as deterministic, not
// a real weather/hazard model).

include!("../../shared/expedition_json.rs");

fn process(input: &[u8], out: &mut Buf<'_>) {
    let objective = find_object(input, "objective").unwrap_or(b"{}");
    let objective_id = find_string(objective, "objective_id").unwrap_or(b"");
    let destination = find_string(objective, "destination").unwrap_or(b"");
    let interpreted_intent = find_object(input, "interpreted_intent").unwrap_or(b"{}");
    let route_preferences = find_array(interpreted_intent, "route_preferences").unwrap_or(b"[]");
    let preferred_style = first_string_in_array(route_preferences).unwrap_or(b"conservative");

    let write_body = |out: &mut Buf<'_>| {
        out.push_str("\"conditions_summary_id\":\"conditions-");
        out.push_bytes(objective_id);
        out.push_str("\",\"objective_id\":");
        out.push_json_string(objective_id);
        out.push_str(",\"overall_rating\":\"watchful\",\"key_findings\":[\"stable morning window for ");
        out.push_bytes(destination);
        out.push_str("\",\"preferred style: ");
        out.push_bytes(preferred_style);
        out.push_str("\"],\"blocking_concerns\":[]");
    };

    out.push_str("{");
    write_body(out);
    out.push_str(",\"conditions_summary\":{");
    write_body(out);
    out.push_str("}}");
}

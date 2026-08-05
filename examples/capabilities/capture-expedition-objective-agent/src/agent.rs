#![no_std]
#![no_main]

// Real, input-driven implementation of expedition.planning.capture-expedition-objective
// (see contracts/examples/expedition/capabilities/capture-expedition-objective/contract.json).
// Normalizes destination/target_window/preferences/notes into a governed
// objective record, deriving objective_id from the destination the same way
// traverse-cli's own execute_capture_expedition_objective does (a lowercase
// ASCII-alphanumeric slug), so this WASM binary and the CLI's example
// fallback produce byte-identical objective_id values for the same input.

include!("../../shared/expedition_json.rs");

fn process(input: &[u8], out: &mut Buf<'_>) {
    let destination = find_string(input, "destination").unwrap_or(b"");
    let target_window = find_object(input, "target_window").unwrap_or(b"{}");
    let preferences = find_object(input, "preferences").unwrap_or(b"{}");
    let notes = find_string(input, "notes").unwrap_or(b"");

    out.push_str("{\"objective_id\":\"objective-");
    write_slug(out, destination);
    out.push_str("\",\"destination\":");
    out.push_json_string(destination);
    out.push_str(",\"target_window\":");
    out.push_bytes(target_window);
    out.push_str(",\"preferences\":");
    out.push_bytes(preferences);
    out.push_str(",\"notes\":");
    out.push_json_string(notes);
    out.push_str(",\"objective\":{\"objective_id\":\"objective-");
    write_slug(out, destination);
    out.push_str("\",\"destination\":");
    out.push_json_string(destination);
    out.push_str(",\"target_window\":");
    out.push_bytes(target_window);
    out.push_str(",\"preferences\":");
    out.push_bytes(preferences);
    out.push_str(",\"notes\":");
    out.push_json_string(notes);
    out.push_str("}}");
}

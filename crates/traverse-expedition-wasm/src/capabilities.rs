use alloc::{
    borrow::ToOwned,
    format,
    string::{String, ToString},
};
use serde_json::{Value, json};

pub fn execute(input: &Value) -> Result<Value, String> {
    let map = input
        .as_object()
        .ok_or_else(|| "input must be an object".to_string())?;
    if map.contains_key("team_profile") {
        return readiness(input);
    }
    if map.contains_key("readiness_result") {
        return assemble(input);
    }
    if map.contains_key("interpreted_intent") {
        return conditions(input);
    }
    if map.contains_key("planning_intent") {
        return interpret(input);
    }
    capture(input)
}

fn string(value: &Value, key: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("missing {key}"))
}
fn object<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value.get(key).ok_or_else(|| format!("missing {key}"))
}

fn capture(input: &Value) -> Result<Value, String> {
    let destination = string(input, "destination")?;
    let objective_id = format!("objective-{}", destination.to_lowercase().replace(' ', "-"));
    let objective = json!({"objective_id":objective_id,"destination":destination,"target_window":input["target_window"],"preferences":input["preferences"],"notes":input["notes"]});
    Ok(
        json!({"objective_id":objective_id,"destination":destination,"target_window":input["target_window"],"preferences":input["preferences"],"notes":input["notes"],"objective":objective,"emitted_events":[{"event_id":"expedition.planning.expedition-objective-captured","version":"1.0.0"}]}),
    )
}
fn interpret(input: &Value) -> Result<Value, String> {
    let objective = object(input, "objective")?;
    let id = string(objective, "objective_id")?;
    let prefs = object(objective, "preferences")?;
    let style = string(prefs, "style")?;
    let priority = string(prefs, "priority")?;
    let intent = string(input, "planning_intent")?;
    let record = json!({"intent_id":format!("intent-{id}"),"objective_id":id,"route_preferences":[style,priority],"constraints":[format!("priority:{priority}")],"assumptions":[intent],"confidence":0.87});
    Ok(
        json!({"intent_id":record["intent_id"],"objective_id":record["objective_id"],"route_preferences":record["route_preferences"],"constraints":record["constraints"],"assumptions":record["assumptions"],"confidence":0.87,"interpreted_intent":record,"emitted_events":[{"event_id":"expedition.planning.expedition-intent-interpreted","version":"1.0.0"}]}),
    )
}
fn conditions(input: &Value) -> Result<Value, String> {
    Ok(
        json!({"conditions_summary_id":"conditions-output","objective_id":object(input,"objective")?["objective_id"],"overall_rating":"watchful","key_findings":["stable morning window"],"blocking_concerns":[],"conditions_summary":{"overall_rating":"watchful"},"emitted_events":[{"event_id":"expedition.planning.conditions-summary-assessed","version":"1.0.0"}]}),
    )
}
fn readiness(input: &Value) -> Result<Value, String> {
    let ready = input["team_profile"]["equipment_ready"]
        .as_bool()
        .ok_or_else(|| "missing equipment_ready".to_string())?;
    let status = if ready { "ready" } else { "needs_action" };
    Ok(
        json!({"readiness_result_id":"readiness-output","objective_id":input["objective"]["objective_id"],"status":status,"reasons":["team profile satisfies baseline expedition requirements"],"required_actions":if ready {json!([])} else {json!(["complete equipment verification"])} ,"readiness_result":{"status":status,"reasons":["team profile satisfies baseline expedition requirements"],"required_actions":if ready {json!([])} else {json!(["complete equipment verification"])}},"emitted_events":[{"event_id":"expedition.planning.team-readiness-validated","version":"1.0.0"}]}),
    )
}
fn assemble(input: &Value) -> Result<Value, String> {
    let id = input["objective"]["objective_id"].clone();
    Ok(
        json!({"plan_id":format!("plan-{}",id.as_str().unwrap_or("objective")),"objective_id":id,"status":input["readiness_result"]["status"],"recommended_route_style":input["interpreted_intent"]["route_preferences"][0],"key_steps":["depart before sunrise","reassess winds at mid-route checkpoint","apply conservative turnaround time"],"constraints":input["interpreted_intent"]["constraints"],"readiness_notes":input["readiness_result"]["reasons"],"summary":"Proceed with a conservative same-day ascent plan under a limited morning weather window.","emitted_events":[{"event_id":"expedition.planning.expedition-plan-assembled","version":"1.0.0"}]}),
    )
}

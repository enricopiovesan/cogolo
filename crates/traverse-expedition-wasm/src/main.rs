//! Expedition example domain — WASM binary entry point.
//!
//! Governed by spec 027-expedition-wasm-port
//!
//! Reads a JSON expedition planning request from stdin, runs simplified
//! deterministic expedition planning logic, and writes a JSON plan response to
//! stdout.  Compiles to `wasm32-wasi` with no OS threads, no tokio, and no
//! ambient host authority.
//!
//! # I/O contract
//!
//! **Input** (stdin, JSON):
//! ```json
//! {
//!   "destination": "...",
//!   "team_size": 4,
//!   "objective": "..."
//! }
//! ```
//!
//! **Output** (stdout, JSON):
//! ```json
//! {
//!   "plan_id": "...",
//!   "objective_id": "...",
//!   "status": "ready",
//!   "recommended_route_style": "...",
//!   "key_steps": [...],
//!   "constraints": [...],
//!   "readiness_notes": [...],
//!   "summary": "..."
//! }
//! ```

use std::io::{self, Read, Write};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Input / Output types
// ---------------------------------------------------------------------------

/// Simplified expedition planning request.
#[derive(Debug, Deserialize)]
struct ExpeditionRequest {
    destination: String,
    #[serde(default)]
    team_size: u32,
    #[serde(default)]
    objective: String,
}

/// Expedition planning response (subset of the full contract schema).
#[derive(Debug, Serialize)]
struct ExpeditionPlan {
    plan_id: String,
    objective_id: String,
    status: String,
    recommended_route_style: String,
    key_steps: Vec<String>,
    constraints: Vec<String>,
    readiness_notes: Vec<String>,
    summary: String,
}

// ---------------------------------------------------------------------------
// Core planning logic (pure, deterministic)
// ---------------------------------------------------------------------------

/// Derive a recommended route style from destination text.
fn route_style(destination: &str) -> &'static str {
    let lower = destination.to_lowercase();
    if lower.contains("mountain") || lower.contains("peak") || lower.contains("alpine") {
        "alpine-traverse"
    } else if lower.contains("river") || lower.contains("canyon") || lower.contains("gorge") {
        "river-descent"
    } else if lower.contains("desert") || lower.contains("dune") {
        "desert-crossing"
    } else {
        "standard-trek"
    }
}

/// Build key planning steps from request fields.
fn build_key_steps(req: &ExpeditionRequest) -> Vec<String> {
    let mut steps = vec![
        format!("Define objective: {}", req.objective),
        format!("Assess destination: {}", req.destination),
        format!("Assemble team of {} members", req.team_size),
        "Evaluate environmental conditions".to_string(),
        "Validate team readiness and equipment".to_string(),
        "Assemble and approve final expedition plan".to_string(),
    ];
    if req.team_size > 8 {
        steps.push("Split into sub-teams for large group logistics".to_string());
    }
    steps
}

/// Derive constraints from request context.
fn build_constraints(req: &ExpeditionRequest) -> Vec<String> {
    let mut constraints = vec![
        "No host API access".to_string(),
        "No network access".to_string(),
        "No filesystem access".to_string(),
    ];
    if req.team_size == 0 {
        constraints.push("Team size must be at least 1".to_string());
    }
    constraints
}

/// Build readiness notes.
fn build_readiness_notes(req: &ExpeditionRequest) -> Vec<String> {
    let mut notes = vec!["Equipment checklist reviewed".to_string()];
    if req.team_size >= 4 {
        notes.push("Team size meets minimum threshold".to_string());
    } else {
        notes.push("WARNING: team size below recommended minimum of 4".to_string());
    }
    notes.push(format!(
        "Destination briefing completed for {}",
        req.destination
    ));
    notes
}

/// Plan an expedition given a request. Pure function — no I/O.
fn plan_expedition(req: &ExpeditionRequest) -> ExpeditionPlan {
    let plan_id = format!("plan-{}-t{}", slugify(&req.destination), req.team_size);
    let objective_id = format!("obj-{}", slugify(&req.objective));
    let style = route_style(&req.destination);

    ExpeditionPlan {
        plan_id,
        objective_id,
        status: "ready".to_string(),
        recommended_route_style: style.to_string(),
        key_steps: build_key_steps(req),
        constraints: build_constraints(req),
        readiness_notes: build_readiness_notes(req),
        summary: format!(
            "Expedition to {} for {} with {} team members via {} route.",
            req.destination, req.objective, req.team_size, style
        ),
    }
}

/// Convert a string to a simple ASCII slug for stable IDs.
fn slugify(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

// ---------------------------------------------------------------------------
// WASI entry point
// ---------------------------------------------------------------------------

fn run() -> Result<(), String> {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| format!("failed to read stdin: {e}"))?;

    let request: ExpeditionRequest =
        serde_json::from_str(&input).map_err(|e| format!("invalid JSON input: {e}"))?;

    let plan = plan_expedition(&request);

    let output =
        serde_json::to_string(&plan).map_err(|e| format!("failed to serialize output: {e}"))?;

    io::stdout()
        .write_all(output.as_bytes())
        .map_err(|e| format!("failed to write stdout: {e}"))?;

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        // Write error to stderr; exit with non-zero status.
        let _ = writeln!(io::stderr(), "traverse-expedition-wasm error: {e}");
        std::process::exit(1);
    }
}

// ---------------------------------------------------------------------------
// Unit tests (run on native — not wasm32-wasi)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn base_request() -> ExpeditionRequest {
        ExpeditionRequest {
            destination: "Alpine Peak".to_string(),
            team_size: 4,
            objective: "Summit attempt".to_string(),
        }
    }

    #[test]
    fn plan_expedition_returns_ready_status() {
        let plan = plan_expedition(&base_request());
        assert_eq!(plan.status, "ready");
    }

    #[test]
    fn plan_expedition_plan_id_is_stable() {
        let plan = plan_expedition(&base_request());
        assert_eq!(plan.plan_id, "plan-alpine-peak-t4");
    }

    #[test]
    fn plan_expedition_objective_id_is_stable() {
        let plan = plan_expedition(&base_request());
        assert_eq!(plan.objective_id, "obj-summit-attempt");
    }

    #[test]
    fn route_style_alpine() {
        assert_eq!(route_style("Alpine Peak"), "alpine-traverse");
        assert_eq!(route_style("Everest Peak"), "alpine-traverse");
    }

    #[test]
    fn route_style_river() {
        assert_eq!(route_style("Grand Canyon"), "river-descent");
        assert_eq!(route_style("Colorado River"), "river-descent");
    }

    #[test]
    fn route_style_desert() {
        assert_eq!(route_style("Sahara Desert"), "desert-crossing");
    }

    #[test]
    fn route_style_default() {
        assert_eq!(route_style("Unknown Land"), "standard-trek");
    }

    #[test]
    fn build_key_steps_includes_objective() {
        let req = base_request();
        let steps = build_key_steps(&req);
        assert!(steps.iter().any(|s| s.contains("Summit attempt")));
    }

    #[test]
    fn build_key_steps_adds_split_for_large_team() {
        let req = ExpeditionRequest {
            destination: "Peak".to_string(),
            team_size: 10,
            objective: "Test".to_string(),
        };
        let steps = build_key_steps(&req);
        assert!(steps.iter().any(|s| s.contains("sub-teams")));
    }

    #[test]
    fn build_key_steps_no_split_for_small_team() {
        let req = base_request(); // team_size = 4
        let steps = build_key_steps(&req);
        assert!(!steps.iter().any(|s| s.contains("sub-teams")));
    }

    #[test]
    fn build_constraints_includes_no_network() {
        let req = base_request();
        let constraints = build_constraints(&req);
        assert!(constraints.iter().any(|s| s.contains("network")));
    }

    #[test]
    fn build_constraints_warns_on_zero_team() {
        let req = ExpeditionRequest {
            destination: "Peak".to_string(),
            team_size: 0,
            objective: "Test".to_string(),
        };
        let constraints = build_constraints(&req);
        assert!(constraints.iter().any(|s| s.contains("at least 1")));
    }

    #[test]
    fn build_readiness_notes_warns_small_team() {
        let req = ExpeditionRequest {
            destination: "Peak".to_string(),
            team_size: 2,
            objective: "Test".to_string(),
        };
        let notes = build_readiness_notes(&req);
        assert!(notes.iter().any(|s| s.contains("WARNING")));
    }

    #[test]
    fn build_readiness_notes_ok_for_adequate_team() {
        let req = base_request(); // team_size = 4
        let notes = build_readiness_notes(&req);
        assert!(notes.iter().any(|s| s.contains("meets minimum")));
    }

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Alpine Peak"), "alpine-peak");
        assert_eq!(slugify("Mount Everest!"), "mount-everest");
        assert_eq!(slugify("---"), "");
    }

    #[test]
    fn slugify_already_clean() {
        assert_eq!(slugify("summit"), "summit");
    }

    #[test]
    fn plan_expedition_summary_contains_destination() {
        let plan = plan_expedition(&base_request());
        assert!(plan.summary.contains("Alpine Peak"));
    }

    #[test]
    fn plan_expedition_key_steps_not_empty() {
        let plan = plan_expedition(&base_request());
        assert!(!plan.key_steps.is_empty());
    }

    #[test]
    fn plan_expedition_constraints_not_empty() {
        let plan = plan_expedition(&base_request());
        assert!(!plan.constraints.is_empty());
    }

    #[test]
    fn plan_expedition_readiness_notes_not_empty() {
        let plan = plan_expedition(&base_request());
        assert!(!plan.readiness_notes.is_empty());
    }
}

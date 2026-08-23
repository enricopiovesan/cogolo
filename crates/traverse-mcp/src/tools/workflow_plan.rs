//! MCP tool surface for the declarative workflow planner (P0).
//!
//! Governed by spec `113-declarative-workflow-planning`. Mirrors the
//! `tools::proposals` pattern: a plain, fully-tested Rust function forms the
//! public MCP surface, independent of `stdio_server.rs`. Given a structured
//! target and a starting fact set, derives candidate `WorkflowProposal`s
//! already shaped for submission to the existing P1 surface
//! (`tools::proposals::submit_proposal`) once a reviewer clears their
//! `mapping_unconfirmed` flag (FR-005/FR-006). This module never submits or
//! executes anything itself, and never mutates the registry or manifest
//! (FR-008) -- every function here only reads through shared references.

use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;

use traverse_contracts::{
    CapabilityContract, ManifestReference, MappingSource, ProposalEdge, ProposalMapping,
    ProposalNode, WorkflowProposal,
};
use traverse_registry::{ApplicationBundleManifest, CapabilityRegistry, LookupScope};

/// At most 5 complete candidate plans per call (spec 113 FR-004).
const PLAN_MAX_CANDIDATES: usize = 5;
/// At most 8 nodes deep per candidate plan (spec 113 FR-004).
const PLAN_MAX_NODES: usize = 8;
/// Defensive cap on total recursive search calls, independent of the node/
/// candidate bounds above, so a pathological declared-capability graph can't
/// make a single planning call do unbounded work (spec 113 FR-004: "never
/// ... search unbounded").
const PLAN_MAX_SEARCH_CALLS: usize = 4_000;

/// What the planner must produce a chain ending in (spec 113 FR-001): a
/// declared event type or an exact capability id/version the caller needs
/// produced. Never a natural-language goal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanTarget {
    Capability {
        capability_id: String,
        capability_version: String,
    },
    EmitsEvent {
        event_type: String,
    },
}

/// Everything [`plan_workflow`] needs to derive candidate plans. Planning is
/// scoped to the capabilities declared as components in `manifest`, matching
/// the same "declared set is the only permitted set" check the P1 surface
/// applies at submission time (spec 109's `UndeclaredCapability` denial) --
/// this keeps a returned candidate structurally submittable as-is (FR-006).
pub struct PlanRequest<'a> {
    pub target: &'a PlanTarget,
    /// The facts already available before any node runs. Bound to
    /// `MappingSource::InitialInput` mappings in every returned candidate.
    pub starting_facts: &'a Value,
    pub manifest: &'a ApplicationBundleManifest,
    /// The exact, already-registered application manifest reference every
    /// returned candidate is bound to (spec 109 FR-002, ADR-0041). The
    /// planner does not compute this itself -- the caller already holds it
    /// from resolving `manifest` against the application registry.
    pub app_manifest: &'a ManifestReference,
    pub registry: &'a CapabilityRegistry,
    pub workspace_id: &'a str,
}

/// One candidate plan, structurally submittable as-is to `submit_proposal`
/// once a reviewer clears its field mappings (spec 113 FR-005, FR-006).
#[derive(Debug, Clone, Serialize)]
pub struct PlanCandidate {
    pub proposal: WorkflowProposal,
    /// Always `true`: the planner never clears this itself (FR-005).
    pub mapping_unconfirmed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlanResponse {
    pub candidates: Vec<PlanCandidate>,
    /// `true` when the search found more than [`PLAN_MAX_CANDIDATES`] valid
    /// complete plans, or hit the node-depth or search-work bound before it
    /// could fully explore a branch that had a structurally valid next step
    /// (spec 113 FR-004). Never a silent partial result.
    pub plan_search_truncated: bool,
}

/// A declared component's capability, resolved to its full contract and
/// exact artifact digest -- the only capabilities the planner may place in a
/// candidate node (spec 113's "published capabilities" scoped to what's
/// actually declared in the target application manifest).
struct DeclaredCapability {
    capability_id: String,
    capability_version: String,
    contract: CapabilityContract,
    artifact_digest: String,
}

/// Derives candidate workflow proposals for `request.target`, pure and
/// read-only over the given manifest/registry snapshot (spec 113 FR-008).
/// Never fails: an empty `candidates` list is itself a valid, correctly
/// reported outcome (acceptance scenario 3) when no declared capability's
/// schema chains to the target.
#[must_use]
pub fn plan_workflow(request: &PlanRequest<'_>) -> PlanResponse {
    let mut declared: Vec<DeclaredCapability> = request
        .manifest
        .components
        .iter()
        .filter_map(|component| {
            let resolved = request.registry.find_exact(
                LookupScope::PreferPrivate,
                &component.manifest.capability_id,
                &component.manifest.capability_version,
            )?;
            let artifact_digest = resolved
                .artifact
                .digests
                .binary_digest
                .clone()
                .unwrap_or_else(|| resolved.artifact.digests.source_digest.clone());
            Some(DeclaredCapability {
                capability_id: component.manifest.capability_id.clone(),
                capability_version: component.manifest.capability_version.clone(),
                contract: resolved.contract,
                artifact_digest,
            })
        })
        .collect();
    declared.sort_by(|a, b| {
        (&a.capability_id, &a.capability_version).cmp(&(&b.capability_id, &b.capability_version))
    });
    declared.dedup_by(|a, b| {
        a.capability_id == b.capability_id && a.capability_version == b.capability_version
    });

    let starting_outputs = starting_facts_output_schema(request.starting_facts);
    let target_indices: Vec<usize> = declared
        .iter()
        .enumerate()
        .filter(|(_, capability)| target_matches(request.target, capability))
        .map(|(index, _)| index)
        .collect();

    let mut truncated = false;
    let mut search_calls_remaining = PLAN_MAX_SEARCH_CALLS;
    let mut all_chains: Vec<Vec<usize>> = Vec::new();
    for &target_index in &target_indices {
        let chains = build_chains(
            &declared,
            &starting_outputs,
            target_index,
            PLAN_MAX_NODES,
            &BTreeSet::new(),
            &mut search_calls_remaining,
            &mut truncated,
        );
        all_chains.extend(chains);
    }

    all_chains.sort_by(|a, b| {
        a.len().cmp(&b.len()).then_with(|| {
            let a_ids: Vec<&str> = a
                .iter()
                .map(|&i| declared[i].capability_id.as_str())
                .collect();
            let b_ids: Vec<&str> = b
                .iter()
                .map(|&i| declared[i].capability_id.as_str())
                .collect();
            a_ids.cmp(&b_ids)
        })
    });
    all_chains.dedup();

    if all_chains.len() > PLAN_MAX_CANDIDATES {
        truncated = true;
    }

    let candidates = all_chains
        .into_iter()
        .take(PLAN_MAX_CANDIDATES)
        .enumerate()
        .map(|(candidate_index, chain)| {
            build_candidate(request, &declared, &chain, candidate_index)
        })
        .collect();

    PlanResponse {
        candidates,
        plan_search_truncated: truncated,
    }
}

fn target_matches(target: &PlanTarget, capability: &DeclaredCapability) -> bool {
    match target {
        PlanTarget::Capability {
            capability_id,
            capability_version,
        } => {
            capability.capability_id == *capability_id
                && capability.capability_version == *capability_version
        }
        PlanTarget::EmitsEvent { event_type } => capability
            .contract
            .emits
            .iter()
            .any(|reference| reference.event_id == *event_type),
    }
}

/// Bounded recursive backward search: enumerates every way to reach
/// `node_index`, either directly from starting facts or through exactly one
/// upstream declared capability whose `outputs.schema` structurally
/// satisfies `node_index`'s `inputs.schema.required` (spec 113 FR-002),
/// recursing on that upstream capability in turn. Returns root-first node
/// index chains ending at `node_index`.
#[allow(clippy::too_many_arguments)]
fn build_chains(
    declared: &[DeclaredCapability],
    starting_outputs: &Value,
    node_index: usize,
    remaining_budget: usize,
    excluded: &BTreeSet<usize>,
    search_calls_remaining: &mut usize,
    truncated: &mut bool,
) -> Vec<Vec<usize>> {
    if *search_calls_remaining == 0 {
        *truncated = true;
        return Vec::new();
    }
    *search_calls_remaining -= 1;

    let node = &declared[node_index];
    let required = schema_required(&node.contract.inputs.schema);
    let node_schema = &node.contract.inputs.schema;

    let mut result = Vec::new();
    if schema_covers_required(starting_outputs, &required, node_schema) {
        result.push(vec![node_index]);
    }

    // A node with no required inputs is satisfied by definition (the direct
    // match above always applies to it) and never gains a predecessor: an
    // empty `required` list is vacuously covered by every candidate's
    // outputs, which would otherwise manufacture meaningless zero-mapping
    // edges for every other declared capability.
    if !required.is_empty() {
        for (candidate_index, candidate) in declared.iter().enumerate() {
            if candidate_index == node_index || excluded.contains(&candidate_index) {
                continue;
            }
            if !schema_covers_required(&candidate.contract.outputs.schema, &required, node_schema) {
                continue;
            }
            if remaining_budget <= 1 {
                // A structurally valid predecessor exists but including it
                // would exceed the node-depth bound -- a real omission, not
                // an absence of alternatives.
                *truncated = true;
                continue;
            }
            let mut branch_excluded = excluded.clone();
            branch_excluded.insert(node_index);
            let sub_chains = build_chains(
                declared,
                starting_outputs,
                candidate_index,
                remaining_budget - 1,
                &branch_excluded,
                search_calls_remaining,
                truncated,
            );
            for mut sub_chain in sub_chains {
                sub_chain.push(node_index);
                result.push(sub_chain);
            }
        }
    }
    result
}

/// Whether every property named in `required` exists in `source_schema`'s
/// `properties` map and in `target_schema`'s `properties` map with the same
/// declared JSON `type` (spec 113 FR-002: an exact, declared-type match on
/// both sides -- an undeclared type on either side is not a match).
fn schema_covers_required(
    source_schema: &Value,
    required: &[String],
    target_schema: &Value,
) -> bool {
    required.iter().all(|property| {
        match (
            schema_property_type(source_schema, property),
            schema_property_type(target_schema, property),
        ) {
            (Some(source_type), Some(target_type)) => source_type == target_type,
            _ => false,
        }
    })
}

fn schema_property_type<'a>(schema: &'a Value, property: &str) -> Option<&'a str> {
    schema
        .get("properties")?
        .get(property)?
        .get("type")?
        .as_str()
}

fn schema_required(schema: &Value) -> Vec<String> {
    schema
        .get("required")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Synthesizes a producer-shaped `outputs.schema` fragment from the starting
/// facts, so root-node satisfiability can reuse the same `properties`/`type`
/// matching [`schema_covers_required`] uses for capability-to-capability
/// chaining, rather than a second comparison rule.
fn starting_facts_output_schema(starting_facts: &Value) -> Value {
    let mut properties = serde_json::Map::new();
    if let Some(object) = starting_facts.as_object() {
        for (key, value) in object {
            properties.insert(key.clone(), json!({"type": json_type_name(value)}));
        }
    }
    json!({"type": "object", "properties": Value::Object(properties)})
}

fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(number) if number.is_i64() || number.is_u64() => "integer",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// Renders one root-first node-index chain into a structurally submittable
/// `WorkflowProposal` (spec 113 FR-006). Each node's entire required input
/// set is sourced from exactly one place: `InitialInput` for the chain's
/// root, or its single immediate predecessor's output otherwise -- matching
/// how [`build_chains`] decided the chain was valid in the first place.
fn build_candidate(
    request: &PlanRequest<'_>,
    declared: &[DeclaredCapability],
    chain: &[usize],
    candidate_index: usize,
) -> PlanCandidate {
    let mut nodes = Vec::with_capacity(chain.len());
    let mut edges = Vec::new();
    let mut mappings = Vec::new();

    for (position, &node_index) in chain.iter().enumerate() {
        let capability = &declared[node_index];
        let node_id = format!("n{position}");
        nodes.push(ProposalNode {
            node_id: node_id.clone(),
            capability_id: capability.capability_id.clone(),
            capability_version: capability.capability_version.clone(),
            artifact_digest: capability.artifact_digest.clone(),
        });

        let required = schema_required(&capability.contract.inputs.schema);
        if position == 0 {
            for property in &required {
                mappings.push(ProposalMapping {
                    source: MappingSource::InitialInput,
                    source_path: format!("/{property}"),
                    target_node_id: node_id.clone(),
                    target_path: format!("/{property}"),
                });
            }
        } else {
            let predecessor_node_id = format!("n{}", position - 1);
            edges.push(ProposalEdge {
                from_node_id: predecessor_node_id.clone(),
                to_node_id: node_id.clone(),
            });
            for property in &required {
                mappings.push(ProposalMapping {
                    source: MappingSource::Node {
                        node_id: predecessor_node_id.clone(),
                    },
                    source_path: format!("/{property}"),
                    target_node_id: node_id.clone(),
                    target_path: format!("/{property}"),
                });
            }
        }
    }

    let proposal = WorkflowProposal {
        kind: "workflow_proposal".to_string(),
        schema_version: "1.0.0".to_string(),
        proposal_id: format!("plan-candidate-{candidate_index}"),
        workspace_id: request.workspace_id.to_string(),
        app_manifest: request.app_manifest.clone(),
        nodes,
        edges,
        mappings,
        initial_input: request.starting_facts.clone(),
    };

    PlanCandidate {
        proposal,
        mapping_unconfirmed: true,
    }
}

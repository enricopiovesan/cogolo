//! Runtime workflow proposal types, canonicalization, and digesting.
//!
//! Governed by spec `109-runtime-workflow-proposals` (P1) and ADR-0041. A
//! proposal is an untrusted, externally-authored, ephemeral bounded sequential
//! DAG over already-registered capabilities. This module owns the portable
//! parts of the lifecycle that need no manifest or registry access: the wire
//! format, canonical-JSON digesting, and structural validation (acyclic,
//! within configured limits, no dangling/ambiguous references). Cross-checks
//! against a loaded application manifest, capability registry, and risk
//! metadata live in `traverse-runtime`, which already depends on both this
//! crate and `traverse-registry`.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const PROPOSAL_KIND: &str = "workflow_proposal";
const PROPOSAL_SCHEMA_VERSION: &str = "1.0.0";
const PROPOSAL_DIGEST_VERSION: &str = "1.0.0";

/// A caller-submitted, ephemeral, manifest-bound workflow proposal (spec 109
/// FR-002). Every field the runtime authorizes or executes against must be
/// explicit here — no inference from schema shape alone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowProposal {
    pub kind: String,
    pub schema_version: String,
    pub proposal_id: String,
    /// Tenant/workspace scope this proposal is submitted under.
    pub workspace_id: String,
    pub app_manifest: ManifestReference,
    pub nodes: Vec<ProposalNode>,
    pub edges: Vec<ProposalEdge>,
    pub mappings: Vec<ProposalMapping>,
    /// Bound to `MappingSource::InitialInput` mappings; the only externally
    /// supplied data a proposal may inject into the graph.
    pub initial_input: Value,
}

/// Identifies the exact, already-registered application manifest version a
/// proposal is bounded by (spec 109 FR-002, ADR-0041: "a proposal is
/// constrained by its versioned application manifest").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestReference {
    pub app_id: String,
    pub app_version: String,
    pub manifest_digest: String,
}

/// One DAG node: an exact, pinned capability artifact (spec 109 FR-007a:
/// "exact resolved capability/artifact versions and digests").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposalNode {
    pub node_id: String,
    pub capability_id: String,
    pub capability_version: String,
    pub artifact_digest: String,
}

/// An explicit control-flow dependency: `to_node_id` may not start before
/// `from_node_id` reaches a terminal status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposalEdge {
    pub from_node_id: String,
    pub to_node_id: String,
}

/// An explicit source-path to target-path data mapping (spec 109 FR-002,
/// FR-011). Every field a node's input receives must arrive through one of
/// these — a node never sees another node's full output implicitly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposalMapping {
    pub source: MappingSource,
    /// JSON Pointer (RFC 6901) into the source's output (or `initial_input`).
    pub source_path: String,
    pub target_node_id: String,
    /// JSON Pointer (RFC 6901) into the target node's input.
    pub target_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum MappingSource {
    InitialInput,
    Node { node_id: String },
}

/// Configured structural limits a proposal must fall within (spec 109
/// FR-007). Values are runtime/host configuration, not caller-supplied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProposalLimits {
    pub max_nodes: usize,
    pub max_edges: usize,
    pub max_mappings: usize,
    pub max_initial_input_bytes: usize,
}

pub const DEFAULT_MAX_PROPOSAL_NODES: usize = 32;
pub const DEFAULT_MAX_PROPOSAL_EDGES: usize = 64;
pub const DEFAULT_MAX_PROPOSAL_MAPPINGS: usize = 128;
pub const DEFAULT_MAX_INITIAL_INPUT_BYTES: usize = 262_144;

impl Default for ProposalLimits {
    fn default() -> Self {
        Self {
            max_nodes: DEFAULT_MAX_PROPOSAL_NODES,
            max_edges: DEFAULT_MAX_PROPOSAL_EDGES,
            max_mappings: DEFAULT_MAX_PROPOSAL_MAPPINGS,
            max_initial_input_bytes: DEFAULT_MAX_INITIAL_INPUT_BYTES,
        }
    }
}

/// A structurally validated proposal with a deterministic execution order
/// (spec 109 FR-007a: "deterministic ready-node tie breaking").
#[derive(Debug, Clone, PartialEq)]
pub struct CanonicalProposal {
    pub proposal: WorkflowProposal,
    /// Node ids in deterministic topological execution order.
    pub execution_order: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProposalValidationFailure {
    pub errors: Vec<ProposalValidationError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProposalValidationError {
    pub code: ProposalValidationErrorCode,
    pub message: String,
    pub path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalValidationErrorCode {
    InvalidLiteral,
    MissingRequiredField,
    NodeLimitExceeded,
    EdgeLimitExceeded,
    MappingLimitExceeded,
    PayloadLimitExceeded,
    DuplicateNodeId,
    UnknownEdgeEndpoint,
    SelfLoopEdge,
    DuplicateEdge,
    CyclicGraph,
    UnknownMappingEndpoint,
    AmbiguousMultiWriterTarget,
    MissingDependencyEdgeForMapping,
}

/// Structurally validates a proposal and computes its deterministic
/// execution order. Performs no manifest, registry, or risk-metadata
/// cross-checks — those require host state and live in `traverse-runtime`.
///
/// # Errors
///
/// Returns [`ProposalValidationFailure`] when the proposal is malformed, over
/// a configured limit, cyclic, or contains a dangling/ambiguous reference.
#[allow(clippy::too_many_lines)]
pub fn canonicalize_proposal(
    proposal: WorkflowProposal,
    limits: &ProposalLimits,
) -> Result<CanonicalProposal, ProposalValidationFailure> {
    let mut errors = Vec::new();

    if proposal.kind != PROPOSAL_KIND {
        errors.push(error(
            ProposalValidationErrorCode::InvalidLiteral,
            "$.kind",
            "kind must equal workflow_proposal",
        ));
    }
    if proposal.schema_version != PROPOSAL_SCHEMA_VERSION {
        errors.push(error(
            ProposalValidationErrorCode::InvalidLiteral,
            "$.schema_version",
            "schema_version must equal 1.0.0",
        ));
    }
    validate_non_empty(&proposal.proposal_id, "$.proposal_id", &mut errors);
    validate_non_empty(&proposal.workspace_id, "$.workspace_id", &mut errors);

    if proposal.nodes.len() > limits.max_nodes {
        errors.push(error(
            ProposalValidationErrorCode::NodeLimitExceeded,
            "$.nodes",
            &format!(
                "proposal declares {} nodes, exceeding the configured limit of {}",
                proposal.nodes.len(),
                limits.max_nodes
            ),
        ));
    }
    if proposal.edges.len() > limits.max_edges {
        errors.push(error(
            ProposalValidationErrorCode::EdgeLimitExceeded,
            "$.edges",
            &format!(
                "proposal declares {} edges, exceeding the configured limit of {}",
                proposal.edges.len(),
                limits.max_edges
            ),
        ));
    }
    if proposal.mappings.len() > limits.max_mappings {
        errors.push(error(
            ProposalValidationErrorCode::MappingLimitExceeded,
            "$.mappings",
            &format!(
                "proposal declares {} mappings, exceeding the configured limit of {}",
                proposal.mappings.len(),
                limits.max_mappings
            ),
        ));
    }
    let initial_input_bytes =
        serde_json::to_vec(&proposal.initial_input).map_or(usize::MAX, |bytes| bytes.len());
    if initial_input_bytes > limits.max_initial_input_bytes {
        errors.push(error(
            ProposalValidationErrorCode::PayloadLimitExceeded,
            "$.initial_input",
            &format!(
                "initial_input is {initial_input_bytes} bytes, exceeding the configured limit of \
                 {} bytes",
                limits.max_initial_input_bytes
            ),
        ));
    }

    let mut node_ids: BTreeSet<String> = BTreeSet::new();
    for (index, node) in proposal.nodes.iter().enumerate() {
        let path = format!("$.nodes[{index}].node_id");
        validate_non_empty(&node.node_id, &path, &mut errors);
        validate_non_empty(
            &node.capability_id,
            &format!("$.nodes[{index}].capability_id"),
            &mut errors,
        );
        validate_non_empty(
            &node.capability_version,
            &format!("$.nodes[{index}].capability_version"),
            &mut errors,
        );
        validate_non_empty(
            &node.artifact_digest,
            &format!("$.nodes[{index}].artifact_digest"),
            &mut errors,
        );
        if !node_ids.insert(node.node_id.clone()) {
            errors.push(error(
                ProposalValidationErrorCode::DuplicateNodeId,
                &path,
                &format!("node_id '{}' is declared more than once", node.node_id),
            ));
        }
    }

    let mut adjacency: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut in_degree: BTreeMap<String, usize> =
        node_ids.iter().map(|id| (id.clone(), 0)).collect();
    let mut declared_edges: BTreeSet<(String, String)> = BTreeSet::new();
    for (index, edge) in proposal.edges.iter().enumerate() {
        let path = format!("$.edges[{index}]");
        if edge.from_node_id == edge.to_node_id {
            errors.push(error(
                ProposalValidationErrorCode::SelfLoopEdge,
                &path,
                &format!("edge from '{}' to itself is not allowed", edge.from_node_id),
            ));
            continue;
        }
        if !node_ids.contains(&edge.from_node_id) {
            errors.push(error(
                ProposalValidationErrorCode::UnknownEdgeEndpoint,
                &format!("{path}.from_node_id"),
                &format!("edge references unknown node_id '{}'", edge.from_node_id),
            ));
            continue;
        }
        if !node_ids.contains(&edge.to_node_id) {
            errors.push(error(
                ProposalValidationErrorCode::UnknownEdgeEndpoint,
                &format!("{path}.to_node_id"),
                &format!("edge references unknown node_id '{}'", edge.to_node_id),
            ));
            continue;
        }
        let key = (edge.from_node_id.clone(), edge.to_node_id.clone());
        if !declared_edges.insert(key) {
            errors.push(error(
                ProposalValidationErrorCode::DuplicateEdge,
                &path,
                &format!(
                    "edge '{}' -> '{}' is declared more than once",
                    edge.from_node_id, edge.to_node_id
                ),
            ));
            continue;
        }
        adjacency
            .entry(edge.from_node_id.clone())
            .or_default()
            .insert(edge.to_node_id.clone());
        *in_degree.entry(edge.to_node_id.clone()).or_insert(0) += 1;
    }

    let mut writer_targets: BTreeMap<(String, String), usize> = BTreeMap::new();
    for (index, mapping) in proposal.mappings.iter().enumerate() {
        let path = format!("$.mappings[{index}]");
        validate_non_empty(
            &mapping.source_path,
            &format!("{path}.source_path"),
            &mut errors,
        );
        validate_non_empty(
            &mapping.target_path,
            &format!("{path}.target_path"),
            &mut errors,
        );
        if !node_ids.contains(&mapping.target_node_id) {
            errors.push(error(
                ProposalValidationErrorCode::UnknownMappingEndpoint,
                &format!("{path}.target_node_id"),
                &format!(
                    "mapping targets unknown node_id '{}'",
                    mapping.target_node_id
                ),
            ));
            continue;
        }
        if let MappingSource::Node { node_id } = &mapping.source {
            if !node_ids.contains(node_id) {
                errors.push(error(
                    ProposalValidationErrorCode::UnknownMappingEndpoint,
                    &format!("{path}.source"),
                    &format!("mapping sources unknown node_id '{node_id}'"),
                ));
                continue;
            }
            if !declared_edges.contains(&(node_id.clone(), mapping.target_node_id.clone())) {
                errors.push(error(
                    ProposalValidationErrorCode::MissingDependencyEdgeForMapping,
                    &path,
                    &format!(
                        "mapping from '{node_id}' to '{}' has no corresponding declared edge",
                        mapping.target_node_id
                    ),
                ));
                continue;
            }
        }
        let writer_key = (mapping.target_node_id.clone(), mapping.target_path.clone());
        *writer_targets.entry(writer_key).or_insert(0) += 1;
    }
    for ((target_node_id, target_path), count) in &writer_targets {
        if *count > 1 {
            errors.push(error(
                ProposalValidationErrorCode::AmbiguousMultiWriterTarget,
                &format!(
                    "$.mappings[?target_node_id={target_node_id}][?target_path={target_path}]"
                ),
                &format!(
                    "target path '{target_path}' on node '{target_node_id}' is written by {count} \
                     mappings; a target path may have at most one writer"
                ),
            ));
        }
    }

    if !errors.is_empty() {
        return Err(ProposalValidationFailure { errors });
    }

    let Ok(execution_order) = topological_order(&node_ids, &adjacency, &in_degree) else {
        return Err(ProposalValidationFailure {
            errors: vec![error(
                ProposalValidationErrorCode::CyclicGraph,
                "$.edges",
                "proposal graph contains a cycle; P1 requires an acyclic graph",
            )],
        });
    };

    Ok(CanonicalProposal {
        proposal,
        execution_order,
    })
}

/// Kahn's algorithm with lexicographic tie-breaking among ready nodes,
/// satisfying spec 109 FR-007a's determinism requirement.
fn topological_order(
    node_ids: &BTreeSet<String>,
    adjacency: &BTreeMap<String, BTreeSet<String>>,
    in_degree: &BTreeMap<String, usize>,
) -> Result<Vec<String>, ()> {
    let mut remaining_in_degree = in_degree.clone();
    let mut ready: BTreeSet<String> = node_ids
        .iter()
        .filter(|id| remaining_in_degree.get(*id).copied().unwrap_or(0) == 0)
        .cloned()
        .collect();
    let mut order = Vec::with_capacity(node_ids.len());

    while let Some(next) = ready.iter().next().cloned() {
        ready.remove(&next);
        order.push(next.clone());
        let Some(successors) = adjacency.get(&next) else {
            continue;
        };
        for successor in successors {
            // Every successor was validated to be a declared node_id before
            // this function runs, and `remaining_in_degree` is seeded with
            // every declared node_id — this entry always already exists.
            let degree = remaining_in_degree.entry(successor.clone()).or_insert(0);
            *degree -= 1;
            if *degree == 0 {
                ready.insert(successor.clone());
            }
        }
    }

    if order.len() == node_ids.len() {
        Ok(order)
    } else {
        Err(())
    }
}

// ---------------------------------------------------------------------------
// Bounded parallel scheduling (spec 110 P2, ADR-0042)
// ---------------------------------------------------------------------------

/// Configured parallel-scheduling bounds a P2 execution schedule must fall
/// within (spec 110 FR-001, FR-005). Values are host configuration, never
/// caller-supplied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParallelScheduleLimits {
    /// Max node ids eligible to become ready and run concurrently in any
    /// single wave.
    pub max_fan_out: usize,
    /// Max direct predecessors (in-degree) converging into any single node.
    pub max_join_width: usize,
    /// Max total node ids across the whole schedule.
    pub max_queue_depth: usize,
    /// Max node executions the runtime may dispatch at once within a wave.
    pub max_concurrent_nodes: usize,
}

pub const DEFAULT_MAX_FAN_OUT: usize = 8;
pub const DEFAULT_MAX_JOIN_WIDTH: usize = 8;
pub const DEFAULT_MAX_QUEUE_DEPTH: usize = 16;
pub const DEFAULT_MAX_CONCURRENT_NODES: usize = 8;

impl Default for ParallelScheduleLimits {
    fn default() -> Self {
        Self {
            max_fan_out: DEFAULT_MAX_FAN_OUT,
            max_join_width: DEFAULT_MAX_JOIN_WIDTH,
            max_queue_depth: DEFAULT_MAX_QUEUE_DEPTH,
            max_concurrent_nodes: DEFAULT_MAX_CONCURRENT_NODES,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ParallelScheduleError {
    pub code: ParallelScheduleErrorCode,
    pub message: String,
    pub path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ParallelScheduleErrorCode {
    FanOutExceeded,
    JoinWidthExceeded,
    QueueDepthExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParallelScheduleFailure {
    pub errors: Vec<ParallelScheduleError>,
}

/// A bounded schedule of concurrency waves over an already-canonicalized
/// proposal (spec 110 FR-001, FR-002). Each wave is the set of node ids
/// whose dependencies are fully satisfied by earlier waves, sorted
/// lexicographically for deterministic dispatch order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParallelSchedule {
    pub waves: Vec<Vec<String>>,
}

/// Levelizes an already-canonicalized, acyclic proposal into concurrency
/// waves and checks it against configured fan-out/join-width/queue-depth
/// bounds (spec 110 FR-001, FR-005: reject before doing any work). Performs
/// no capability-contract lookups — the `pure_read`-only constraint
/// (FR-004a) requires resolved contracts and is enforced in
/// `traverse-runtime`.
///
/// # Errors
///
/// Returns [`ParallelScheduleFailure`] listing every exceeded bound.
pub fn compute_parallel_schedule(
    canonical: &CanonicalProposal,
    limits: &ParallelScheduleLimits,
) -> Result<ParallelSchedule, ParallelScheduleFailure> {
    let mut errors = Vec::new();

    let node_ids: BTreeSet<String> = canonical
        .proposal
        .nodes
        .iter()
        .map(|node| node.node_id.clone())
        .collect();
    if node_ids.len() > limits.max_queue_depth {
        errors.push(ParallelScheduleError {
            code: ParallelScheduleErrorCode::QueueDepthExceeded,
            message: format!(
                "schedule has {} nodes, exceeding the configured queue depth of {}",
                node_ids.len(),
                limits.max_queue_depth
            ),
            path: "$.nodes".to_string(),
        });
    }

    let mut adjacency: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut in_degree: BTreeMap<String, usize> =
        node_ids.iter().map(|id| (id.clone(), 0)).collect();
    for edge in &canonical.proposal.edges {
        adjacency
            .entry(edge.from_node_id.clone())
            .or_default()
            .insert(edge.to_node_id.clone());
        *in_degree.entry(edge.to_node_id.clone()).or_insert(0) += 1;
    }

    for (node_id, degree) in &in_degree {
        if *degree > limits.max_join_width {
            errors.push(ParallelScheduleError {
                code: ParallelScheduleErrorCode::JoinWidthExceeded,
                message: format!(
                    "node '{node_id}' has {degree} direct predecessors, exceeding the configured join width of {}",
                    limits.max_join_width
                ),
                path: format!("$.nodes[?node_id={node_id}]"),
            });
        }
    }

    let mut remaining_in_degree = in_degree.clone();
    let mut ready: BTreeSet<String> = node_ids
        .iter()
        .filter(|id| remaining_in_degree.get(*id).copied().unwrap_or(0) == 0)
        .cloned()
        .collect();
    let mut waves = Vec::new();
    while !ready.is_empty() {
        let wave: Vec<String> = ready.iter().cloned().collect();
        if wave.len() > limits.max_fan_out {
            errors.push(ParallelScheduleError {
                code: ParallelScheduleErrorCode::FanOutExceeded,
                message: format!(
                    "{} nodes became ready concurrently, exceeding the configured fan-out of {}",
                    wave.len(),
                    limits.max_fan_out
                ),
                path: "$.nodes".to_string(),
            });
        }
        let mut next_ready = BTreeSet::new();
        for node_id in &wave {
            if let Some(successors) = adjacency.get(node_id) {
                for successor in successors {
                    let degree = remaining_in_degree.entry(successor.clone()).or_insert(0);
                    *degree -= 1;
                    if *degree == 0 {
                        next_ready.insert(successor.clone());
                    }
                }
            }
        }
        waves.push(wave);
        ready = next_ready;
    }

    if errors.is_empty() {
        Ok(ParallelSchedule { waves })
    } else {
        Err(ParallelScheduleFailure { errors })
    }
}

/// Deterministic, independently-reproducible digest of a proposal's canonical
/// JSON form (spec 109 FR-003, FR-007a). Uses recursively key-sorted JSON
/// (not Rust `Debug` formatting) hashed with SHA-256, so an external proposer
/// can recompute the identical digest from the same JSON payload.
#[must_use]
pub fn proposal_digest(proposal: &WorkflowProposal) -> String {
    let value = serde_json::to_value(proposal).unwrap_or(Value::Null);
    digest_json_value(&value)
}

/// Binds a proposal digest to the pinned snapshot digests it was validated
/// against (spec 109 FR-003: "bind its digest to pinned manifest, registry,
/// binding, policy, and budget snapshots"). This is the digest an approval
/// token is scoped to (ADR-0041, FR-006a) — it changes if any governing
/// snapshot changes even when the proposal JSON is byte-identical.
#[must_use]
pub fn proposal_snapshot_digest(proposal_digest: &str, snapshots: &SnapshotDigests) -> String {
    let value = serde_json::json!({
        "proposal_digest": proposal_digest,
        "manifest_digest": snapshots.manifest_digest,
        "registry_digest": snapshots.registry_digest,
        "binding_digest": snapshots.binding_digest,
        "policy_digest": snapshots.policy_digest,
        "budget_digest": snapshots.budget_digest,
    });
    digest_json_value(&value)
}

/// The pinned snapshot digests a proposal's authorization is bound to (spec
/// 109 FR-003). Each field is a digest computed by the caller (typically
/// `traverse-runtime`) over the corresponding live host state at validation
/// time — this type only carries them, it does not compute them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotDigests {
    pub manifest_digest: String,
    pub registry_digest: String,
    pub binding_digest: String,
    pub policy_digest: String,
    pub budget_digest: String,
}

fn digest_json_value(value: &Value) -> String {
    let canonical = canonical_json_string(value);
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    let digest = hasher.finalize();
    format!("{PROPOSAL_DIGEST_VERSION}:sha256:{}", hex_encode(&digest))
}

fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// Serializes a JSON value with recursively sorted object keys and no
/// insignificant whitespace, so semantically identical JSON always produces
/// byte-identical output regardless of source field order.
fn canonical_json_string(value: &Value) -> String {
    let mut out = String::new();
    write_canonical(value, &mut out);
    out
}

fn write_canonical(value: &Value, out: &mut String) {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) => {
            out.push_str(&value.to_string());
        }
        Value::String(s) => {
            out.push_str(&serde_json::to_string(s).unwrap_or_default());
        }
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            out.push('{');
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for (index, key) in keys.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(key).unwrap_or_default());
                out.push(':');
                write_canonical(&map[*key], out);
            }
            out.push('}');
        }
    }
}

fn validate_non_empty(value: &str, path: &str, errors: &mut Vec<ProposalValidationError>) {
    if value.trim().is_empty() {
        errors.push(error(
            ProposalValidationErrorCode::MissingRequiredField,
            path,
            "value must be non-empty",
        ));
    }
}

fn error(code: ProposalValidationErrorCode, path: &str, message: &str) -> ProposalValidationError {
    ProposalValidationError {
        code,
        message: message.to_string(),
        path: path.to_string(),
    }
}

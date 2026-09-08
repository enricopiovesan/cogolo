//! Local governed handoff and execution of a browser-composed workflow
//! (spec `1277-browser-local-workflow-composition` FR-005/FR-006, issue #1270).
//!
//! [`browser_local_plan`](crate::browser_local_plan) produces an untrusted
//! [`BrowserWorkflowProposal`](crate::BrowserWorkflowProposal). Once a reviewer
//! has accepted it, [`execute_composed_workflow`] runs it entirely locally:
//!
//! 1. structural validation and deterministic ordering via spec 109
//!    [`canonicalize_proposal`];
//! 2. every node resolved to an **exact, already prepared and digest-verified**
//!    `registry_ref` dependency — a mismatch on capability identity, selected
//!    version, artifact digest, or the snapshot the dependency was prepared
//!    against fails closed with a stable secret-free error, and no substitute
//!    candidate, range re-resolution, fetch, sync, or bundle fallback is ever
//!    attempted (spec 1277 FR-006, spec 1258);
//! 3. execution through the spec 109 [`execute_proposal`] engine, which runs
//!    one node at a time in the canonical order, threads data only through the
//!    proposal's explicit mappings, and **stops at the first failed node** with
//!    no retry, compensation, replanning, or graph mutation (spec 1277 FR-006).
//!
//! The result is a redacted [`ProposalTrace`]: per-node outcomes, mapping
//! paths (never values), the bound snapshot digest, and a terminal state. The
//! browser is never a runtime authority — a proposal whose nodes are not all
//! automatically authorizable is rejected here rather than executed, because
//! this path carries no reviewer approval token (spec 1277 FR-005).

use serde_json::{Value, json};

use traverse_contracts::{
    CanonicalProposal, CapabilityContract, ProposalLimits, canonicalize_proposal, proposal_digest,
};
use traverse_registry::{
    ArtifactDigests, BinaryFormat, BinaryReference, CapabilityArtifactRecord,
    CapabilityRegistration, CapabilityRegistry, ComposabilityMetadata, CompositionKind,
    CompositionPattern, ImplementationKind, RegistryProvenance, RegistryScope, SourceKind,
    SourceReference,
};
use traverse_runtime::proposal::{
    AuthorizationSummary, ProposalTrace, ResolvedProposalNode, execute_proposal,
    proposal_is_automatic_eligible,
};
use traverse_runtime::{LocalExecutor, Runtime};

use crate::{BrowserWorkflowProposal, SnapshotIdentity, VerifiedRegistryDependency};

const GOVERNING_SPEC: &str = "1277-browser-local-workflow-composition";

/// Renders a validation failure's errors into one stable, secret-free string.
/// Branch-free so the redacted-detail path is always exercised.
fn join_errors<T>(errors: &[T], render: impl Fn(&T) -> String) -> String {
    errors.iter().map(render).collect::<Vec<_>>().join("; ")
}

/// Stable, secret-free failure classes for the local composed-workflow handoff
/// (spec 1277 FR-006/FR-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposedWorkflowErrorCode {
    /// The reviewed proposal is not bound to the supplied snapshot identity.
    SnapshotMismatch,
    /// The proposal is structurally invalid or exceeds a spec 109 limit.
    ProposalInvalid,
    /// A node names a capability with no prepared, verified dependency.
    MissingCapability,
    /// A prepared dependency was prepared against a different snapshot.
    DependencyEvidenceMismatch,
    /// A node's pinned artifact digest disagrees with its prepared dependency.
    ArtifactDigestDrift,
    /// A prepared dependency's bytes are not a valid capability contract.
    DependencyContractInvalid,
    /// The governed registry rejected a verified contract at registration.
    RegistryRejectedContract,
    /// One or more nodes are not automatically authorizable; this path holds
    /// no reviewer approval token.
    ApprovalRequired,
}

impl ComposedWorkflowErrorCode {
    /// Stable `snake_case` identifier.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ComposedWorkflowErrorCode::SnapshotMismatch => "composed_workflow_snapshot_mismatch",
            ComposedWorkflowErrorCode::ProposalInvalid => "composed_workflow_proposal_invalid",
            ComposedWorkflowErrorCode::MissingCapability => "composed_workflow_missing_capability",
            ComposedWorkflowErrorCode::DependencyEvidenceMismatch => {
                "composed_workflow_dependency_evidence_mismatch"
            }
            ComposedWorkflowErrorCode::ArtifactDigestDrift => {
                "composed_workflow_artifact_digest_drift"
            }
            ComposedWorkflowErrorCode::DependencyContractInvalid => {
                "composed_workflow_dependency_contract_invalid"
            }
            ComposedWorkflowErrorCode::RegistryRejectedContract => {
                "composed_workflow_registry_rejected_contract"
            }
            ComposedWorkflowErrorCode::ApprovalRequired => "composed_workflow_approval_required",
        }
    }
}

/// A stable, redacted composed-workflow failure. `node_id` and `detail` name
/// declared identities only — never paths, raw values, or bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposedWorkflowError {
    pub code: ComposedWorkflowErrorCode,
    pub node_id: Option<String>,
    pub detail: String,
}

impl ComposedWorkflowError {
    fn new(code: ComposedWorkflowErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            node_id: None,
            detail: detail.into(),
        }
    }

    fn at_node(code: ComposedWorkflowErrorCode, node_id: &str, detail: impl Into<String>) -> Self {
        Self {
            code,
            node_id: Some(node_id.to_string()),
            detail: detail.into(),
        }
    }

    /// Secret-free JSON projection.
    #[must_use]
    pub fn as_value(&self) -> Value {
        json!({
            "code": self.code.as_str(),
            "node_id": self.node_id,
            "detail": self.detail,
        })
    }
}

type BoundNode = (String, CapabilityContract, CapabilityArtifactRecord);

/// Resolves every canonical node to its exact, already prepared and
/// digest-verified `registry_ref` dependency (spec 1277 FR-006, spec 1258),
/// failing closed on any identity, snapshot, or digest drift.
fn bind_composed_nodes(
    canonical: &CanonicalProposal,
    snapshot_identity: &SnapshotIdentity,
    verified_dependencies: &[VerifiedRegistryDependency],
) -> Result<Vec<BoundNode>, ComposedWorkflowError> {
    let mut bound = Vec::with_capacity(canonical.proposal.nodes.len());
    for node in &canonical.proposal.nodes {
        let dependency = verified_dependencies
            .iter()
            .find(|dependency| {
                dependency.evidence.id == node.capability_id
                    && dependency.evidence.selected_version == node.capability_version
            })
            .ok_or_else(|| {
                ComposedWorkflowError::at_node(
                    ComposedWorkflowErrorCode::MissingCapability,
                    &node.node_id,
                    format!(
                        "no prepared verified dependency for {}@{}",
                        node.capability_id, node.capability_version
                    ),
                )
            })?;

        if dependency.evidence.index_digest != snapshot_identity.registry_snapshot_digest {
            return Err(ComposedWorkflowError::at_node(
                ComposedWorkflowErrorCode::DependencyEvidenceMismatch,
                &node.node_id,
                format!(
                    "dependency {}@{} was prepared against a different snapshot",
                    node.capability_id, node.capability_version
                ),
            ));
        }
        if node.artifact_digest != dependency.wasm_digest
            || node.artifact_digest != dependency.evidence.artifact_digest
        {
            return Err(ComposedWorkflowError::at_node(
                ComposedWorkflowErrorCode::ArtifactDigestDrift,
                &node.node_id,
                format!(
                    "node pins an artifact digest that disagrees with dependency {}@{}",
                    node.capability_id, node.capability_version
                ),
            ));
        }

        let contract: CapabilityContract = serde_json::from_slice(&dependency.contract_bytes)
            .map_err(|_| {
                ComposedWorkflowError::at_node(
                    ComposedWorkflowErrorCode::DependencyContractInvalid,
                    &node.node_id,
                    format!(
                        "dependency {}@{} contract bytes are not a capability contract",
                        node.capability_id, node.capability_version
                    ),
                )
            })?;

        bound.push((
            node.node_id.clone(),
            contract,
            verified_artifact_record(dependency),
        ));
    }
    Ok(bound)
}

/// Registers each bound node's verified contract and artifact into a fresh
/// private registry, failing closed if the governed validator rejects one.
fn build_composed_registry(
    bound: Vec<BoundNode>,
) -> Result<CapabilityRegistry, ComposedWorkflowError> {
    let mut registry = CapabilityRegistry::new();
    for (_, contract, artifact) in bound {
        registry
            .register(CapabilityRegistration {
                scope: RegistryScope::Private,
                contract,
                contract_path: "browser-composed/verified-cache".to_string(),
                artifact,
                registered_at: "browser-composed".to_string(),
                tags: Vec::new(),
                composability: ComposabilityMetadata {
                    kind: CompositionKind::Atomic,
                    patterns: vec![CompositionPattern::Sequential],
                    provides: Vec::new(),
                    requires: Vec::new(),
                },
                governing_spec: GOVERNING_SPEC.to_string(),
                validator_version: env!("CARGO_PKG_VERSION").to_string(),
            })
            .map_err(|failure| {
                ComposedWorkflowError::new(
                    ComposedWorkflowErrorCode::RegistryRejectedContract,
                    join_errors(&failure.errors, |error| {
                        format!("{}: {}", error.target, error.message)
                    }),
                )
            })?;
    }
    Ok(registry)
}

/// Executes a reviewed browser-composed workflow locally and offline.
///
/// `executor` is the host's local WASM executor — in production
/// `traverse_runtime::ArtifactRouter::new()`. Every node's artifact is served
/// from the digest-verified path recorded on its [`VerifiedRegistryDependency`].
///
/// # Errors
///
/// Returns a [`ComposedWorkflowError`] when the proposal is not bound to
/// `snapshot_identity`, is structurally invalid, names a capability with no
/// prepared verified dependency, pins a digest that disagrees with that
/// dependency, or requires reviewer approval this path cannot provide. A node
/// that runs and fails is **not** an error here: it produces a `ProposalTrace`
/// with a `Failed` terminal state and the later nodes skipped.
pub fn execute_composed_workflow<E: LocalExecutor>(
    reviewed: &BrowserWorkflowProposal,
    snapshot_identity: &SnapshotIdentity,
    verified_dependencies: &[VerifiedRegistryDependency],
    executor: E,
    security: crate::SecurityPosture,
) -> Result<ProposalTrace, ComposedWorkflowError> {
    if reviewed.snapshot_digest != snapshot_identity.registry_snapshot_digest {
        return Err(ComposedWorkflowError::new(
            ComposedWorkflowErrorCode::SnapshotMismatch,
            "reviewed proposal is not bound to the supplied snapshot identity",
        ));
    }

    let canonical = canonicalize_proposal(reviewed.proposal.clone(), &ProposalLimits::default())
        .map_err(|failure| {
            ComposedWorkflowError::new(
                ComposedWorkflowErrorCode::ProposalInvalid,
                join_errors(&failure.errors, |error| {
                    format!("{:?}: {}", error.code, error.path)
                }),
            )
        })?;

    let bound = bind_composed_nodes(&canonical, snapshot_identity, verified_dependencies)?;
    let resolved_nodes: Vec<ResolvedProposalNode> = bound
        .iter()
        .map(|(node_id, contract, _)| ResolvedProposalNode {
            node_id: node_id.clone(),
            contract: contract.clone(),
        })
        .collect();

    if !proposal_is_automatic_eligible(&resolved_nodes) {
        return Err(ComposedWorkflowError::new(
            ComposedWorkflowErrorCode::ApprovalRequired,
            "one or more nodes require reviewer approval this path cannot provide",
        ));
    }

    let registry = build_composed_registry(bound)?;

    let security = match security {
        crate::SecurityPosture::Production => {
            traverse_runtime::security::RuntimeSecurityConfig::production()
        }
        crate::SecurityPosture::Development => {
            traverse_runtime::security::RuntimeSecurityConfig::development()
        }
    };
    let runtime = Runtime::new(registry, executor).with_security_config(security);

    let digest = proposal_digest(&canonical.proposal);
    Ok(execute_proposal(
        &runtime,
        &canonical,
        &resolved_nodes,
        AuthorizationSummary {
            automatic: true,
            approval_token_id: None,
        },
        &digest,
        &reviewed.snapshot_digest,
    ))
}

/// A [`CapabilityArtifactRecord`] pointing at the dependency's already
/// digest-verified WASM bytes on disk.
fn verified_artifact_record(dependency: &VerifiedRegistryDependency) -> CapabilityArtifactRecord {
    CapabilityArtifactRecord {
        artifact_ref: format!("verified:{}", dependency.wasm_digest),
        implementation_kind: ImplementationKind::Executable,
        source: SourceReference {
            kind: SourceKind::Git,
            location: dependency.evidence.source_release.clone(),
        },
        binary: Some(BinaryReference {
            format: BinaryFormat::Wasm,
            location: dependency.wasm_binary_path.to_string_lossy().into_owned(),
            signature: None,
        }),
        workflow_ref: None,
        digests: ArtifactDigests {
            source_digest: dependency.wasm_digest.clone(),
            binary_digest: Some(dependency.wasm_digest.clone()),
        },
        provenance: RegistryProvenance {
            source: "public-registry".to_string(),
            author: dependency.evidence.namespace.clone(),
            created_at: "browser-composed".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;
    use crate::{RegistryPrepareEvidence, SecurityPosture};
    use traverse_contracts::{
        DeterminismClass, EffectClass, ManifestReference, MappingSource, ProposalEdge,
        ProposalMapping, ProposalNode, ReliabilityMetadata, RiskMetadata, WorkflowProposal,
    };
    use traverse_runtime::{
        LocalExecutionFailure, LocalExecutionFailureCode, LocalExecutionOutput,
    };

    const BASE_CONTRACT: &str = include_str!(
        "../../../contracts/examples/meeting-notes/capabilities/process/contract.json"
    );

    const SNAPSHOT_DIGEST: &str =
        "sha256:1111111111111111111111111111111111111111111111111111111111111111";
    const SOURCE_RELEASE: &str = "registry-v243";

    fn digest_for(marker: &str) -> String {
        format!("sha256:{marker:0>64}")
    }

    fn identity() -> SnapshotIdentity {
        SnapshotIdentity {
            registry_snapshot_digest: SNAPSHOT_DIGEST.to_string(),
            source_release: SOURCE_RELEASE.to_string(),
            contract_schema_version: crate::SUPPORTED_CONTRACT_SCHEMA_VERSION.to_string(),
        }
    }

    fn automatic_risk() -> RiskMetadata {
        RiskMetadata {
            effect_class: EffectClass::PureRead,
            determinism_class: DeterminismClass::Deterministic,
            data_flow: traverse_contracts::DataFlowPolicy::default(),
            reliability: ReliabilityMetadata {
                idempotency_required: false,
                retryable: false,
                compensation_available: false,
            },
        }
    }

    fn contract(
        id: &str,
        version: &str,
        inputs: &[(&str, &str)],
        outputs: &[(&str, &str)],
        automatic: bool,
    ) -> CapabilityContract {
        let mut parsed: CapabilityContract =
            serde_json::from_str(BASE_CONTRACT).expect("base contract parses");
        parsed.id = id.to_string();
        parsed.namespace = id.split('.').next().unwrap_or(id).to_string();
        parsed.name = id.rsplit('.').next().unwrap_or(id).to_string();
        parsed.version = version.to_string();
        parsed.inputs.schema = schema(inputs);
        parsed.outputs.schema = schema(outputs);
        parsed.risk = if automatic {
            automatic_risk()
        } else {
            traverse_contracts::default_risk_metadata()
        };
        parsed
    }

    fn schema(properties: &[(&str, &str)]) -> Value {
        let mut map = serde_json::Map::new();
        let mut required = Vec::new();
        for (name, ty) in properties {
            map.insert((*name).to_string(), json!({ "type": ty }));
            required.push(Value::String((*name).to_string()));
        }
        json!({ "type": "object", "required": required, "properties": Value::Object(map) })
    }

    fn dependency(contract: &CapabilityContract, digest: &str) -> VerifiedRegistryDependency {
        VerifiedRegistryDependency {
            wasm_binary_path: format!("/verified-cache/{digest}.wasm").into(),
            contract_path: format!("/verified-cache/{digest}.json").into(),
            contract_bytes: serde_json::to_vec(contract).expect("contract serializes"),
            wasm_digest: digest.to_string(),
            evidence: RegistryPrepareEvidence {
                namespace: contract.namespace.clone(),
                id: contract.id.clone(),
                selected_version: contract.version.clone(),
                version_range: format!("={}", contract.version),
                source_release: SOURCE_RELEASE.to_string(),
                index_digest: SNAPSHOT_DIGEST.to_string(),
                artifact_digest: digest.to_string(),
                verified_at: 1_757_000_000,
                outcome: "prepared",
            },
        }
    }

    fn node(node_id: &str, contract: &CapabilityContract, digest: &str) -> ProposalNode {
        ProposalNode {
            node_id: node_id.to_string(),
            capability_id: contract.id.clone(),
            capability_version: contract.version.clone(),
            artifact_digest: digest.to_string(),
        }
    }

    fn reviewed(
        nodes: Vec<ProposalNode>,
        edges: Vec<ProposalEdge>,
        mappings: Vec<ProposalMapping>,
        initial_input: Value,
        snapshot_digest: &str,
    ) -> BrowserWorkflowProposal {
        BrowserWorkflowProposal {
            kind: "browser_workflow_proposal",
            schema_version: crate::BROWSER_WORKFLOW_PROPOSAL_SCHEMA_VERSION,
            snapshot_digest: snapshot_digest.to_string(),
            source_release: SOURCE_RELEASE.to_string(),
            proposal: WorkflowProposal {
                kind: "workflow_proposal".to_string(),
                schema_version: "1.0.0".to_string(),
                proposal_id: "reviewed-1".to_string(),
                workspace_id: "local".to_string(),
                app_manifest: ManifestReference {
                    app_id: "discover.demo".to_string(),
                    app_version: "1.0.0".to_string(),
                    manifest_digest: digest_for("m"),
                },
                nodes,
                edges,
                mappings,
                initial_input,
            },
            mapping_unconfirmed: false,
        }
    }

    struct FakeExecutor;
    impl LocalExecutor for FakeExecutor {
        fn execute(
            &self,
            capability: &traverse_registry::ResolvedCapability,
            _input: &Value,
        ) -> Result<LocalExecutionOutput, LocalExecutionFailure> {
            // Produce an output object that satisfies the contract's declared
            // required output fields, so the runtime's output-schema check
            // passes and the executor seam itself is what is under test.
            let mut output = serde_json::Map::new();
            let required = capability.contract.outputs.schema["required"]
                .as_array()
                .expect("test output schemas always declare required fields");
            for field in required.iter().filter_map(Value::as_str) {
                output.insert(field.to_string(), json!("produced"));
            }
            Ok(LocalExecutionOutput {
                value: Value::Object(output),
                emitted_events: Vec::new(),
            })
        }
    }

    struct FailingExecutor;
    impl LocalExecutor for FailingExecutor {
        fn execute(
            &self,
            _capability: &traverse_registry::ResolvedCapability,
            _input: &Value,
        ) -> Result<LocalExecutionOutput, LocalExecutionFailure> {
            Err(LocalExecutionFailure {
                code: LocalExecutionFailureCode::ExecutionFailed,
                message: "node blew up".to_string(),
            })
        }
    }

    /// A → B, where A produces `text` from `seed` and B consumes `text`.
    fn two_node_world() -> (BrowserWorkflowProposal, Vec<VerifiedRegistryDependency>) {
        let produce = contract(
            "evidence.produce",
            "1.0.0",
            &[("seed", "string")],
            &[("text", "string")],
            true,
        );
        let consume = contract(
            "evidence.consume",
            "1.0.0",
            &[("text", "string")],
            &[("done", "string")],
            true,
        );
        let da = digest_for("a");
        let db = digest_for("b");
        let proposal = reviewed(
            vec![node("n0", &produce, &da), node("n1", &consume, &db)],
            vec![ProposalEdge {
                from_node_id: "n0".to_string(),
                to_node_id: "n1".to_string(),
            }],
            vec![
                ProposalMapping {
                    source: MappingSource::InitialInput,
                    source_path: "/seed".to_string(),
                    target_node_id: "n0".to_string(),
                    target_path: "/seed".to_string(),
                },
                ProposalMapping {
                    source: MappingSource::Node {
                        node_id: "n0".to_string(),
                    },
                    source_path: "/text".to_string(),
                    target_node_id: "n1".to_string(),
                    target_path: "/text".to_string(),
                },
            ],
            json!({ "seed": "hello" }),
            SNAPSHOT_DIGEST,
        );
        let deps = vec![dependency(&produce, &da), dependency(&consume, &db)];
        (proposal, deps)
    }

    #[test]
    fn executes_a_two_node_plan_and_returns_a_redacted_trace() {
        let (proposal, deps) = two_node_world();
        let trace = execute_composed_workflow(
            &proposal,
            &identity(),
            &deps,
            FakeExecutor,
            SecurityPosture::Development,
        )
        .expect("composed workflow executes");
        assert_eq!(
            trace.terminal_state,
            traverse_runtime::proposal::ProposalTerminalState::Succeeded
        );
        assert_eq!(trace.node_outcomes.len(), 2);
        assert_eq!(trace.snapshot_digest, SNAPSHOT_DIGEST);
        let serialized = serde_json::to_string(&trace).expect("trace serializes");
        assert!(
            !serialized.contains("/verified-cache/"),
            "trace leaked a path: {serialized}"
        );
    }

    #[test]
    fn production_security_posture_is_honored() {
        // Browser-composed artifacts carry digest + lifecycle verification
        // (spec 1258); a host running the Production posture additionally
        // requires signed artifacts (spec 065). The unsigned fixture artifact
        // is therefore rejected by the runtime and the trace reports a failed
        // node -- the posture is threaded through, not ignored.
        let (proposal, deps) = two_node_world();
        let trace = execute_composed_workflow(
            &proposal,
            &identity(),
            &deps,
            FakeExecutor,
            SecurityPosture::Production,
        )
        .expect("composed workflow produces a trace under production posture");
        assert_eq!(
            trace.terminal_state,
            traverse_runtime::proposal::ProposalTerminalState::Failed
        );
    }

    #[test]
    fn a_failing_node_halts_execution_without_replan() {
        let (proposal, deps) = two_node_world();
        let trace = execute_composed_workflow(
            &proposal,
            &identity(),
            &deps,
            FailingExecutor,
            SecurityPosture::Development,
        )
        .expect("a node failure is a trace, not an error");
        assert_eq!(
            trace.terminal_state,
            traverse_runtime::proposal::ProposalTerminalState::Failed
        );
        assert_eq!(
            trace.node_outcomes[0].status,
            traverse_runtime::proposal::ProposalNodeStatus::Failed
        );
        assert_eq!(
            trace.node_outcomes[1].status,
            traverse_runtime::proposal::ProposalNodeStatus::SkippedAfterEarlierFailure
        );
    }

    #[test]
    fn snapshot_mismatch_fails_closed() {
        let (mut proposal, deps) = two_node_world();
        proposal.snapshot_digest = digest_for("deadbeef");
        let error = execute_composed_workflow(
            &proposal,
            &identity(),
            &deps,
            FakeExecutor,
            SecurityPosture::Development,
        )
        .expect_err("snapshot mismatch must fail");
        assert_eq!(error.code, ComposedWorkflowErrorCode::SnapshotMismatch);
    }

    #[test]
    fn structurally_invalid_proposal_fails_closed() {
        let (mut proposal, deps) = two_node_world();
        // Self-loop edge -> structural validation rejects the proposal.
        proposal.proposal.edges.push(ProposalEdge {
            from_node_id: "n0".to_string(),
            to_node_id: "n0".to_string(),
        });
        let error = execute_composed_workflow(
            &proposal,
            &identity(),
            &deps,
            FakeExecutor,
            SecurityPosture::Development,
        )
        .expect_err("invalid proposal must fail");
        assert_eq!(error.code, ComposedWorkflowErrorCode::ProposalInvalid);
    }

    #[test]
    fn missing_capability_fails_closed_with_node_id() {
        let (proposal, mut deps) = two_node_world();
        deps.pop();
        let error = execute_composed_workflow(
            &proposal,
            &identity(),
            &deps,
            FakeExecutor,
            SecurityPosture::Development,
        )
        .expect_err("missing capability must fail");
        assert_eq!(error.code, ComposedWorkflowErrorCode::MissingCapability);
        assert_eq!(error.node_id.as_deref(), Some("n1"));
    }

    #[test]
    fn dependency_prepared_against_a_different_snapshot_fails_closed() {
        let (proposal, mut deps) = two_node_world();
        deps[1].evidence.index_digest = digest_for("otherdigest");
        let error = execute_composed_workflow(
            &proposal,
            &identity(),
            &deps,
            FakeExecutor,
            SecurityPosture::Development,
        )
        .expect_err("cross-snapshot dependency must fail");
        assert_eq!(
            error.code,
            ComposedWorkflowErrorCode::DependencyEvidenceMismatch
        );
    }

    #[test]
    fn artifact_digest_drift_fails_closed() {
        let (mut proposal, deps) = two_node_world();
        proposal.proposal.nodes[0].artifact_digest = digest_for("drift");
        let error = execute_composed_workflow(
            &proposal,
            &identity(),
            &deps,
            FakeExecutor,
            SecurityPosture::Development,
        )
        .expect_err("digest drift must fail");
        assert_eq!(error.code, ComposedWorkflowErrorCode::ArtifactDigestDrift);
    }

    #[test]
    fn invalid_dependency_contract_bytes_fails_closed() {
        let (proposal, mut deps) = two_node_world();
        deps[0].contract_bytes = b"not a contract".to_vec();
        let error = execute_composed_workflow(
            &proposal,
            &identity(),
            &deps,
            FakeExecutor,
            SecurityPosture::Development,
        )
        .expect_err("invalid contract bytes must fail");
        assert_eq!(
            error.code,
            ComposedWorkflowErrorCode::DependencyContractInvalid
        );
    }

    #[test]
    fn a_non_automatic_node_requires_reviewer_approval() {
        let produce = contract(
            "evidence.produce",
            "1.0.0",
            &[("seed", "string")],
            &[("text", "string")],
            false,
        );
        let da = digest_for("a");
        let proposal = reviewed(
            vec![node("n0", &produce, &da)],
            Vec::new(),
            vec![ProposalMapping {
                source: MappingSource::InitialInput,
                source_path: "/seed".to_string(),
                target_node_id: "n0".to_string(),
                target_path: "/seed".to_string(),
            }],
            json!({ "seed": "x" }),
            SNAPSHOT_DIGEST,
        );
        let deps = vec![dependency(&produce, &da)];
        let error = execute_composed_workflow(
            &proposal,
            &identity(),
            &deps,
            FakeExecutor,
            SecurityPosture::Development,
        )
        .expect_err("a non-automatic node must require approval");
        assert_eq!(error.code, ComposedWorkflowErrorCode::ApprovalRequired);
    }

    #[test]
    fn registry_rejecting_a_verified_contract_fails_closed() {
        let (mut proposal, mut deps) = two_node_world();
        proposal.proposal.nodes.truncate(1);
        proposal.proposal.edges.clear();
        proposal
            .proposal
            .mappings
            .retain(|m| m.target_node_id == "n0");
        deps.truncate(1);
        let mut broken: CapabilityContract =
            serde_json::from_slice(&deps[0].contract_bytes).expect("contract parses");
        broken.owner.team = String::new();
        deps[0].contract_bytes = serde_json::to_vec(&broken).expect("contract serializes");
        let error = execute_composed_workflow(
            &proposal,
            &identity(),
            &deps,
            FakeExecutor,
            SecurityPosture::Development,
        )
        .expect_err("a contract that fails validation must fail closed");
        assert_eq!(
            error.code,
            ComposedWorkflowErrorCode::RegistryRejectedContract
        );
    }

    #[test]
    fn every_error_code_has_a_distinct_stable_slug() {
        use std::collections::BTreeSet;
        let codes = [
            ComposedWorkflowErrorCode::SnapshotMismatch,
            ComposedWorkflowErrorCode::ProposalInvalid,
            ComposedWorkflowErrorCode::MissingCapability,
            ComposedWorkflowErrorCode::DependencyEvidenceMismatch,
            ComposedWorkflowErrorCode::ArtifactDigestDrift,
            ComposedWorkflowErrorCode::DependencyContractInvalid,
            ComposedWorkflowErrorCode::RegistryRejectedContract,
            ComposedWorkflowErrorCode::ApprovalRequired,
        ];
        let slugs: BTreeSet<&str> = codes.iter().map(|code| code.as_str()).collect();
        assert_eq!(slugs.len(), codes.len());
        for code in codes {
            assert!(code.as_str().starts_with("composed_workflow_"));
        }
        let value = ComposedWorkflowError::at_node(
            ComposedWorkflowErrorCode::MissingCapability,
            "n1",
            "detail",
        )
        .as_value();
        assert_eq!(value["node_id"], "n1");
        assert_eq!(value["code"], "composed_workflow_missing_capability");
    }

    #[test]
    fn the_real_artifact_router_is_accepted_as_the_executor() {
        // The production executor wires in without a panic; with no real WASM
        // at the verified-cache path the node fails and the trace reports it.
        let (proposal, deps) = two_node_world();
        let executor = traverse_runtime::ArtifactRouter::new()
            .expect("bounded Wasmtime configuration initializes");
        let trace = execute_composed_workflow(
            &proposal,
            &identity(),
            &deps,
            executor,
            SecurityPosture::Development,
        )
        .expect("composed workflow produces a trace");
        assert_eq!(
            trace.terminal_state,
            traverse_runtime::proposal::ProposalTerminalState::Failed
        );
    }
}

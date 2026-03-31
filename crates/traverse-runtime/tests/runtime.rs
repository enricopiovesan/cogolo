use serde_json::{Value, json};
use traverse_contracts::{
    BinaryFormat as ContractBinaryFormat, Condition, DependencyReference, Entrypoint,
    EntrypointKind, EventReference, Execution, ExecutionConstraints, ExecutionTarget,
    FilesystemAccess, HostApiAccess, IdReference, Lifecycle, NetworkAccess, Owner, Provenance,
    ProvenanceSource, SchemaContainer, SideEffect, SideEffectKind,
};
use traverse_registry::{
    ArtifactDigests, BinaryFormat, BinaryReference, CapabilityArtifactRecord,
    CapabilityRegistration, CapabilityRegistry, ComposabilityMetadata, CompositionKind,
    CompositionPattern, ImplementationKind, RegistryProvenance, RegistryScope, SourceKind,
    SourceReference,
};
use traverse_runtime::{
    CandidateReason, ExecutionFailureReason, ExecutionStatus, LocalExecutionFailure,
    LocalExecutionFailureCode, LocalExecutor, PlacementTarget, Runtime, RuntimeContext,
    RuntimeErrorCode, RuntimeLookup, RuntimeLookupScope, RuntimeRequest, RuntimeResultStatus,
    RuntimeState, SelectionFailureReason, SelectionStatus, parse_runtime_request,
};

#[test]
fn parses_runtime_request_from_json() {
    let request = parse_runtime_request(&base_request().to_string());

    assert_eq!(
        request.as_ref().map(|item| item.request_id.as_str()),
        Ok("req-123")
    );
    assert_eq!(
        request.as_ref().map(|item| item.governing_spec.as_str()),
        Ok("006-runtime-request-execution")
    );
}

#[test]
fn executes_one_exact_registered_capability_locally() {
    let runtime = Runtime::new(
        registry_with(vec![registration(
            RegistryScope::Private,
            "content.comments.create-comment-draft",
            "1.0.0",
            Lifecycle::Active,
        )]),
        EchoExecutor,
    );

    let outcome = runtime.execute(base_request_exact());

    assert_eq!(outcome.result.status, RuntimeResultStatus::Completed);
    assert_eq!(
        outcome.result.output,
        Some(json!({"draft_id": "draft-001"}))
    );
    assert_eq!(
        states(&outcome.state_events),
        vec![
            RuntimeState::LoadingRegistry,
            RuntimeState::Ready,
            RuntimeState::Discovering,
            RuntimeState::EvaluatingConstraints,
            RuntimeState::Selecting,
            RuntimeState::Executing,
            RuntimeState::EmittingEvents,
            RuntimeState::Completed,
            RuntimeState::Ready,
        ]
    );
    assert_eq!(outcome.trace.selection.status, SelectionStatus::Selected);
    assert_eq!(outcome.trace.candidate_collection.candidates.len(), 1);
    assert_eq!(
        outcome.trace.candidate_collection.candidates[0].reason,
        CandidateReason::ExactMatch
    );
    assert_eq!(outcome.trace.execution.status, ExecutionStatus::Succeeded);
    assert!(outcome.trace.execution.output_digest.is_some());
}

#[test]
fn exact_lookup_uses_private_overlay_before_public() {
    let runtime = Runtime::new(
        registry_with(vec![
            registration(
                RegistryScope::Public,
                "content.comments.create-comment-draft",
                "1.0.0",
                Lifecycle::Active,
            ),
            registration(
                RegistryScope::Private,
                "content.comments.create-comment-draft",
                "1.0.0",
                Lifecycle::Active,
            ),
        ]),
        EchoExecutor,
    );

    let outcome = runtime.execute(base_request_exact());

    assert_eq!(
        outcome.trace.candidate_collection.candidates[0].scope,
        traverse_runtime::RuntimeRegistryScope::Private
    );
}

#[test]
fn discovers_by_intent_key_and_fails_when_no_candidate_matches() {
    let runtime = Runtime::new(registry_with(vec![]), EchoExecutor);
    let mut request = base_request_exact();
    request.intent.capability_id = None;
    request.intent.capability_version = None;
    request.intent.intent_key = Some("content.comments.create-comment-draft".to_string());

    let outcome = runtime.execute(request);

    assert_eq!(outcome.result.status, RuntimeResultStatus::Error);
    assert_eq!(
        outcome.result.error.as_ref().map(|error| error.code),
        Some(RuntimeErrorCode::CapabilityNotFound)
    );
    assert_eq!(outcome.trace.selection.status, SelectionStatus::NoMatch);
    assert_eq!(
        outcome.trace.selection.failure_reason,
        Some(SelectionFailureReason::NoMatch)
    );
    assert!(outcome.trace.candidate_collection.candidates.is_empty());
    assert!(matches!(
        outcome.trace.execution.failure_reason,
        Some(ExecutionFailureReason::ArtifactNotRunnable)
    ));
}

#[test]
fn rejects_ambiguous_intent_matches() {
    let runtime = Runtime::new(
        registry_with(vec![
            registration(
                RegistryScope::Private,
                "content.comments.create-comment-draft",
                "1.1.0",
                Lifecycle::Active,
            ),
            registration(
                RegistryScope::Private,
                "content.comments.create-comment-draft",
                "1.0.0",
                Lifecycle::Active,
            ),
        ]),
        EchoExecutor,
    );
    let mut request = base_request_exact();
    request.intent.capability_id = None;
    request.intent.capability_version = None;
    request.intent.intent_key = Some("content.comments.create-comment-draft".to_string());

    let outcome = runtime.execute(request);

    assert_eq!(
        outcome.result.error.as_ref().map(|error| error.code),
        Some(RuntimeErrorCode::CapabilityAmbiguous)
    );
    assert_eq!(outcome.trace.selection.status, SelectionStatus::Ambiguous);
    assert_eq!(outcome.trace.selection.remaining_candidates.len(), 2);
}

#[test]
fn rejects_invalid_request_before_discovery() {
    let runtime = Runtime::new(registry_with(vec![]), EchoExecutor);
    let mut request = base_request_exact();
    request.lookup.allow_ambiguity = true;

    let outcome = runtime.execute(request);

    assert_eq!(
        outcome.result.error.as_ref().map(|error| error.code),
        Some(RuntimeErrorCode::RequestInvalid)
    );
    assert_eq!(
        states(&outcome.state_events),
        vec![
            RuntimeState::LoadingRegistry,
            RuntimeState::Ready,
            RuntimeState::Discovering,
            RuntimeState::EvaluatingConstraints,
            RuntimeState::Error,
            RuntimeState::Ready
        ]
    );
    assert_eq!(
        outcome.trace.selection.status,
        SelectionStatus::InvalidRequest
    );
    assert_eq!(
        outcome.trace.selection.failure_reason,
        Some(SelectionFailureReason::InvalidRequest)
    );
}

#[test]
fn rejects_non_runnable_candidates_before_execution() {
    let mut not_runnable = registration(
        RegistryScope::Private,
        "content.comments.create-comment-draft",
        "1.0.0",
        Lifecycle::Active,
    );
    not_runnable.contract.execution.constraints.network_access = NetworkAccess::Required;
    let runtime = Runtime::new(registry_with(vec![not_runnable]), EchoExecutor);

    let outcome = runtime.execute(base_request_exact());

    assert_eq!(
        outcome.result.error.as_ref().map(|error| error.code),
        Some(RuntimeErrorCode::CapabilityNotRunnable)
    );
    assert_eq!(
        outcome.trace.selection.failure_reason,
        Some(SelectionFailureReason::NotRunnable)
    );
    assert_eq!(
        outcome.trace.candidate_collection.rejected_candidates.len(),
        1
    );
}

#[test]
fn rejects_non_runtime_lifecycle_candidates() {
    let runtime = Runtime::new(
        registry_with(vec![registration(
            RegistryScope::Private,
            "content.comments.create-comment-draft",
            "1.0.0",
            Lifecycle::Archived,
        )]),
        EchoExecutor,
    );

    let outcome = runtime.execute(base_request_exact());

    assert_eq!(
        outcome.result.error.as_ref().map(|error| error.code),
        Some(RuntimeErrorCode::CapabilityNotRunnable)
    );
    assert_eq!(
        outcome.trace.candidate_collection.rejected_candidates[0].reason,
        traverse_runtime::RejectedCandidateReason::LifecycleNotRunnable
    );
}

#[test]
fn rejects_invalid_input_against_contract() {
    let runtime = Runtime::new(
        registry_with(vec![registration(
            RegistryScope::Private,
            "content.comments.create-comment-draft",
            "1.0.0",
            Lifecycle::Active,
        )]),
        EchoExecutor,
    );
    let mut request = base_request_exact();
    request.input = json!({"resource_id": "res-1"});

    let outcome = runtime.execute(request);

    assert_eq!(
        outcome.result.error.as_ref().map(|error| error.code),
        Some(RuntimeErrorCode::RequestInvalid)
    );
    assert_eq!(
        outcome.trace.execution.failure_reason,
        Some(ExecutionFailureReason::ContractInputInvalid)
    );
}

#[test]
fn surfaces_executor_failures() {
    let runtime = Runtime::new(
        registry_with(vec![registration(
            RegistryScope::Private,
            "content.comments.create-comment-draft",
            "1.0.0",
            Lifecycle::Active,
        )]),
        FailingExecutor,
    );

    let outcome = runtime.execute(base_request_exact());

    assert_eq!(
        outcome.result.error.as_ref().map(|error| error.code),
        Some(RuntimeErrorCode::ExecutionFailed)
    );
    assert_eq!(outcome.trace.execution.status, ExecutionStatus::Failed);
    assert_eq!(
        outcome.trace.execution.failure_reason,
        Some(ExecutionFailureReason::ExecutionFailed)
    );
}

#[test]
fn rejects_invalid_executor_output_against_contract() {
    let runtime = Runtime::new(
        registry_with(vec![registration(
            RegistryScope::Private,
            "content.comments.create-comment-draft",
            "1.0.0",
            Lifecycle::Active,
        )]),
        WrongOutputExecutor,
    );

    let outcome = runtime.execute(base_request_exact());

    assert_eq!(
        outcome.result.error.as_ref().map(|error| error.code),
        Some(RuntimeErrorCode::OutputValidationFailed)
    );
    assert_eq!(
        outcome.trace.execution.failure_reason,
        Some(ExecutionFailureReason::ContractOutputInvalid)
    );
}

#[test]
fn records_local_placement_decision_for_successful_execution() {
    let runtime = Runtime::new(
        registry_with(vec![registration(
            RegistryScope::Private,
            "content.comments.create-comment-draft",
            "1.0.0",
            Lifecycle::Active,
        )]),
        EchoExecutor,
    );

    let outcome = runtime.execute(base_request_exact());

    assert_eq!(outcome.result.status, RuntimeResultStatus::Completed);
    assert_eq!(
        outcome.trace.execution.placement.requested_target,
        PlacementTarget::Local
    );
    assert_eq!(
        outcome.trace.execution.placement.selected_target,
        Some(PlacementTarget::Local)
    );
    assert_eq!(
        outcome.trace.execution.placement.status,
        traverse_runtime::PlacementDecisionStatus::Selected
    );
    assert_eq!(
        outcome.trace.execution.placement.reason,
        traverse_runtime::PlacementDecisionReason::RequestedTargetSelected
    );
    assert_eq!(
        outcome.trace.execution.placement.supported_executor_targets,
        vec![PlacementTarget::Local]
    );
}

#[test]
fn rejects_unsupported_non_local_placement_requests() {
    let runtime = Runtime::new(
        registry_with(vec![registration(
            RegistryScope::Private,
            "content.comments.create-comment-draft",
            "1.0.0",
            Lifecycle::Active,
        )]),
        EchoExecutor,
    );
    let mut request = base_request_exact();
    request.context.requested_target = PlacementTarget::Cloud;

    let outcome = runtime.execute(request);

    assert_eq!(outcome.result.status, RuntimeResultStatus::Error);
    assert_eq!(
        outcome.result.error.as_ref().map(|error| error.code),
        Some(RuntimeErrorCode::PlacementUnsupported)
    );
    assert_eq!(
        outcome.trace.execution.failure_reason,
        Some(ExecutionFailureReason::PlacementUnsupported)
    );
    assert_eq!(
        outcome.trace.execution.placement.status,
        traverse_runtime::PlacementDecisionStatus::NotAttempted
    );
    assert_eq!(
        outcome.trace.execution.placement.reason,
        traverse_runtime::PlacementDecisionReason::RequestedTargetUnsupported
    );
    assert_eq!(
        outcome.trace.execution.placement.supported_executor_targets,
        vec![PlacementTarget::Local]
    );
}

#[test]
fn uses_public_only_scope_when_requested() {
    let runtime = Runtime::new(
        registry_with(vec![
            registration(
                RegistryScope::Public,
                "content.comments.create-comment-draft",
                "1.0.0",
                Lifecycle::Active,
            ),
            registration(
                RegistryScope::Private,
                "content.comments.create-comment-draft",
                "1.0.0",
                Lifecycle::Active,
            ),
        ]),
        EchoExecutor,
    );
    let mut request = base_request_exact();
    request.lookup.scope = RuntimeLookupScope::PublicOnly;

    let outcome = runtime.execute(request);

    assert_eq!(
        outcome.trace.candidate_collection.lookup_scope,
        RuntimeLookupScope::PublicOnly
    );
    assert_eq!(
        outcome.trace.candidate_collection.candidates[0].scope,
        traverse_runtime::RuntimeRegistryScope::Public
    );
}

fn states(events: &[traverse_runtime::RuntimeStateEvent]) -> Vec<RuntimeState> {
    events.iter().map(|event| event.state).collect()
}

fn base_request() -> Value {
    json!({
        "kind": "runtime_request",
        "schema_version": "1.0.0",
        "request_id": "req-123",
        "intent": {
            "capability_id": "content.comments.create-comment-draft",
            "capability_version": "1.0.0",
            "intent_key": "content.comments.create-comment-draft"
        },
        "input": {
            "comment_text": "Hello",
            "resource_id": "res-1"
        },
        "lookup": {
            "scope": "prefer_private",
            "allow_ambiguity": false
        },
        "context": {
            "requested_target": "local",
            "correlation_id": "corr-1",
            "caller": "cli"
        },
        "governing_spec": "006-runtime-request-execution"
    })
}

fn base_request_exact() -> RuntimeRequest {
    RuntimeRequest {
        kind: "runtime_request".to_string(),
        schema_version: "1.0.0".to_string(),
        request_id: "req-123".to_string(),
        intent: traverse_runtime::RuntimeIntent {
            capability_id: Some("content.comments.create-comment-draft".to_string()),
            capability_version: Some("1.0.0".to_string()),
            intent_key: Some("content.comments.create-comment-draft".to_string()),
        },
        input: json!({
            "comment_text": "Hello",
            "resource_id": "res-1"
        }),
        lookup: RuntimeLookup {
            scope: RuntimeLookupScope::PreferPrivate,
            allow_ambiguity: false,
        },
        context: RuntimeContext {
            requested_target: PlacementTarget::Local,
            correlation_id: Some("corr-1".to_string()),
            caller: Some("cli".to_string()),
            metadata: None,
        },
        governing_spec: "006-runtime-request-execution".to_string(),
    }
}

fn registry_with(registrations: Vec<CapabilityRegistration>) -> CapabilityRegistry {
    let mut registry = CapabilityRegistry::new();
    for registration in registrations {
        let outcome = registry.register(registration);
        assert!(outcome.is_ok());
    }
    registry
}

fn registration(
    scope: RegistryScope,
    id: &str,
    version: &str,
    lifecycle: Lifecycle,
) -> CapabilityRegistration {
    CapabilityRegistration {
        scope,
        contract: capability_contract(id, version, lifecycle),
        contract_path: format!("registry/{id}/{version}/contract.json"),
        artifact: artifact_record(id, version),
        registered_at: "2026-03-27T00:00:00Z".to_string(),
        tags: vec!["comments".to_string()],
        composability: ComposabilityMetadata {
            kind: CompositionKind::Atomic,
            patterns: vec![CompositionPattern::Sequential],
            provides: vec!["draft".to_string()],
            requires: vec!["authenticated-user".to_string()],
        },
        governing_spec: "005-capability-registry".to_string(),
        validator_version: "0.1.0".to_string(),
    }
}

fn capability_contract(
    id: &str,
    version: &str,
    lifecycle: Lifecycle,
) -> traverse_contracts::CapabilityContract {
    traverse_contracts::CapabilityContract {
        kind: "capability_contract".to_string(),
        schema_version: "1.0.0".to_string(),
        id: id.to_string(),
        namespace: "content.comments".to_string(),
        name: "create-comment-draft".to_string(),
        version: version.to_string(),
        lifecycle,
        owner: Owner {
            team: "comments".to_string(),
            contact: "comments@example.com".to_string(),
        },
        summary: "Create a comment draft for a resource".to_string(),
        description: "Creates a draft comment and returns the generated draft identifier."
            .to_string(),
        inputs: SchemaContainer {
            schema: json!({
                "type": "object",
                "required": ["comment_text", "resource_id"],
                "properties": {
                    "comment_text": {"type": "string"},
                    "resource_id": {"type": "string"}
                }
            }),
        },
        outputs: SchemaContainer {
            schema: json!({
                "type": "object",
                "required": ["draft_id"],
                "properties": {
                    "draft_id": {"type": "string"}
                }
            }),
        },
        preconditions: vec![Condition {
            id: "user_authenticated".to_string(),
            description: "The caller is authenticated.".to_string(),
        }],
        postconditions: vec![Condition {
            id: "draft_created".to_string(),
            description: "A draft identifier is produced.".to_string(),
        }],
        side_effects: vec![SideEffect {
            kind: SideEffectKind::MemoryOnly,
            description: "Produces a draft representation in memory.".to_string(),
        }],
        emits: vec![EventReference {
            event_id: "content.comments.draft-created".to_string(),
            version: "1.0.0".to_string(),
        }],
        consumes: Vec::new(),
        permissions: vec![IdReference {
            id: "comments.create".to_string(),
        }],
        execution: Execution {
            binary_format: ContractBinaryFormat::Wasm,
            entrypoint: Entrypoint {
                kind: EntrypointKind::WasiCommand,
                command: "run".to_string(),
            },
            preferred_targets: vec![ExecutionTarget::Local],
            constraints: ExecutionConstraints {
                host_api_access: HostApiAccess::None,
                network_access: NetworkAccess::Forbidden,
                filesystem_access: FilesystemAccess::None,
            },
        },
        policies: vec![IdReference {
            id: "policy.comments.default".to_string(),
        }],
        dependencies: vec![DependencyReference {
            artifact_type: traverse_contracts::DependencyArtifactType::Event,
            id: "content.comments.draft-created".to_string(),
            version: "1.0.0".to_string(),
        }],
        provenance: Provenance {
            source: ProvenanceSource::Greenfield,
            author: "Enrico Piovesan".to_string(),
            created_at: "2026-03-27T00:00:00Z".to_string(),
            spec_ref: Some("006-runtime-request-execution".to_string()),
            adr_refs: Vec::new(),
            exception_refs: Vec::new(),
        },
        evidence: Vec::new(),
    }
}

fn artifact_record(id: &str, version: &str) -> CapabilityArtifactRecord {
    CapabilityArtifactRecord {
        artifact_ref: format!("artifact:{id}:{version}"),
        implementation_kind: ImplementationKind::Executable,
        source: SourceReference {
            kind: SourceKind::Git,
            location: "https://github.com/enricopiovesan/cogolo".to_string(),
        },
        binary: Some(BinaryReference {
            format: BinaryFormat::Wasm,
            location: format!("artifacts/{id}/{version}/capability.wasm"),
        }),
        workflow_ref: None,
        digests: ArtifactDigests {
            source_digest: format!("src-{version}"),
            binary_digest: Some(format!("bin-{version}")),
        },
        provenance: RegistryProvenance {
            source: "test".to_string(),
            author: "Enrico Piovesan".to_string(),
            created_at: "2026-03-27T00:00:00Z".to_string(),
        },
    }
}

struct EchoExecutor;

impl LocalExecutor for EchoExecutor {
    fn execute(
        &self,
        _capability: &traverse_registry::ResolvedCapability,
        _input: &Value,
    ) -> Result<Value, LocalExecutionFailure> {
        Ok(json!({"draft_id": "draft-001"}))
    }
}

struct FailingExecutor;

impl LocalExecutor for FailingExecutor {
    fn execute(
        &self,
        _capability: &traverse_registry::ResolvedCapability,
        _input: &Value,
    ) -> Result<Value, LocalExecutionFailure> {
        Err(LocalExecutionFailure {
            code: LocalExecutionFailureCode::ExecutionFailed,
            message: "executor failed".to_string(),
        })
    }
}

struct WrongOutputExecutor;

impl LocalExecutor for WrongOutputExecutor {
    fn execute(
        &self,
        _capability: &traverse_registry::ResolvedCapability,
        _input: &Value,
    ) -> Result<Value, LocalExecutionFailure> {
        Ok(json!({"missing": "draft_id"}))
    }
}

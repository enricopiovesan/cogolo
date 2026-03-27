#![allow(clippy::expect_used)]

use serde_json::json;
use traverse_contracts::{
    BinaryFormat as ContractBinaryFormat, CapabilityContract, Condition, DependencyArtifactType,
    DependencyReference, Entrypoint, EntrypointKind, EventReference, EvidenceStatus, EvidenceType,
    Execution, ExecutionConstraints, ExecutionTarget, FilesystemAccess, HostApiAccess, IdReference,
    Lifecycle, NetworkAccess, Owner, Provenance, ProvenanceSource, SchemaContainer, SideEffect,
    SideEffectKind, ValidationEvidence,
};
use traverse_registry::{
    ArtifactDigests, BinaryFormat, BinaryReference, CapabilityArtifactRecord,
    CapabilityRegistration, CapabilityRegistry, ComposabilityMetadata, CompositionKind,
    CompositionPattern, DiscoveryQuery, ImplementationKind, LookupScope, RegistryErrorCode,
    RegistryProvenance, RegistryScope, SourceKind, SourceReference, WorkflowReference,
};

#[test]
fn registers_and_finds_public_executable_capability() {
    let mut registry = CapabilityRegistry::new();
    let request = executable_registration(
        RegistryScope::Public,
        base_contract("content.comments.create-comment-draft", "1.0.0"),
    );

    let outcome = registry
        .register(request)
        .expect("registration should pass");
    let resolved = registry
        .find_exact(
            LookupScope::PublicOnly,
            "content.comments.create-comment-draft",
            "1.0.0",
        )
        .expect("capability should resolve");

    assert_eq!(resolved.record, outcome.record);
    assert_eq!(resolved.artifact, outcome.artifact);
    assert_eq!(resolved.index_entry, outcome.index_entry);
    assert_eq!(resolved.record.scope, RegistryScope::Public);
}

#[test]
fn duplicate_identical_registration_is_idempotent() {
    let mut registry = CapabilityRegistry::new();
    let request = executable_registration(
        RegistryScope::Public,
        base_contract("content.comments.create-comment-draft", "1.0.0"),
    );

    let first = registry
        .register(request.clone())
        .expect("first registration should pass");
    let second = registry
        .register(request)
        .expect("duplicate identical registration should be a no-op");

    assert_eq!(first.record, second.record);
    assert_eq!(first.index_entry, second.index_entry);
}

#[test]
fn rejects_immutable_version_conflict_for_changed_contract() {
    let mut registry = CapabilityRegistry::new();
    registry
        .register(executable_registration(
            RegistryScope::Public,
            base_contract("content.comments.create-comment-draft", "1.0.0"),
        ))
        .expect("seed registration should pass");

    let mut changed = base_contract("content.comments.create-comment-draft", "1.0.0");
    changed.summary = "Create a materially different comment draft result.".to_string();

    let failure = registry
        .register(executable_registration(RegistryScope::Public, changed))
        .expect_err("republishing same version with changed content must fail");

    assert_eq!(
        failure.errors[0].code,
        RegistryErrorCode::ContractValidationFailed
    );
}

#[test]
fn private_overlay_takes_precedence_over_public() {
    let mut registry = CapabilityRegistry::new();
    let public = executable_registration(
        RegistryScope::Public,
        base_contract("content.comments.create-comment-draft", "1.0.0"),
    );
    let mut private_contract = base_contract("content.comments.create-comment-draft", "1.0.0");
    private_contract.summary = "Create a private overlay comment draft variant.".to_string();
    let private = executable_registration(RegistryScope::Private, private_contract);

    registry
        .register(public)
        .expect("public registration should pass");
    registry
        .register(private)
        .expect("private registration should pass");

    let resolved = registry
        .find_exact(
            LookupScope::PreferPrivate,
            "content.comments.create-comment-draft",
            "1.0.0",
        )
        .expect("lookup should resolve");

    assert_eq!(resolved.record.scope, RegistryScope::Private);
    assert_eq!(
        resolved.record.contract_path,
        "registry/private/content.comments.create-comment-draft/1.0.0/contract.json"
    );
}

#[test]
fn discover_filters_and_orders_results_deterministically() {
    let mut registry = CapabilityRegistry::new();

    let mut older = executable_registration(
        RegistryScope::Public,
        base_contract("content.comments.create-comment-draft", "1.0.0"),
    );
    older.tags = vec!["comments".to_string(), "draft".to_string()];

    let mut newer = executable_registration(
        RegistryScope::Public,
        additive_contract("content.comments.create-comment-draft", "1.1.0"),
    );
    newer.tags = vec!["comments".to_string(), "draft".to_string()];

    registry
        .register(older)
        .expect("older registration should pass");
    registry
        .register(newer)
        .expect("newer registration should pass");

    let results = registry.discover(
        LookupScope::PreferPrivate,
        &DiscoveryQuery {
            tag: Some("draft".to_string()),
            composition_pattern: Some(CompositionPattern::Sequential),
            ..DiscoveryQuery::default()
        },
    );

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].version, "1.1.0");
    assert_eq!(results[1].version, "1.0.0");
}

#[test]
fn additive_changes_require_at_least_minor_version() {
    let mut registry = CapabilityRegistry::new();
    registry
        .register(executable_registration(
            RegistryScope::Public,
            base_contract("content.comments.create-comment-draft", "1.0.0"),
        ))
        .expect("seed registration should pass");

    let failure = registry
        .register(executable_registration(
            RegistryScope::Public,
            additive_contract("content.comments.create-comment-draft", "1.0.1"),
        ))
        .expect_err("patch bump should be too small for additive change");

    assert_eq!(failure.errors[0].code, RegistryErrorCode::SemverTooSmall);

    registry
        .register(executable_registration(
            RegistryScope::Public,
            additive_contract("content.comments.create-comment-draft", "1.1.0"),
        ))
        .expect("minor bump should pass for additive change");

    assert_eq!(registry.compatibility_records().len(), 1);
}

#[test]
fn breaking_changes_require_major_version() {
    let mut registry = CapabilityRegistry::new();
    registry
        .register(executable_registration(
            RegistryScope::Public,
            base_contract("content.comments.create-comment-draft", "1.0.0"),
        ))
        .expect("seed registration should pass");

    let failure = registry
        .register(executable_registration(
            RegistryScope::Public,
            breaking_contract("content.comments.create-comment-draft", "1.1.0"),
        ))
        .expect_err("minor bump should be too small for breaking change");

    assert_eq!(failure.errors[0].code, RegistryErrorCode::SemverTooSmall);

    registry
        .register(executable_registration(
            RegistryScope::Public,
            breaking_contract("content.comments.create-comment-draft", "2.0.0"),
        ))
        .expect("major bump should pass for breaking change");
}

#[test]
fn unknown_schema_changes_fail_closed() {
    let mut registry = CapabilityRegistry::new();
    registry
        .register(executable_registration(
            RegistryScope::Public,
            base_contract("content.comments.create-comment-draft", "1.0.0"),
        ))
        .expect("seed registration should pass");

    let failure = registry
        .register(executable_registration(
            RegistryScope::Public,
            schema_changed_contract("content.comments.create-comment-draft", "1.1.0"),
        ))
        .expect_err("unknown compatibility should fail closed");

    assert_eq!(
        failure.errors[0].code,
        RegistryErrorCode::UnknownCompatibility
    );
}

#[test]
fn workflow_backed_capabilities_require_composite_metadata() {
    let mut registry = CapabilityRegistry::new();
    let workflow_request = workflow_registration(
        RegistryScope::Private,
        base_contract("content.comments.publish-comment", "1.0.0"),
    );

    let outcome = registry
        .register(workflow_request)
        .expect("workflow-backed capability should register");

    assert_eq!(
        outcome.record.implementation_kind,
        ImplementationKind::Workflow
    );
    assert_eq!(
        outcome.index_entry.composability.kind,
        CompositionKind::Composite
    );
}

#[test]
fn rejects_invalid_registration_metadata() {
    let mut registry = CapabilityRegistry::new();
    let mut request = executable_registration(
        RegistryScope::Public,
        base_contract("content.comments.create-comment-draft", "1.0.0"),
    );
    request.contract_path.clear();
    request.tags.push("comments".to_string());
    request
        .composability
        .patterns
        .push(CompositionPattern::Sequential);

    let failure = registry
        .register(request)
        .expect_err("invalid registration metadata should fail");

    assert_eq!(failure.errors.len(), 3);
    assert_eq!(
        failure.errors[0].code,
        RegistryErrorCode::MissingRequiredField
    );
    assert_eq!(failure.errors[1].code, RegistryErrorCode::DuplicateItem);
    assert_eq!(failure.errors[2].code, RegistryErrorCode::DuplicateItem);
}

#[test]
fn rejects_artifact_conflicts_for_reused_artifact_refs() {
    let mut registry = CapabilityRegistry::new();
    registry
        .register(executable_registration(
            RegistryScope::Public,
            base_contract("content.comments.create-comment-draft", "1.0.0"),
        ))
        .expect("seed registration should pass");

    let mut request = executable_registration(
        RegistryScope::Public,
        additive_contract("content.comments.create-comment-draft", "1.1.0"),
    );
    request.artifact.artifact_ref = "artifact:create-comment-draft:1.0.0".to_string();
    request.artifact.digests.source_digest = "different-source-digest".to_string();

    let failure = registry
        .register(request)
        .expect_err("reusing an artifact_ref for different metadata must fail");

    assert_eq!(failure.errors[0].code, RegistryErrorCode::ArtifactConflict);
}

fn executable_registration(
    scope: RegistryScope,
    contract: CapabilityContract,
) -> CapabilityRegistration {
    CapabilityRegistration {
        scope,
        contract_path: format!(
            "registry/{}/{}{}/contract.json",
            scope_name(scope),
            "",
            contract.id.replace(':', "")
        )
        .replace(
            &format!("registry/{}/{}", scope_name(scope), contract.id),
            &format!(
                "registry/{}/{}/{}",
                scope_name(scope),
                contract.id,
                contract.version
            ),
        ),
        artifact: CapabilityArtifactRecord {
            artifact_ref: format!("artifact:{}:{}", contract.name, contract.version),
            implementation_kind: ImplementationKind::Executable,
            source: SourceReference {
                kind: SourceKind::Git,
                location: format!("https://github.com/enricopiovesan/cogolo/{}", contract.name),
            },
            binary: Some(BinaryReference {
                format: BinaryFormat::Wasm,
                location: format!("artifacts/{}/{}.wasm", contract.name, contract.version),
            }),
            workflow_ref: None,
            digests: ArtifactDigests {
                source_digest: format!("source:{}:{}", contract.name, contract.version),
                binary_digest: Some(format!("binary:{}:{}", contract.name, contract.version)),
            },
            provenance: RegistryProvenance {
                source: "greenfield".to_string(),
                author: "enricopiovesan".to_string(),
                created_at: "2026-03-27T00:00:00Z".to_string(),
            },
        },
        registered_at: "2026-03-27T00:00:00Z".to_string(),
        tags: vec!["comments".to_string()],
        composability: ComposabilityMetadata {
            kind: CompositionKind::Atomic,
            patterns: vec![CompositionPattern::Sequential],
            provides: vec!["comment-draft".to_string()],
            requires: vec!["validated-request".to_string()],
        },
        governing_spec: "005-capability-registry".to_string(),
        validator_version: "registry-test".to_string(),
        contract,
    }
}

fn workflow_registration(
    scope: RegistryScope,
    contract: CapabilityContract,
) -> CapabilityRegistration {
    CapabilityRegistration {
        composability: ComposabilityMetadata {
            kind: CompositionKind::Composite,
            patterns: vec![
                CompositionPattern::EventDriven,
                CompositionPattern::Aggregation,
            ],
            provides: vec!["published-comment".to_string()],
            requires: vec!["comment-draft".to_string()],
        },
        artifact: CapabilityArtifactRecord {
            artifact_ref: format!("artifact:{}:{}", contract.name, contract.version),
            implementation_kind: ImplementationKind::Workflow,
            source: SourceReference {
                kind: SourceKind::Local,
                location: format!("examples/workflows/{}/", contract.name),
            },
            binary: None,
            workflow_ref: Some(WorkflowReference {
                workflow_id: "content.comments.publish-comment-flow".to_string(),
                workflow_version: "1.0.0".to_string(),
            }),
            digests: ArtifactDigests {
                source_digest: format!("source:{}:{}", contract.name, contract.version),
                binary_digest: None,
            },
            provenance: RegistryProvenance {
                source: "greenfield".to_string(),
                author: "enricopiovesan".to_string(),
                created_at: "2026-03-27T00:00:00Z".to_string(),
            },
        },
        tags: vec!["workflow".to_string(), "comments".to_string()],
        ..executable_registration(scope, contract)
    }
}

fn base_contract(id: &str, version: &str) -> CapabilityContract {
    let (namespace, name) = split_id(id);
    CapabilityContract {
        kind: "capability_contract".to_string(),
        schema_version: "1.0.0".to_string(),
        id: id.to_string(),
        namespace,
        name: name.to_string(),
        version: version.to_string(),
        lifecycle: Lifecycle::Active,
        owner: Owner {
            team: "traverse-core".to_string(),
            contact: "enrico.piovesan10@gmail.com".to_string(),
        },
        summary: "Create a validated comment draft for downstream composition.".to_string(),
        description: "Portable capability for creating a validated comment draft before further workflow processing.".to_string(),
        inputs: SchemaContainer {
            schema: json!({"type": "object", "required": ["comment_text"]}),
        },
        outputs: SchemaContainer {
            schema: json!({"type": "object", "required": ["draft_id"]}),
        },
        preconditions: vec![Condition {
            id: "request-authenticated".to_string(),
            description: "Caller identity has already been established.".to_string(),
        }],
        postconditions: vec![Condition {
            id: "draft-created".to_string(),
            description: "A draft payload is produced.".to_string(),
        }],
        side_effects: vec![SideEffect {
            kind: SideEffectKind::MemoryOnly,
            description: "The capability produces in-memory draft state only.".to_string(),
        }],
        emits: vec![EventReference {
            event_id: "content.comments.comment-draft-created".to_string(),
            version: "1.0.0".to_string(),
        }],
        consumes: vec![],
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
            id: "default-comment-safety".to_string(),
        }],
        dependencies: vec![DependencyReference {
            artifact_type: DependencyArtifactType::Event,
            id: "content.comments.comment-draft-created".to_string(),
            version: "1.0.0".to_string(),
        }],
        provenance: Provenance {
            source: ProvenanceSource::Greenfield,
            author: "enricopiovesan".to_string(),
            created_at: "2026-03-27T00:00:00Z".to_string(),
            spec_ref: Some("002-capability-contracts".to_string()),
            adr_refs: vec![],
            exception_refs: vec![],
        },
        evidence: vec![ValidationEvidence {
            evidence_id: "validation:contract".to_string(),
            evidence_type: EvidenceType::ContractValidation,
            status: EvidenceStatus::Passed,
        }],
    }
}

fn additive_contract(id: &str, version: &str) -> CapabilityContract {
    let mut contract = base_contract(id, version);
    contract.emits.push(EventReference {
        event_id: "content.comments.comment-draft-indexed".to_string(),
        version: "1.0.0".to_string(),
    });
    contract
}

fn breaking_contract(id: &str, version: &str) -> CapabilityContract {
    let mut contract = base_contract(id, version);
    contract.permissions.clear();
    contract
}

fn schema_changed_contract(id: &str, version: &str) -> CapabilityContract {
    let mut contract = base_contract(id, version);
    contract.inputs = SchemaContainer {
        schema: json!({"type": "object", "required": ["comment_text", "resource_id"]}),
    };
    contract
}

fn split_id(id: &str) -> (String, &str) {
    let mut parts = id.rsplitn(2, '.');
    let name = parts.next().expect("id must include a name");
    let namespace = parts.next().expect("id must include a namespace");
    (namespace.to_string(), name)
}

fn scope_name(scope: RegistryScope) -> &'static str {
    match scope {
        RegistryScope::Public => "public",
        RegistryScope::Private => "private",
    }
}

use serde_json::Value;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use traverse_contracts::{
    EventContract, EventValidationContext, parse_event_contract, validate_event_contract,
};
use traverse_registry::{
    ArtifactDigests, BinaryFormat, BinaryReference, CapabilityArtifactRecord,
    CapabilityRegistration, CapabilityRegistry, ComposabilityMetadata, CompositionKind,
    CompositionPattern, EventRegistration, EventRegistry, ImplementationKind, RegistryBundle,
    RegistryProvenance, SourceKind, SourceReference, WorkflowDefinition, WorkflowReference,
    WorkflowRegistration, WorkflowRegistry, load_registry_bundle,
};

#[derive(Debug)]
enum Command {
    BundleInspect { manifest_path: PathBuf },
    BundleRegister { manifest_path: PathBuf },
    Event { contract_path: PathBuf },
    Workflow { workflow_path: PathBuf },
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    match run(&args) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<String, String> {
    match parse_command(args)? {
        Command::BundleInspect { manifest_path } => inspect_bundle(&manifest_path),
        Command::BundleRegister { manifest_path } => register_bundle(&manifest_path),
        Command::Event { contract_path } => inspect_event(&contract_path),
        Command::Workflow { workflow_path } => inspect_workflow(&workflow_path),
    }
}

fn parse_command(args: &[String]) -> Result<Command, String> {
    if args.len() != 4 {
        return Err(usage());
    }

    match (args[1].as_str(), args[2].as_str()) {
        ("bundle", "inspect") => Ok(Command::BundleInspect {
            manifest_path: PathBuf::from(&args[3]),
        }),
        ("bundle", "register") => Ok(Command::BundleRegister {
            manifest_path: PathBuf::from(&args[3]),
        }),
        ("event", "inspect") => Ok(Command::Event {
            contract_path: PathBuf::from(&args[3]),
        }),
        ("workflow", "inspect") => Ok(Command::Workflow {
            workflow_path: PathBuf::from(&args[3]),
        }),
        _ => Err(usage()),
    }
}

fn inspect_bundle(manifest_path: &Path) -> Result<String, String> {
    let bundle =
        load_registry_bundle(manifest_path).map_err(|failure| failure.errors[0].message.clone())?;
    Ok(render_bundle_summary(&bundle))
}

fn register_bundle(manifest_path: &Path) -> Result<String, String> {
    let bundle =
        load_registry_bundle(manifest_path).map_err(|failure| failure.errors[0].message.clone())?;

    let mut capability_registry = CapabilityRegistry::new();
    let mut event_registry = EventRegistry::new();
    let mut workflow_registry = WorkflowRegistry::new();

    let mut capability_records = Vec::new();
    let mut event_records = Vec::new();
    let mut workflow_records = Vec::new();

    for capability in &bundle.capabilities {
        let request = build_capability_registration(&bundle, capability)?;
        let outcome = capability_registry
            .register(request)
            .map_err(render_registry_failure)?;
        capability_records.push(format_capability_record(
            &outcome.record.id,
            &outcome.record.version,
            outcome.record.implementation_kind,
        ));
    }

    for event in &bundle.events {
        let outcome = event_registry
            .register(EventRegistration {
                scope: bundle.scope,
                contract: event.contract.clone(),
                contract_path: event.path.display().to_string(),
                registered_at: bundle_registered_at(&bundle),
                governing_spec: "011-event-registry".to_string(),
                validator_version: env!("CARGO_PKG_VERSION").to_string(),
            })
            .map_err(render_event_registry_failure)?;
        event_records.push(format!("{}@{}", outcome.record.id, outcome.record.version));
    }

    for workflow in &bundle.workflows {
        let outcome = workflow_registry
            .register(
                &capability_registry,
                WorkflowRegistration {
                    scope: bundle.scope,
                    definition: workflow.definition.clone(),
                    workflow_path: workflow.path.display().to_string(),
                    registered_at: bundle_registered_at(&bundle),
                    validator_version: env!("CARGO_PKG_VERSION").to_string(),
                },
            )
            .map_err(render_workflow_failure)?;
        workflow_records.push(format!("{}@{}", outcome.record.id, outcome.record.version));
    }

    Ok(render_bundle_registration_summary(
        &bundle,
        &capability_records,
        &event_records,
        &workflow_records,
    ))
}

fn inspect_event(contract_path: &Path) -> Result<String, String> {
    let contents = read_text_file(contract_path, "event contract")?;
    let parsed = parse_event_contract(&contents)
        .map_err(|failure| render_validation_failure("event contract", contract_path, failure))?;
    let validated = validate_event_contract(
        parsed,
        &EventValidationContext {
            governing_spec: "003-event-contracts",
            validator_version: env!("CARGO_PKG_VERSION"),
            existing_published: None,
        },
    )
    .map_err(|failure| render_validation_failure("event contract", contract_path, failure))?;

    Ok(render_event_summary(contract_path, &validated.normalized))
}

fn inspect_workflow(workflow_path: &Path) -> Result<String, String> {
    let contents = read_text_file(workflow_path, "workflow artifact")?;
    let definition = serde_json::from_str::<WorkflowDefinition>(&contents).map_err(|error| {
        format!(
            "failed to parse workflow artifact {}: {error}",
            workflow_path.display()
        )
    })?;

    Ok(render_workflow_summary(workflow_path, &definition))
}

fn read_text_file(path: &Path, artifact_kind: &str) -> Result<String, String> {
    fs::read_to_string(path)
        .map_err(|error| format!("failed to read {artifact_kind} {}: {error}", path.display()))
}

fn render_validation_failure(
    artifact_kind: &str,
    path: &Path,
    failure: traverse_contracts::ValidationFailure,
) -> String {
    let details = failure
        .errors
        .into_iter()
        .map(|error| format!("{} at {}", error.message, error.path))
        .collect::<Vec<_>>()
        .join("; ");

    format!(
        "failed to validate {artifact_kind} {}: {details}",
        path.display()
    )
}

fn render_bundle_summary(bundle: &RegistryBundle) -> String {
    let mut lines = vec![
        format!("bundle_id: {}", bundle.bundle_id),
        format!("version: {}", bundle.version),
        format!("scope: {:?}", bundle.scope).to_lowercase(),
        format!("capabilities: {}", bundle.capabilities.len()),
        format!("events: {}", bundle.events.len()),
        format!("workflows: {}", bundle.workflows.len()),
        "capability_ids:".to_string(),
    ];

    for capability in &bundle.capabilities {
        lines.push(format!(
            "  - {}@{}",
            capability.manifest.id, capability.manifest.version
        ));
    }

    lines.push("event_ids:".to_string());
    for event in &bundle.events {
        lines.push(format!(
            "  - {}@{}",
            event.manifest.id, event.manifest.version
        ));
    }

    lines.push("workflow_ids:".to_string());
    for workflow in &bundle.workflows {
        lines.push(format!(
            "  - {}@{}",
            workflow.manifest.id, workflow.manifest.version
        ));
    }

    lines.join("\n")
}

fn render_bundle_registration_summary(
    bundle: &RegistryBundle,
    capability_records: &[String],
    event_records: &[String],
    workflow_records: &[String],
) -> String {
    let mut lines = vec![
        format!("bundle_id: {}", bundle.bundle_id),
        format!("version: {}", bundle.version),
        format!("scope: {:?}", bundle.scope).to_lowercase(),
        format!("registered_capabilities: {}", capability_records.len()),
        format!("registered_events: {}", event_records.len()),
        format!("registered_workflows: {}", workflow_records.len()),
        "capability_records:".to_string(),
    ];

    for record in capability_records {
        lines.push(format!("  - {record}"));
    }

    lines.push("event_records:".to_string());
    for record in event_records {
        lines.push(format!("  - {record}"));
    }

    lines.push("workflow_records:".to_string());
    for record in workflow_records {
        lines.push(format!("  - {record}"));
    }

    lines.join("\n")
}

fn render_event_summary(path: &Path, contract: &EventContract) -> String {
    let mut lines = vec![
        format!("path: {}", path.display()),
        format!("id: {}", contract.id),
        format!("version: {}", contract.version),
        format!("lifecycle: {:?}", contract.lifecycle).to_lowercase(),
        format!("event_type: {:?}", contract.classification.event_type).to_lowercase(),
        format!("domain: {}", contract.classification.domain),
        format!(
            "bounded_context: {}",
            contract.classification.bounded_context
        ),
        format!("publishers: {}", contract.publishers.len()),
        format!("subscribers: {}", contract.subscribers.len()),
        format!("tags: {}", contract.tags.join(",")),
        "publisher_ids:".to_string(),
    ];

    for publisher in &contract.publishers {
        lines.push(format!(
            "  - {}@{}",
            publisher.capability_id, publisher.version
        ));
    }

    lines.push("subscriber_ids:".to_string());
    for subscriber in &contract.subscribers {
        lines.push(format!(
            "  - {}@{}",
            subscriber.capability_id, subscriber.version
        ));
    }

    lines.join("\n")
}

fn render_workflow_summary(path: &Path, definition: &WorkflowDefinition) -> String {
    let mut lines = vec![
        format!("path: {}", path.display()),
        format!("id: {}", definition.id),
        format!("version: {}", definition.version),
        format!("lifecycle: {:?}", definition.lifecycle).to_lowercase(),
        format!("start_node: {}", definition.start_node),
        format!("terminal_nodes: {}", definition.terminal_nodes.join(",")),
        format!("node_count: {}", definition.nodes.len()),
        format!("edge_count: {}", definition.edges.len()),
        format!("governing_spec: {}", definition.governing_spec),
        "node_capabilities:".to_string(),
    ];

    for node in &definition.nodes {
        lines.push(format!(
            "  - {} -> {}@{}",
            node.node_id, node.capability_id, node.capability_version
        ));
    }

    lines.push("edges:".to_string());
    for edge in &definition.edges {
        lines.push(format!(
            "  - {}: {} -> {}",
            edge.edge_id, edge.from, edge.to
        ));
    }

    lines.join("\n")
}

fn usage() -> String {
    "usage: traverse-cli <bundle|event|workflow> <inspect|register> <artifact-path>".to_string()
}

fn build_capability_registration(
    bundle: &RegistryBundle,
    capability: &traverse_registry::CapabilityBundleArtifact,
) -> Result<CapabilityRegistration, String> {
    let raw_contract = read_text_file(&capability.path, "capability contract")?;
    let envelope =
        parse_capability_registration_envelope(&raw_contract, capability.path.as_path())?;
    let implementation_kind = derive_implementation_kind(envelope.get("composability"));
    let workflow_ref = derive_workflow_ref(envelope.get("composability"))?;
    let composability =
        derive_composability_metadata(implementation_kind, workflow_ref.as_ref(), capability)?;
    let artifact = build_capability_artifact(bundle, capability, implementation_kind, workflow_ref);

    Ok(CapabilityRegistration {
        scope: bundle.scope,
        contract: capability.contract.clone(),
        contract_path: capability.path.display().to_string(),
        artifact,
        registered_at: bundle_registered_at(bundle),
        tags: Vec::new(),
        composability,
        governing_spec: "005-capability-registry".to_string(),
        validator_version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

fn parse_capability_registration_envelope(
    raw_contract: &str,
    path: &Path,
) -> Result<Value, String> {
    serde_json::from_str::<Value>(raw_contract).map_err(|error| {
        format!(
            "failed to parse capability registration metadata {}: {error}",
            path.display()
        )
    })
}

fn derive_implementation_kind(composability_value: Option<&Value>) -> ImplementationKind {
    match composability_value
        .and_then(|composability| composability.get("implementation_kind"))
        .and_then(Value::as_str)
    {
        Some("workflow") => ImplementationKind::Workflow,
        _ => ImplementationKind::Executable,
    }
}

fn derive_workflow_ref(
    composability_value: Option<&Value>,
) -> Result<Option<WorkflowReference>, String> {
    composability_value
        .and_then(|composability| composability.get("workflow_ref"))
        .map(parse_workflow_ref)
        .transpose()
}

fn derive_composability_metadata(
    implementation_kind: ImplementationKind,
    workflow_ref: Option<&WorkflowReference>,
    capability: &traverse_registry::CapabilityBundleArtifact,
) -> Result<ComposabilityMetadata, String> {
    let requires = capability
        .contract
        .consumes
        .iter()
        .map(|event| event.event_id.clone())
        .collect();

    match implementation_kind {
        ImplementationKind::Workflow => {
            if workflow_ref.is_none() {
                return Err(format!(
                    "workflow-backed capability {} must declare workflow_ref",
                    capability.contract.id
                ));
            }
            Ok(ComposabilityMetadata {
                kind: CompositionKind::Composite,
                patterns: vec![CompositionPattern::Sequential],
                provides: vec![capability.contract.id.clone()],
                requires,
            })
        }
        ImplementationKind::Executable => Ok(ComposabilityMetadata {
            kind: CompositionKind::Atomic,
            patterns: vec![CompositionPattern::Sequential],
            provides: vec![capability.contract.id.clone()],
            requires,
        }),
    }
}

fn build_capability_artifact(
    bundle: &RegistryBundle,
    capability: &traverse_registry::CapabilityBundleArtifact,
    implementation_kind: ImplementationKind,
    workflow_ref: Option<WorkflowReference>,
) -> CapabilityArtifactRecord {
    CapabilityArtifactRecord {
        artifact_ref: format!(
            "bundle:{}:{}:{}",
            bundle.bundle_id, capability.contract.id, capability.contract.version
        ),
        implementation_kind,
        source: SourceReference {
            kind: SourceKind::Local,
            location: capability.path.display().to_string(),
        },
        binary: match implementation_kind {
            ImplementationKind::Executable => Some(BinaryReference {
                format: BinaryFormat::Wasm,
                location: format!(
                    "bundled://{}/{}/module.wasm",
                    capability.contract.id, capability.contract.version
                ),
            }),
            ImplementationKind::Workflow => None,
        },
        workflow_ref,
        digests: ArtifactDigests {
            source_digest: format!(
                "source:{}:{}",
                capability.contract.id, capability.contract.version
            ),
            binary_digest: match implementation_kind {
                ImplementationKind::Executable => Some(format!(
                    "binary:{}:{}",
                    capability.contract.id, capability.contract.version
                )),
                ImplementationKind::Workflow => None,
            },
        },
        provenance: RegistryProvenance {
            source: provenance_source_label(&capability.contract.provenance.source),
            author: capability.contract.provenance.author.clone(),
            created_at: capability.contract.provenance.created_at.clone(),
        },
    }
}

fn bundle_registered_at(bundle: &RegistryBundle) -> String {
    format!("bundle:{}@{}", bundle.bundle_id, bundle.version)
}

fn parse_workflow_ref(value: &Value) -> Result<WorkflowReference, String> {
    let workflow_id = value
        .get("workflow_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "workflow_ref.workflow_id must be a string".to_string())?;
    let workflow_version = value
        .get("workflow_version")
        .and_then(Value::as_str)
        .ok_or_else(|| "workflow_ref.workflow_version must be a string".to_string())?;
    Ok(WorkflowReference {
        workflow_id: workflow_id.to_string(),
        workflow_version: workflow_version.to_string(),
    })
}

fn provenance_source_label(source: &traverse_contracts::ProvenanceSource) -> String {
    match source {
        traverse_contracts::ProvenanceSource::Greenfield => "greenfield",
        traverse_contracts::ProvenanceSource::BrownfieldExtracted => "brownfield-extracted",
        traverse_contracts::ProvenanceSource::AiGenerated => "ai-generated",
        traverse_contracts::ProvenanceSource::AiAssisted => "ai-assisted",
    }
    .to_string()
}

fn format_capability_record(
    id: &str,
    version: &str,
    implementation_kind: ImplementationKind,
) -> String {
    let kind = match implementation_kind {
        ImplementationKind::Executable => "executable",
        ImplementationKind::Workflow => "workflow",
    };
    format!("{id}@{version} ({kind})")
}

fn render_registry_failure(failure: traverse_registry::RegistryFailure) -> String {
    failure
        .errors
        .into_iter()
        .map(|error| format!("{} at {}", error.message, error.target))
        .collect::<Vec<_>>()
        .join("; ")
}

fn render_event_registry_failure(failure: traverse_registry::EventRegistryFailure) -> String {
    failure
        .errors
        .into_iter()
        .map(|error| format!("{} at {}", error.message, error.target))
        .collect::<Vec<_>>()
        .join("; ")
}

fn render_workflow_failure(failure: traverse_registry::WorkflowFailure) -> String {
    failure
        .errors
        .into_iter()
        .map(|error| format!("{} at {}", error.message, error.path))
        .collect::<Vec<_>>()
        .join("; ")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::{inspect_bundle, inspect_event, inspect_workflow, parse_command, register_bundle};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parse_command_accepts_supported_inspect_commands() {
        let bundle = vec![
            "traverse-cli".to_string(),
            "bundle".to_string(),
            "inspect".to_string(),
            "examples/expedition/registry-bundle/manifest.json".to_string(),
        ];
        let bundle_register = vec![
            "traverse-cli".to_string(),
            "bundle".to_string(),
            "register".to_string(),
            "examples/expedition/registry-bundle/manifest.json".to_string(),
        ];
        let event = vec![
            "traverse-cli".to_string(),
            "event".to_string(),
            "inspect".to_string(),
            "contracts/examples/expedition/events/expedition-objective-captured/contract.json"
                .to_string(),
        ];
        let workflow = vec![
            "traverse-cli".to_string(),
            "workflow".to_string(),
            "inspect".to_string(),
            "workflows/examples/expedition/plan-expedition/workflow.json".to_string(),
        ];

        assert!(parse_command(&bundle).is_ok());
        assert!(parse_command(&bundle_register).is_ok());
        assert!(parse_command(&event).is_ok());
        assert!(parse_command(&workflow).is_ok());
    }

    #[test]
    fn parse_command_rejects_unknown_shape() {
        let args = vec!["traverse-cli".to_string()];
        let result = parse_command(&args);
        assert!(result.is_err());
        let error = result.err().unwrap_or_default();
        assert!(error.contains("usage: traverse-cli"));
    }

    #[test]
    fn inspect_bundle_renders_canonical_example_bundle() {
        let manifest_path = repo_root().join("examples/expedition/registry-bundle/manifest.json");

        let output = inspect_bundle(&manifest_path).expect("bundle inspect should succeed");

        assert!(output.contains("bundle_id: expedition.planning.seed-bundle"));
        assert!(output.contains("event_ids:"));
        assert!(output.contains("workflow_ids:"));
    }

    #[test]
    fn inspect_bundle_rejects_missing_artifact_paths() {
        let temp_dir = unique_temp_dir();
        let manifest_path = temp_dir.join("manifest.json");
        fs::write(
            &manifest_path,
            r#"{
  "bundle_id": "expedition.planning.seed-bundle",
  "version": "1.0.0",
  "scope": "public",
  "capabilities": [
    {
      "id": "expedition.planning.capture-expedition-objective",
      "version": "1.0.0",
      "path": "missing/capability.json"
    }
  ],
  "events": [],
  "workflows": []
}"#,
        )
        .expect("manifest should write");

        let error = inspect_bundle(&manifest_path).expect_err("missing artifact path should fail");
        assert!(error.contains("missing artifact file"));
    }

    #[test]
    fn register_bundle_registers_canonical_expedition_artifacts() {
        let manifest_path = repo_root().join("examples/expedition/registry-bundle/manifest.json");

        let output = register_bundle(&manifest_path).expect("bundle register should succeed");

        assert!(output.contains("registered_capabilities: 6"));
        assert!(output.contains("registered_events: 5"));
        assert!(output.contains("registered_workflows: 1"));
        assert!(output.contains("expedition.planning.plan-expedition@1.0.0 (workflow)"));
    }

    #[test]
    fn register_bundle_rejects_duplicate_manifest_entries() {
        let temp_dir = unique_temp_dir();
        let manifest_path = temp_dir.join("manifest.json");
        fs::write(
            &manifest_path,
            r#"{
  "bundle_id": "expedition.planning.seed-bundle",
  "version": "1.0.0",
  "scope": "public",
  "capabilities": [
    {
      "id": "expedition.planning.capture-expedition-objective",
      "version": "1.0.0",
      "path": "../../../contracts/examples/expedition/capabilities/capture-expedition-objective/contract.json"
    },
    {
      "id": "expedition.planning.capture-expedition-objective",
      "version": "1.0.0",
      "path": "../../../contracts/examples/expedition/capabilities/capture-expedition-objective/contract.json"
    }
  ],
  "events": [],
  "workflows": []
}"#,
        )
        .expect("manifest should write");

        let error =
            register_bundle(&manifest_path).expect_err("duplicate bundle entries should fail");

        assert!(error.contains("duplicate capability artifact entry"));
    }

    #[test]
    fn inspect_event_renders_canonical_event_contract() {
        let path = repo_root().join(
            "contracts/examples/expedition/events/expedition-objective-captured/contract.json",
        );

        let output = inspect_event(&path).expect("event inspect should succeed");

        assert!(output.contains("id: expedition.planning.expedition-objective-captured"));
        assert!(output.contains("event_type: domain"));
        assert!(output.contains("publisher_ids:"));
    }

    #[test]
    fn inspect_event_rejects_malformed_contract() {
        let temp_dir = unique_temp_dir();
        let path = temp_dir.join("event.json");
        fs::write(&path, "{\"kind\":\"event_contract\"}").expect("event file should write");

        let error = inspect_event(&path).expect_err("malformed event contract should fail");

        assert!(error.contains("failed to validate event contract"));
    }

    #[test]
    fn inspect_workflow_renders_canonical_workflow() {
        let path = repo_root().join("workflows/examples/expedition/plan-expedition/workflow.json");

        let output = inspect_workflow(&path).expect("workflow inspect should succeed");

        assert!(output.contains("id: expedition.planning.plan-expedition"));
        assert!(output.contains("start_node: capture_objective"));
        assert!(output.contains("node_capabilities:"));
    }

    #[test]
    fn inspect_workflow_rejects_malformed_definition() {
        let temp_dir = unique_temp_dir();
        let path = temp_dir.join("workflow.json");
        fs::write(&path, "{\"id\":true}").expect("workflow file should write");

        let error = inspect_workflow(&path).expect_err("malformed workflow should fail");

        assert!(error.contains("failed to parse workflow artifact"));
    }

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn unique_temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("traverse-cli-test-{nanos}"));
        fs::create_dir_all(&path).expect("temporary directory should create");
        path
    }
}

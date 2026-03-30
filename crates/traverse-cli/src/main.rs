use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use traverse_contracts::{
    EventContract, EventValidationContext, parse_event_contract, validate_event_contract,
};
use traverse_registry::{RegistryBundle, WorkflowDefinition, load_registry_bundle};

#[derive(Debug)]
enum Command {
    Bundle { manifest_path: PathBuf },
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
        Command::Bundle { manifest_path } => inspect_bundle(&manifest_path),
        Command::Event { contract_path } => inspect_event(&contract_path),
        Command::Workflow { workflow_path } => inspect_workflow(&workflow_path),
    }
}

fn parse_command(args: &[String]) -> Result<Command, String> {
    if args.len() != 4 {
        return Err(usage());
    }

    match (args[1].as_str(), args[2].as_str()) {
        ("bundle", "inspect") => Ok(Command::Bundle {
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
    "usage: traverse-cli <bundle|event|workflow> inspect <artifact-path>".to_string()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::{inspect_bundle, inspect_event, inspect_workflow, parse_command};
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

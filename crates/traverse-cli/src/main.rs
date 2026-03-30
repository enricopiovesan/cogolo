use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use traverse_registry::{RegistryBundle, load_registry_bundle};

#[derive(Debug)]
enum Command {
    BundleInspect { manifest_path: PathBuf },
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
    let command = parse_command(args)?;
    match command {
        Command::BundleInspect { manifest_path } => inspect_bundle(&manifest_path),
    }
}

fn parse_command(args: &[String]) -> Result<Command, String> {
    if args.len() != 4 || args[1] != "bundle" || args[2] != "inspect" {
        return Err("usage: traverse-cli bundle inspect <bundle-manifest-path>".to_string());
    }

    Ok(Command::BundleInspect {
        manifest_path: PathBuf::from(&args[3]),
    })
}

fn inspect_bundle(manifest_path: &Path) -> Result<String, String> {
    let bundle =
        load_registry_bundle(manifest_path).map_err(|failure| failure.errors[0].message.clone())?;
    Ok(render_bundle_summary(&bundle))
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

#[cfg(test)]
mod tests {
    use super::{RegistryBundle, inspect_bundle, parse_command, render_bundle_summary};
    use serde_json::json;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
    use traverse_contracts::{
        CapabilityContract, Condition, DependencyReference, Entrypoint, EntrypointKind,
        EventContract, EventPayload, EventProvenance, EventProvenanceSource, EventReference,
        EventType, Execution, ExecutionConstraints, ExecutionTarget, FilesystemAccess,
        HostApiAccess, IdReference, Lifecycle, NetworkAccess, Owner, PayloadCompatibility,
        Provenance, ProvenanceSource, SchemaContainer, SideEffect, SideEffectKind,
    };
    use traverse_registry::{
        BundleArtifactManifest, CapabilityBundleArtifact, EventBundleArtifact, RegistryScope,
        WorkflowBundleArtifact, WorkflowDefinition, WorkflowNode, WorkflowNodeInput,
        WorkflowNodeOutput,
    };

    #[test]
    fn parse_command_accepts_bundle_inspect() {
        let args = vec![
            "traverse-cli".to_string(),
            "bundle".to_string(),
            "inspect".to_string(),
            "examples/expedition/registry-bundle/manifest.json".to_string(),
        ];

        let command = parse_command(&args);
        assert!(command.is_ok());
    }

    #[test]
    fn parse_command_rejects_unknown_shape() {
        let args = vec!["traverse-cli".to_string()];
        let result = parse_command(&args);
        assert!(result.is_err());
        let error = result.err().unwrap_or_default();
        assert!(error.contains("usage: traverse-cli bundle inspect"));
    }

    #[test]
    fn render_bundle_summary_lists_governed_ids() {
        let bundle = RegistryBundle {
            bundle_id: "expedition.planning.seed-bundle".to_string(),
            version: "1.0.0".to_string(),
            scope: RegistryScope::Public,
            capabilities: vec![CapabilityBundleArtifact {
                manifest: BundleArtifactManifest {
                    id: "expedition.planning.capture-expedition-objective".to_string(),
                    version: "1.0.0".to_string(),
                    path: "contracts/examples/expedition/capabilities/capture-expedition-objective/contract.json"
                        .to_string(),
                },
                path: PathBuf::from(
                    "contracts/examples/expedition/capabilities/capture-expedition-objective/contract.json",
                ),
                contract: example_capability_contract(),
            }],
            events: vec![EventBundleArtifact {
                manifest: BundleArtifactManifest {
                    id: "expedition.planning.expedition-objective-captured".to_string(),
                    version: "1.0.0".to_string(),
                    path: "contracts/examples/expedition/events/expedition-objective-captured/contract.json"
                        .to_string(),
                },
                path: PathBuf::from(
                    "contracts/examples/expedition/events/expedition-objective-captured/contract.json",
                ),
                contract: example_event_contract(),
            }],
            workflows: vec![WorkflowBundleArtifact {
                manifest: BundleArtifactManifest {
                    id: "expedition.planning.plan-expedition".to_string(),
                    version: "1.0.0".to_string(),
                    path: "workflows/examples/expedition/plan-expedition/workflow.json"
                        .to_string(),
                },
                path: PathBuf::from("workflows/examples/expedition/plan-expedition/workflow.json"),
                definition: example_workflow_definition(),
            }],
        };

        let rendered = render_bundle_summary(&bundle);
        assert!(rendered.contains("bundle_id: expedition.planning.seed-bundle"));
        assert!(rendered.contains("expedition.planning.capture-expedition-objective@1.0.0"));
        assert!(rendered.contains("expedition.planning.plan-expedition@1.0.0"));
    }

    #[test]
    fn inspect_bundle_rejects_missing_artifact_paths() {
        let temp_dir = unique_temp_dir();
        assert!(fs::create_dir_all(&temp_dir).is_ok());

        let manifest_path = temp_dir.join("manifest.json");
        assert!(
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
            .is_ok()
        );

        let result = inspect_bundle(&manifest_path);
        assert!(result.is_err());
        let error = result.err().unwrap_or_default();
        assert!(error.contains("missing artifact file"));
    }

    fn unique_temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("traverse-cli-test-{nanos}"))
    }

    fn example_capability_contract() -> CapabilityContract {
        CapabilityContract {
            kind: "capability_contract".to_string(),
            schema_version: "1.0.0".to_string(),
            id: "expedition.planning.capture-expedition-objective".to_string(),
            namespace: "expedition.planning".to_string(),
            name: "capture-expedition-objective".to_string(),
            version: "1.0.0".to_string(),
            lifecycle: Lifecycle::Active,
            owner: Owner {
                team: "Traverse".to_string(),
                contact: "team@traverse.dev".to_string(),
            },
            summary: "Summary".to_string(),
            description: "Description".to_string(),
            inputs: SchemaContainer {
                schema: json!({"type": "object"}),
            },
            outputs: SchemaContainer {
                schema: json!({"type": "object"}),
            },
            preconditions: vec![Condition {
                id: "input-available".to_string(),
                description: "Input exists".to_string(),
            }],
            postconditions: vec![Condition {
                id: "output-created".to_string(),
                description: "Output exists".to_string(),
            }],
            side_effects: vec![SideEffect {
                kind: SideEffectKind::None,
                description: "No side effects".to_string(),
            }],
            emits: vec![EventReference {
                event_id: "expedition.planning.expedition-objective-captured".to_string(),
                version: "1.0.0".to_string(),
            }],
            consumes: vec![],
            permissions: vec![IdReference {
                id: "expedition.plan.capture-objective".to_string(),
            }],
            execution: Execution {
                binary_format: traverse_contracts::BinaryFormat::Wasm,
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
                id: "policy.expedition".to_string(),
            }],
            dependencies: vec![DependencyReference {
                artifact_type: traverse_contracts::DependencyArtifactType::Policy,
                id: "policy.expedition".to_string(),
                version: "1.0.0".to_string(),
            }],
            provenance: Provenance {
                source: ProvenanceSource::Greenfield,
                author: "Traverse".to_string(),
                created_at: "2026-03-30T00:00:00Z".to_string(),
                spec_ref: Some("009-expedition-example-artifacts".to_string()),
                adr_refs: vec![],
                exception_refs: vec![],
            },
            evidence: vec![],
        }
    }

    fn example_event_contract() -> EventContract {
        EventContract {
            kind: "event_contract".to_string(),
            schema_version: "1.0.0".to_string(),
            id: "expedition.planning.expedition-objective-captured".to_string(),
            namespace: "expedition.planning".to_string(),
            name: "expedition-objective-captured".to_string(),
            version: "1.0.0".to_string(),
            lifecycle: Lifecycle::Active,
            owner: Owner {
                team: "Traverse".to_string(),
                contact: "team@traverse.dev".to_string(),
            },
            summary: "Summary".to_string(),
            description: "Description".to_string(),
            payload: EventPayload {
                schema: json!({"type":"object"}),
                compatibility: PayloadCompatibility::BackwardCompatible,
            },
            classification: traverse_contracts::EventClassification {
                domain: "expedition".to_string(),
                bounded_context: "planning".to_string(),
                event_type: EventType::Domain,
                tags: vec!["expedition".to_string()],
            },
            publishers: vec![traverse_contracts::CapabilityReference {
                capability_id: "expedition.planning.capture-expedition-objective".to_string(),
                version: "1.0.0".to_string(),
            }],
            subscribers: vec![],
            policies: vec![IdReference {
                id: "policy.expedition".to_string(),
            }],
            tags: vec!["expedition".to_string()],
            provenance: EventProvenance {
                source: EventProvenanceSource::Greenfield,
                author: "Traverse".to_string(),
                created_at: "2026-03-30T00:00:00Z".to_string(),
            },
            evidence: vec![],
        }
    }

    fn example_workflow_definition() -> WorkflowDefinition {
        WorkflowDefinition {
            kind: "workflow_definition".to_string(),
            schema_version: "1.0.0".to_string(),
            id: "expedition.planning.plan-expedition".to_string(),
            name: "plan-expedition".to_string(),
            version: "1.0.0".to_string(),
            lifecycle: Lifecycle::Active,
            owner: Owner {
                team: "Traverse".to_string(),
                contact: "team@traverse.dev".to_string(),
            },
            summary: "Summary".to_string(),
            inputs: SchemaContainer {
                schema: json!({"type":"object"}),
            },
            outputs: SchemaContainer {
                schema: json!({"type":"object"}),
            },
            nodes: vec![WorkflowNode {
                node_id: "capture_objective".to_string(),
                capability_id: "expedition.planning.capture-expedition-objective".to_string(),
                capability_version: "1.0.0".to_string(),
                input: WorkflowNodeInput {
                    from_workflow_input: vec!["objective".to_string()],
                },
                output: WorkflowNodeOutput {
                    to_workflow_state: vec!["objective".to_string()],
                },
            }],
            edges: vec![],
            start_node: "capture_objective".to_string(),
            terminal_nodes: vec!["capture_objective".to_string()],
            tags: vec!["expedition".to_string()],
            governing_spec: "009-expedition-example-artifacts".to_string(),
        }
    }
}

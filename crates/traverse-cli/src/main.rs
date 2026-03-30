use serde::Deserialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Debug, Deserialize)]
struct RegistryBundle {
    bundle_id: String,
    version: String,
    scope: String,
    capabilities: Vec<BundleArtifact>,
    events: Vec<BundleArtifact>,
    workflows: Vec<BundleArtifact>,
}

#[derive(Debug, Deserialize)]
struct BundleArtifact {
    id: String,
    version: String,
    path: String,
}

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
    let manifest_contents = fs::read_to_string(manifest_path).map_err(|error| {
        format!(
            "failed to read bundle manifest {}: {error}",
            manifest_path.display()
        )
    })?;
    let bundle: RegistryBundle = serde_json::from_str(&manifest_contents).map_err(|error| {
        format!(
            "failed to parse bundle manifest {}: {error}",
            manifest_path.display()
        )
    })?;

    let manifest_dir = manifest_path.parent().ok_or_else(|| {
        format!(
            "bundle manifest {} has no parent directory",
            manifest_path.display()
        )
    })?;

    validate_paths(manifest_dir, &bundle.capabilities, "capability")?;
    validate_paths(manifest_dir, &bundle.events, "event")?;
    validate_paths(manifest_dir, &bundle.workflows, "workflow")?;

    Ok(render_bundle_summary(&bundle))
}

fn validate_paths(
    manifest_dir: &Path,
    artifacts: &[BundleArtifact],
    artifact_kind: &str,
) -> Result<(), String> {
    for artifact in artifacts {
        let resolved_path = manifest_dir.join(&artifact.path);
        if !resolved_path.is_file() {
            return Err(format!(
                "missing {artifact_kind} artifact file for {} at {}",
                artifact.id,
                resolved_path.display()
            ));
        }
    }

    Ok(())
}

fn render_bundle_summary(bundle: &RegistryBundle) -> String {
    let mut lines = vec![
        format!("bundle_id: {}", bundle.bundle_id),
        format!("version: {}", bundle.version),
        format!("scope: {}", bundle.scope),
        format!("capabilities: {}", bundle.capabilities.len()),
        format!("events: {}", bundle.events.len()),
        format!("workflows: {}", bundle.workflows.len()),
        "capability_ids:".to_string(),
    ];

    for capability in &bundle.capabilities {
        lines.push(format!("  - {}@{}", capability.id, capability.version));
    }

    lines.push("event_ids:".to_string());
    for event in &bundle.events {
        lines.push(format!("  - {}@{}", event.id, event.version));
    }

    lines.push("workflow_ids:".to_string());
    for workflow in &bundle.workflows {
        lines.push(format!("  - {}@{}", workflow.id, workflow.version));
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::{
        BundleArtifact, RegistryBundle, inspect_bundle, parse_command, render_bundle_summary,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

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
            scope: "public".to_string(),
            capabilities: vec![BundleArtifact {
                id: "expedition.planning.capture-expedition-objective".to_string(),
                version: "1.0.0".to_string(),
                path: "contracts/examples/expedition/capabilities/capture-expedition-objective/contract.json"
                    .to_string(),
            }],
            events: vec![BundleArtifact {
                id: "expedition.planning.expedition-objective-captured".to_string(),
                version: "1.0.0".to_string(),
                path: "contracts/examples/expedition/events/expedition-objective-captured/contract.json"
                    .to_string(),
            }],
            workflows: vec![BundleArtifact {
                id: "expedition.planning.plan-expedition".to_string(),
                version: "1.0.0".to_string(),
                path: "workflows/examples/expedition/plan-expedition/workflow.json".to_string(),
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
        assert!(error.contains("missing capability artifact file"));
    }

    fn unique_temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("traverse-cli-test-{nanos}"))
    }
}

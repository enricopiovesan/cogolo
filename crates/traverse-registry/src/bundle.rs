use crate::{RegistryScope, WorkflowDefinition};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use traverse_contracts::{
    CapabilityContract, EventContract, ValidationError, parse_contract, parse_event_contract,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryBundle {
    pub bundle_id: String,
    pub version: String,
    pub scope: RegistryScope,
    pub capabilities: Vec<CapabilityBundleArtifact>,
    pub events: Vec<EventBundleArtifact>,
    pub workflows: Vec<WorkflowBundleArtifact>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityBundleArtifact {
    pub manifest: BundleArtifactManifest,
    pub path: PathBuf,
    pub contract: CapabilityContract,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventBundleArtifact {
    pub manifest: BundleArtifactManifest,
    pub path: PathBuf,
    pub contract: EventContract,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowBundleArtifact {
    pub manifest: BundleArtifactManifest,
    pub path: PathBuf,
    pub definition: WorkflowDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleArtifactManifest {
    pub id: String,
    pub version: String,
    pub path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundleLoadErrorCode {
    ManifestParentMissing,
    ManifestReadFailed,
    ManifestParseFailed,
    InvalidScope,
    DuplicateArtifactId,
    MissingArtifactFile,
    CapabilityParseFailed,
    EventParseFailed,
    WorkflowParseFailed,
    ArtifactIdMismatch,
    ArtifactVersionMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleLoadError {
    pub code: BundleLoadErrorCode,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleLoadFailure {
    pub errors: Vec<BundleLoadError>,
}

#[derive(Debug, Deserialize)]
struct RegistryBundleManifest {
    bundle_id: String,
    version: String,
    scope: String,
    capabilities: Vec<BundleArtifactManifestSerde>,
    events: Vec<BundleArtifactManifestSerde>,
    workflows: Vec<BundleArtifactManifestSerde>,
}

#[derive(Debug, Deserialize)]
struct BundleArtifactManifestSerde {
    id: String,
    version: String,
    path: String,
}

/// Loads one canonical registry bundle manifest together with all referenced
/// capability, event, and workflow artifacts.
///
/// # Errors
///
/// Returns [`BundleLoadFailure`] when the manifest cannot be read or parsed,
/// the bundle contains duplicate or missing artifact entries, or an artifact's
/// file contents do not match the manifest identity.
pub fn load_registry_bundle(manifest_path: &Path) -> Result<RegistryBundle, BundleLoadFailure> {
    let manifest_dir = manifest_path.parent().ok_or_else(|| {
        single_error(
            BundleLoadErrorCode::ManifestParentMissing,
            manifest_path.display().to_string(),
            format!(
                "bundle manifest {} has no parent directory",
                manifest_path.display()
            ),
        )
    })?;

    let manifest_contents = fs::read_to_string(manifest_path).map_err(|error| {
        single_error(
            BundleLoadErrorCode::ManifestReadFailed,
            manifest_path.display().to_string(),
            format!(
                "failed to read bundle manifest {}: {error}",
                manifest_path.display()
            ),
        )
    })?;

    let manifest: RegistryBundleManifest =
        serde_json::from_str(&manifest_contents).map_err(|error| {
            single_error(
                BundleLoadErrorCode::ManifestParseFailed,
                manifest_path.display().to_string(),
                format!(
                    "failed to parse bundle manifest {}: {error}",
                    manifest_path.display()
                ),
            )
        })?;

    let scope = parse_scope(&manifest.scope).ok_or_else(|| {
        single_error(
            BundleLoadErrorCode::InvalidScope,
            "$.scope".to_string(),
            format!(
                "bundle manifest {} declares unsupported scope {}",
                manifest_path.display(),
                manifest.scope
            ),
        )
    })?;

    ensure_unique_ids(&manifest.capabilities, "capability")?;
    ensure_unique_ids(&manifest.events, "event")?;
    ensure_unique_ids(&manifest.workflows, "workflow")?;

    let capabilities = manifest
        .capabilities
        .iter()
        .map(|artifact| load_capability_artifact(manifest_dir, artifact))
        .collect::<Result<Vec<_>, _>>()?;
    let events = manifest
        .events
        .iter()
        .map(|artifact| load_event_artifact(manifest_dir, artifact))
        .collect::<Result<Vec<_>, _>>()?;
    let workflows = manifest
        .workflows
        .iter()
        .map(|artifact| load_workflow_artifact(manifest_dir, artifact))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(RegistryBundle {
        bundle_id: manifest.bundle_id,
        version: manifest.version,
        scope,
        capabilities,
        events,
        workflows,
    })
}

fn ensure_unique_ids(
    artifacts: &[BundleArtifactManifestSerde],
    artifact_kind: &str,
) -> Result<(), BundleLoadFailure> {
    let mut seen = BTreeSet::new();
    for artifact in artifacts {
        let key = format!("{}@{}", artifact.id, artifact.version);
        if !seen.insert(key.clone()) {
            return Err(single_error(
                BundleLoadErrorCode::DuplicateArtifactId,
                key.clone(),
                format!("duplicate {artifact_kind} artifact entry in bundle manifest: {key}"),
            ));
        }
    }
    Ok(())
}

fn load_capability_artifact(
    manifest_dir: &Path,
    artifact: &BundleArtifactManifestSerde,
) -> Result<CapabilityBundleArtifact, BundleLoadFailure> {
    let path = resolve_artifact_path(manifest_dir, artifact)?;
    let contents = read_artifact_file(&path)?;
    let contract = parse_contract(&contents).map_err(|failure| {
        map_contract_failure(
            BundleLoadErrorCode::CapabilityParseFailed,
            &path,
            &artifact.id,
            failure.errors,
        )
    })?;
    ensure_artifact_identity(&path, artifact, &contract.id, &contract.version)?;

    Ok(CapabilityBundleArtifact {
        manifest: to_manifest(artifact),
        path,
        contract,
    })
}

fn load_event_artifact(
    manifest_dir: &Path,
    artifact: &BundleArtifactManifestSerde,
) -> Result<EventBundleArtifact, BundleLoadFailure> {
    let path = resolve_artifact_path(manifest_dir, artifact)?;
    let contents = read_artifact_file(&path)?;
    let contract = parse_event_contract(&contents).map_err(|failure| {
        map_contract_failure(
            BundleLoadErrorCode::EventParseFailed,
            &path,
            &artifact.id,
            failure.errors,
        )
    })?;
    ensure_artifact_identity(&path, artifact, &contract.id, &contract.version)?;

    Ok(EventBundleArtifact {
        manifest: to_manifest(artifact),
        path,
        contract,
    })
}

fn load_workflow_artifact(
    manifest_dir: &Path,
    artifact: &BundleArtifactManifestSerde,
) -> Result<WorkflowBundleArtifact, BundleLoadFailure> {
    let path = resolve_artifact_path(manifest_dir, artifact)?;
    let contents = read_artifact_file(&path)?;
    let definition = serde_json::from_str::<WorkflowDefinition>(&contents).map_err(|error| {
        single_error(
            BundleLoadErrorCode::WorkflowParseFailed,
            path.display().to_string(),
            format!(
                "failed to parse workflow artifact {} for {}: {error}",
                path.display(),
                artifact.id
            ),
        )
    })?;
    ensure_artifact_identity(&path, artifact, &definition.id, &definition.version)?;

    Ok(WorkflowBundleArtifact {
        manifest: to_manifest(artifact),
        path,
        definition,
    })
}

fn resolve_artifact_path(
    manifest_dir: &Path,
    artifact: &BundleArtifactManifestSerde,
) -> Result<PathBuf, BundleLoadFailure> {
    let path = manifest_dir.join(&artifact.path);
    if !path.is_file() {
        return Err(single_error(
            BundleLoadErrorCode::MissingArtifactFile,
            path.display().to_string(),
            format!(
                "missing artifact file for {} at {}",
                artifact.id,
                path.display()
            ),
        ));
    }
    Ok(path)
}

fn read_artifact_file(path: &Path) -> Result<String, BundleLoadFailure> {
    fs::read_to_string(path).map_err(|error| {
        single_error(
            BundleLoadErrorCode::MissingArtifactFile,
            path.display().to_string(),
            format!("failed to read artifact file {}: {error}", path.display()),
        )
    })
}

fn ensure_artifact_identity(
    path: &Path,
    artifact: &BundleArtifactManifestSerde,
    actual_id: &str,
    actual_version: &str,
) -> Result<(), BundleLoadFailure> {
    if artifact.id != actual_id {
        return Err(single_error(
            BundleLoadErrorCode::ArtifactIdMismatch,
            path.display().to_string(),
            format!(
                "artifact id mismatch for {}: manifest declared {}, file contains {}",
                path.display(),
                artifact.id,
                actual_id
            ),
        ));
    }
    if artifact.version != actual_version {
        return Err(single_error(
            BundleLoadErrorCode::ArtifactVersionMismatch,
            path.display().to_string(),
            format!(
                "artifact version mismatch for {}: manifest declared {}, file contains {}",
                path.display(),
                artifact.version,
                actual_version
            ),
        ));
    }

    Ok(())
}

fn map_contract_failure(
    code: BundleLoadErrorCode,
    path: &Path,
    artifact_id: &str,
    errors: Vec<ValidationError>,
) -> BundleLoadFailure {
    let detail = errors
        .into_iter()
        .map(|error| format!("{} at {}", error.message, error.path))
        .collect::<Vec<_>>()
        .join("; ");

    single_error(
        code,
        path.display().to_string(),
        format!(
            "failed to parse artifact {} at {}: {}",
            artifact_id,
            path.display(),
            detail
        ),
    )
}

fn parse_scope(value: &str) -> Option<RegistryScope> {
    match value {
        "public" => Some(RegistryScope::Public),
        "private" => Some(RegistryScope::Private),
        _ => None,
    }
}

fn to_manifest(artifact: &BundleArtifactManifestSerde) -> BundleArtifactManifest {
    BundleArtifactManifest {
        id: artifact.id.clone(),
        version: artifact.version.clone(),
        path: artifact.path.clone(),
    }
}

fn single_error(code: BundleLoadErrorCode, path: String, message: String) -> BundleLoadFailure {
    BundleLoadFailure {
        errors: vec![BundleLoadError {
            code,
            path,
            message,
        }],
    }
}

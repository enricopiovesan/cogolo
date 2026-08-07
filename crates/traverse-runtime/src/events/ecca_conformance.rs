//! ECCA catalog-drift reconciliation and migration-exit conformance.
//!
//! Closes the remaining #897 slices against `traverse-registry` 0.11.0:
//! declared/observed drift via [`ObservedLineageStore`], portable descriptor
//! fixture conformance, and Spec 534 FR-015 migration-exit evidence.

use crate::events::{
    EventLineageRecord, EventQuarantineRecord, EventTelemetryRecord, EventValidationEvidence,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use traverse_registry::{
    DriftEvidence, EventProductDescriptor, EventProductErrorCode, EventProductRegistry,
    LookupScope, ObservedEventInteraction, ObservedLineageStore, ObservedRole, RegistryScope,
    validate_event_product_descriptor,
};

/// Reconciles broker lineage against a declared [`EventProductRegistry`].
#[derive(Debug, Clone, Default)]
pub struct CatalogDriftReconciler {
    registry: EventProductRegistry,
    observed: ObservedLineageStore,
    observed_keys: BTreeSet<(String, String)>,
}

impl CatalogDriftReconciler {
    #[must_use]
    pub fn new(registry: EventProductRegistry) -> Self {
        Self {
            registry,
            observed: ObservedLineageStore::new(),
            observed_keys: BTreeSet::new(),
        }
    }

    #[must_use]
    pub fn registry(&self) -> &EventProductRegistry {
        &self.registry
    }

    #[must_use]
    pub fn observed(&self) -> &ObservedLineageStore {
        &self.observed
    }

    /// Records one publish observation against declared publishers for the event.
    pub fn observe_publication(
        &mut self,
        event_id: &str,
        event_version: &str,
        capability_id: &str,
        observed_at: &str,
    ) {
        self.observed_keys
            .insert((event_id.to_string(), event_version.to_string()));
        let declared = declared_capability_ids(
            &self.registry,
            event_id,
            event_version,
            ObservedRole::Publisher,
        );
        self.observed.record(
            ObservedEventInteraction {
                event_id: event_id.to_string(),
                event_version: event_version.to_string(),
                capability_id: capability_id.to_string(),
                role: ObservedRole::Publisher,
                observed_at: observed_at.to_string(),
            },
            &declared,
        );
    }

    /// Records one consume observation against declared subscribers for the event.
    pub fn observe_consumption(
        &mut self,
        event_id: &str,
        event_version: &str,
        capability_id: &str,
        observed_at: &str,
    ) {
        self.observed_keys
            .insert((event_id.to_string(), event_version.to_string()));
        let declared = declared_capability_ids(
            &self.registry,
            event_id,
            event_version,
            ObservedRole::Subscriber,
        );
        self.observed.record(
            ObservedEventInteraction {
                event_id: event_id.to_string(),
                event_version: event_version.to_string(),
                capability_id: capability_id.to_string(),
                role: ObservedRole::Subscriber,
                observed_at: observed_at.to_string(),
            },
            &declared,
        );
    }

    /// Projects sanitized broker lineage into the registry observed-lineage store.
    pub fn reconcile_broker_lineage(&mut self, lineage: &[EventLineageRecord], observed_at: &str) {
        for record in lineage {
            self.observe_publication(
                &record.contract_id,
                &record.contract_version,
                &record.producer_id,
                observed_at,
            );
            self.observe_consumption(
                &record.contract_id,
                &record.contract_version,
                &record.consumer_id,
                observed_at,
            );
        }
    }

    /// All unresolved drift evidence recorded so far, in deterministic key order.
    #[must_use]
    pub fn unresolved_drift(&self) -> Vec<DriftEvidence> {
        let mut keys = self.observed_keys.clone();
        for descriptor in self.registry.discover(LookupScope::PreferPrivate) {
            keys.insert((
                descriptor.contract.id.clone(),
                descriptor.contract.version.clone(),
            ));
        }
        keys.into_iter()
            .flat_map(|(event_id, event_version)| {
                self.observed
                    .drift_for(&event_id, &event_version)
                    .into_iter()
                    .cloned()
            })
            .collect()
    }
}

fn declared_capability_ids(
    registry: &EventProductRegistry,
    event_id: &str,
    event_version: &str,
    role: ObservedRole,
) -> Vec<String> {
    let Some(descriptor) = find_descriptor(registry, event_id, event_version) else {
        return Vec::new();
    };
    match role {
        ObservedRole::Publisher => descriptor
            .contract
            .publishers
            .iter()
            .map(|reference| reference.capability_id.clone())
            .collect(),
        ObservedRole::Subscriber => descriptor
            .contract
            .subscribers
            .iter()
            .map(|reference| reference.capability_id.clone())
            .collect(),
    }
}

fn find_descriptor<'a>(
    registry: &'a EventProductRegistry,
    event_id: &str,
    event_version: &str,
) -> Option<&'a EventProductDescriptor> {
    registry
        .find_exact(RegistryScope::Private, event_id, event_version)
        .or_else(|| registry.find_exact(RegistryScope::Public, event_id, event_version))
}

/// Stable finding codes for FR-015 migration-exit evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationExitFindingCode {
    MissingProducerTelemetry,
    MissingConsumerTelemetry,
    UnresolvedDrift,
    InvalidEventFinding,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationExitFinding {
    pub code: MigrationExitFindingCode,
    pub detail: String,
}

/// Persisted evidence for one release conformance run (FR-015 consecutive runs).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationExitEvidence {
    pub run_id: String,
    pub clean: bool,
    pub finding_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationExitReport {
    pub permitted: bool,
    pub findings: Vec<MigrationExitFinding>,
    pub evidence: MigrationExitEvidence,
    pub previous_run_clean: bool,
}

/// Evaluate Spec 534 FR-015 cutover readiness for one conformance run.
#[must_use]
pub fn evaluate_migration_exit(
    registry: &EventProductRegistry,
    reconciler: &CatalogDriftReconciler,
    validation_evidence: &[EventValidationEvidence],
    quarantine: &[EventQuarantineRecord],
    telemetry: &[EventTelemetryRecord],
    previous: Option<&MigrationExitEvidence>,
    run_id: &str,
) -> MigrationExitReport {
    let mut findings = Vec::new();

    // Registered descriptors already passed `validate_event_product_descriptor`
    // at register time; FR-015 "every published contract validates" is therefore
    // satisfied by registry membership plus the portable fixture suite.
    for descriptor in registry.discover(LookupScope::PreferPrivate) {
        let contract_id = descriptor.contract.id.as_str();
        let contract_version = descriptor.contract.version.as_str();
        let has_publish_telemetry = telemetry.iter().any(|record| {
            record.operation == "traverse.event.publish"
                && record.contract_id == contract_id
                && record.contract_version == contract_version
        });
        if !descriptor.contract.publishers.is_empty() && !has_publish_telemetry {
            findings.push(MigrationExitFinding {
                code: MigrationExitFindingCode::MissingProducerTelemetry,
                detail: format!(
                    "declared producers for {contract_id}@{contract_version} have no publish telemetry"
                ),
            });
        }

        for subscriber in &descriptor.contract.subscribers {
            let has_delivery = telemetry.iter().any(|record| {
                record.operation == "traverse.event.delivery"
                    && record.contract_id == contract_id
                    && record.contract_version == contract_version
                    && record.consumer_id.as_deref() == Some(subscriber.capability_id.as_str())
            });
            if !has_delivery {
                findings.push(MigrationExitFinding {
                    code: MigrationExitFindingCode::MissingConsumerTelemetry,
                    detail: format!(
                        "declared consumer '{}' for {contract_id}@{contract_version} has no delivery telemetry",
                        subscriber.capability_id
                    ),
                });
            }
        }
    }

    for evidence in reconciler.unresolved_drift() {
        findings.push(MigrationExitFinding {
            code: MigrationExitFindingCode::UnresolvedDrift,
            detail: format!(
                "{:?} capability '{}' on {}@{}",
                evidence.kind, evidence.capability_id, evidence.event_id, evidence.event_version
            ),
        });
    }

    for evidence in validation_evidence {
        findings.push(MigrationExitFinding {
            code: MigrationExitFindingCode::InvalidEventFinding,
            detail: format!(
                "validation diagnostic for {}@{} ({})",
                evidence.contract_id,
                evidence.version,
                evidence
                    .diagnostics
                    .first()
                    .map_or("unknown", |diagnostic| diagnostic.code)
            ),
        });
    }
    for record in quarantine {
        findings.push(MigrationExitFinding {
            code: MigrationExitFindingCode::InvalidEventFinding,
            detail: format!(
                "quarantine recorded for {}@{}",
                record.evidence.contract_id, record.evidence.version
            ),
        });
    }

    let clean = findings.is_empty();
    let finding_count = findings.len();
    let previous_run_clean = previous.is_some_and(|evidence| evidence.clean);
    let permitted = clean && previous_run_clean;

    MigrationExitReport {
        permitted,
        findings,
        evidence: MigrationExitEvidence {
            run_id: run_id.to_string(),
            clean,
            finding_count,
        },
        previous_run_clean,
    }
}

/// One portable descriptor-fixture expectation from the registry corpus.
#[derive(Debug, Clone, Deserialize)]
struct FixtureManifest {
    fixtures: Vec<FixtureEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct FixtureEntry {
    file: String,
    expect: String,
    #[serde(default)]
    error_code: Option<String>,
    #[serde(default)]
    existing: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureConformanceFailure {
    pub file: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureConformanceReport {
    pub passed: usize,
    pub failures: Vec<FixtureConformanceFailure>,
}

/// Run the portable ECCA descriptor fixture corpus against
/// [`validate_event_product_descriptor`].
///
/// # Errors
///
/// Returns an error when the manifest or fixture files cannot be read/parsed.
pub fn run_descriptor_fixture_conformance(
    fixtures_dir: &Path,
) -> Result<FixtureConformanceReport, String> {
    let manifest_path = fixtures_dir.join("MANIFEST.json");
    let manifest_text = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("failed to read {}: {error}", manifest_path.display()))?;
    let manifest: FixtureManifest = serde_json::from_str(&manifest_text)
        .map_err(|error| format!("failed to parse {}: {error}", manifest_path.display()))?;

    let mut passed = 0;
    let mut failures = Vec::new();
    let mut loaded: BTreeMap<String, EventProductDescriptor> = BTreeMap::new();

    for entry in &manifest.fixtures {
        let path = fixtures_dir.join(&entry.file);
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) => {
                failures.push(FixtureConformanceFailure {
                    file: entry.file.clone(),
                    detail: format!("failed to read fixture: {error}"),
                });
                continue;
            }
        };
        let descriptor: EventProductDescriptor = match serde_json::from_str(&text) {
            Ok(descriptor) => descriptor,
            Err(error) => {
                failures.push(FixtureConformanceFailure {
                    file: entry.file.clone(),
                    detail: format!("failed to parse fixture JSON: {error}"),
                });
                continue;
            }
        };

        let existing = entry
            .existing
            .as_ref()
            .and_then(|existing_file| loaded.get(existing_file));
        let result = validate_event_product_descriptor(&descriptor, existing);
        match (entry.expect.as_str(), result) {
            ("accept", Ok(())) => {
                passed += 1;
                loaded.insert(entry.file.clone(), descriptor);
            }
            ("accept", Err(failure)) => failures.push(FixtureConformanceFailure {
                file: entry.file.clone(),
                detail: format!(
                    "expected accept, got {:?}",
                    failure
                        .errors
                        .first()
                        .map_or(EventProductErrorCode::MissingSupportRoute, |error| error
                            .code)
                ),
            }),
            ("reject", Ok(())) => failures.push(FixtureConformanceFailure {
                file: entry.file.clone(),
                detail: "expected reject, got accept".to_string(),
            }),
            ("reject", Err(failure)) => {
                let actual = failure
                    .errors
                    .first()
                    .map(|error| format!("{:?}", error.code));
                if entry.error_code.as_ref() == actual.as_ref() {
                    passed += 1;
                } else {
                    failures.push(FixtureConformanceFailure {
                        file: entry.file.clone(),
                        detail: format!(
                            "expected error_code {:?}, got {:?}",
                            entry.error_code, actual
                        ),
                    });
                }
            }
            (other, _) => failures.push(FixtureConformanceFailure {
                file: entry.file.clone(),
                detail: format!("unsupported expect value '{other}'"),
            }),
        }
    }

    Ok(FixtureConformanceReport { passed, failures })
}

/// Validate one event-product descriptor JSON document.
///
/// # Errors
///
/// Returns a stable string error when the file is unreadable, unparsable, or
/// fails ECCA descriptor validation.
pub fn validate_event_product_file(path: &Path) -> Result<EventProductDescriptor, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read event product descriptor: {error}"))?;
    let descriptor: EventProductDescriptor = serde_json::from_str(&text)
        .map_err(|error| format!("failed to parse event product descriptor: {error}"))?;
    validate_event_product_descriptor(&descriptor, None).map_err(|failure| {
        failure
            .errors
            .into_iter()
            .map(|error| format!("{:?}: {}", error.code, error.message))
            .collect::<Vec<_>>()
            .join("; ")
    })?;
    Ok(descriptor)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::events::validation::EventValidationDiagnostic;
    use traverse_contracts::EventContract;
    use traverse_registry::{
        DataClassification, DriftKind, EventExposureClass, EventProductRegistration,
        FieldClassification,
    };

    fn sample_descriptor(
        event_id: &str,
        publisher: &str,
        subscriber: Option<&str>,
    ) -> EventProductDescriptor {
        let mut contract: EventContract = serde_json::from_str(
            r#"{
              "kind":"event_contract",
              "schema_version":"1.0.0",
              "id":"placeholder",
              "namespace":"content.comments",
              "name":"comment-draft-created",
              "version":"1.0.0",
              "lifecycle":"active",
              "owner":{"team":"traverse-core","contact":"test@example.com"},
              "summary":"Published when a comment draft has been created.",
              "description":"Governed event contract for comment draft creation.",
              "payload":{"schema":{"type":"object","properties":{"draft_id":{"type":"string"}},"required":["draft_id"]},"compatibility":"backward-compatible"},
              "classification":{"domain":"content.comments","bounded_context":"comments","event_type":"domain","tags":[]},
              "publishers":[{"capability_id":"content.comments.create-comment-draft","version":"1.0.0"}],
              "subscribers":[],
              "policies":[],
              "tags":[],
              "provenance":{"source":"greenfield","author":"test","created_at":"2026-03-30T00:00:00Z"},
              "evidence":[]
            }"#,
        )
        .expect("contract json");
        contract.id = event_id.to_string();
        contract.name = event_id.rsplit('.').next().unwrap_or(event_id).to_string();
        contract.publishers[0].capability_id = publisher.to_string();
        if let Some(subscriber) = subscriber {
            contract.subscribers = vec![traverse_contracts::CapabilityReference {
                capability_id: subscriber.to_string(),
                version: "1.0.0".to_string(),
            }];
        }
        EventProductDescriptor {
            contract,
            support_route: "https://support.traverse.dev/comments".to_string(),
            exposure: EventExposureClass::Internal,
            field_classifications: vec![FieldClassification {
                field_path: "draft_id".to_string(),
                classification: DataClassification::NoClassification,
            }],
            replacement: None,
            cloud_events_source: format!("traverse://capability/{publisher}"),
            cloud_events_subject_field: Some("draft_id".to_string()),
            deduplication_id_field: "draft_id".to_string(),
            ordering_scope_field: None,
            correlation_id_field: "envelope.correlation_id".to_string(),
            causation_id_field: Some("envelope.causation_id".to_string()),
            retention_policy: "retain 90 days".to_string(),
        }
    }

    fn registry_with(descriptor: EventProductDescriptor) -> EventProductRegistry {
        let mut registry = EventProductRegistry::new();
        registry
            .register(EventProductRegistration {
                scope: RegistryScope::Private,
                descriptor,
            })
            .expect("descriptor must register");
        registry
    }

    #[test]
    fn declared_producer_and_consumer_produce_no_drift() {
        let descriptor = sample_descriptor(
            "content.comments.comment-draft-created",
            "content.comments.create-comment-draft",
            Some("content.comments.notify-author"),
        );
        let mut reconciler = CatalogDriftReconciler::new(registry_with(descriptor));
        reconciler.observe_publication(
            "content.comments.comment-draft-created",
            "1.0.0",
            "content.comments.create-comment-draft",
            "2026-08-07T00:00:00Z",
        );
        reconciler.observe_consumption(
            "content.comments.comment-draft-created",
            "1.0.0",
            "content.comments.notify-author",
            "2026-08-07T00:00:01Z",
        );
        assert!(reconciler.unresolved_drift().is_empty());
    }

    #[test]
    fn undeclared_producer_is_reported_as_drift() {
        let descriptor = sample_descriptor(
            "content.comments.comment-draft-created",
            "content.comments.create-comment-draft",
            None,
        );
        let mut reconciler = CatalogDriftReconciler::new(registry_with(descriptor));
        reconciler.observe_publication(
            "content.comments.comment-draft-created",
            "1.0.0",
            "rogue.publisher",
            "2026-08-07T00:00:00Z",
        );
        let drift = reconciler.unresolved_drift();
        assert_eq!(drift.len(), 1);
        assert_eq!(drift[0].kind, DriftKind::UndeclaredPublisher);
        assert_eq!(drift[0].capability_id, "rogue.publisher");
    }

    #[test]
    fn broker_lineage_projects_into_observed_store() {
        let descriptor = sample_descriptor(
            "content.comments.comment-draft-created",
            "content.comments.create-comment-draft",
            Some("content.comments.notify-author"),
        );
        let mut reconciler = CatalogDriftReconciler::new(registry_with(descriptor));
        reconciler.reconcile_broker_lineage(
            &[EventLineageRecord {
                contract_id: "content.comments.comment-draft-created".to_string(),
                contract_version: "1.0.0".to_string(),
                event_id: "evt-1".to_string(),
                producer_id: "content.comments.create-comment-draft".to_string(),
                consumer_id: "content.comments.notify-author".to_string(),
                subscription_id: "sub-1".to_string(),
                cursor: "1".to_string(),
            }],
            "2026-08-07T00:00:00Z",
        );
        assert!(reconciler.unresolved_drift().is_empty());
        assert_eq!(
            reconciler
                .observed()
                .interactions_for("content.comments.comment-draft-created", "1.0.0")
                .len(),
            2
        );
    }

    #[test]
    fn migration_exit_requires_two_consecutive_clean_runs() {
        let descriptor = sample_descriptor(
            "content.comments.comment-draft-created",
            "content.comments.create-comment-draft",
            Some("content.comments.notify-author"),
        );
        let registry = registry_with(descriptor);
        let mut reconciler = CatalogDriftReconciler::new(registry.clone());
        reconciler.observe_publication(
            "content.comments.comment-draft-created",
            "1.0.0",
            "content.comments.create-comment-draft",
            "2026-08-07T00:00:00Z",
        );
        reconciler.observe_consumption(
            "content.comments.comment-draft-created",
            "1.0.0",
            "content.comments.notify-author",
            "2026-08-07T00:00:01Z",
        );
        let telemetry = vec![
            EventTelemetryRecord {
                operation: "traverse.event.publish",
                outcome: "accepted",
                contract_id: "content.comments.comment-draft-created".to_string(),
                contract_version: "1.0.0".to_string(),
                event_id: "evt-1".to_string(),
                deduplication_id: None,
                ordering_scope: None,
                correlation_id: None,
                causation_id: None,
                consumer_id: None,
                cursor: None,
                retry_count: 0,
                latency_ms: 0,
            },
            EventTelemetryRecord {
                operation: "traverse.event.delivery",
                outcome: "delivered",
                contract_id: "content.comments.comment-draft-created".to_string(),
                contract_version: "1.0.0".to_string(),
                event_id: "evt-1".to_string(),
                deduplication_id: None,
                ordering_scope: None,
                correlation_id: None,
                causation_id: None,
                consumer_id: Some("content.comments.notify-author".to_string()),
                cursor: Some("1".to_string()),
                retry_count: 0,
                latency_ms: 0,
            },
        ];

        let first =
            evaluate_migration_exit(&registry, &reconciler, &[], &[], &telemetry, None, "run-1");
        assert!(first.evidence.clean, "findings: {:?}", first.findings);
        assert!(!first.permitted);

        let second = evaluate_migration_exit(
            &registry,
            &reconciler,
            &[],
            &[],
            &telemetry,
            Some(&first.evidence),
            "run-2",
        );
        assert!(second.evidence.clean);
        assert!(second.permitted);
        assert!(second.previous_run_clean);
    }

    #[test]
    fn migration_exit_blocks_on_invalid_event_findings() {
        let descriptor = sample_descriptor(
            "content.comments.comment-draft-created",
            "content.comments.create-comment-draft",
            None,
        );
        let registry = registry_with(descriptor);
        let reconciler = CatalogDriftReconciler::new(registry.clone());
        let evidence = EventValidationEvidence {
            contract_id: "content.comments.comment-draft-created".to_string(),
            version: "1.0.0".to_string(),
            diagnostics: vec![EventValidationDiagnostic {
                code: "EVP-001",
                path: "/id",
                severity: "error",
                remediation: "provide id",
                contract_id: "content.comments.comment-draft-created".to_string(),
                version: "1.0.0".to_string(),
            }],
        };
        let report = evaluate_migration_exit(
            &registry,
            &reconciler,
            &[evidence],
            &[],
            &[],
            None,
            "run-bad",
        );
        assert!(!report.evidence.clean);
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.code == MigrationExitFindingCode::InvalidEventFinding)
        );
    }

    #[test]
    fn validate_event_product_file_accepts_valid_fixture() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/ecca-event-products/valid.json");
        let descriptor = validate_event_product_file(&path).expect("valid fixture");
        assert_eq!(
            descriptor.contract.id,
            "content.comments.comment-draft-created"
        );
    }

    #[test]
    fn registry_accessor_and_unknown_event_observation_are_covered() {
        let descriptor = sample_descriptor(
            "content.comments.comment-draft-created",
            "content.comments.create-comment-draft",
            None,
        );
        let mut reconciler = CatalogDriftReconciler::new(registry_with(descriptor));
        assert_eq!(
            reconciler
                .registry()
                .discover(LookupScope::PreferPrivate)
                .len(),
            1
        );
        reconciler.observe_publication(
            "unknown.event",
            "1.0.0",
            "any.capability",
            "2026-08-07T00:00:00Z",
        );
        assert_eq!(reconciler.unresolved_drift().len(), 1);
    }

    #[test]
    fn public_scope_descriptors_resolve_for_declared_capabilities() {
        let descriptor = sample_descriptor(
            "content.comments.comment-draft-created",
            "content.comments.create-comment-draft",
            None,
        );
        let mut registry = EventProductRegistry::new();
        registry
            .register(EventProductRegistration {
                scope: RegistryScope::Public,
                descriptor,
            })
            .expect("public descriptor must register");
        let mut reconciler = CatalogDriftReconciler::new(registry);
        reconciler.observe_publication(
            "content.comments.comment-draft-created",
            "1.0.0",
            "content.comments.create-comment-draft",
            "2026-08-07T00:00:00Z",
        );
        assert!(reconciler.unresolved_drift().is_empty());
    }

    #[test]
    fn migration_exit_reports_missing_consumer_telemetry_and_drift() {
        let descriptor = sample_descriptor(
            "content.comments.comment-draft-created",
            "content.comments.create-comment-draft",
            Some("content.comments.notify-author"),
        );
        let registry = registry_with(descriptor);
        let mut reconciler = CatalogDriftReconciler::new(registry.clone());
        reconciler.observe_publication(
            "content.comments.comment-draft-created",
            "1.0.0",
            "rogue.publisher",
            "2026-08-07T00:00:00Z",
        );
        let report =
            evaluate_migration_exit(&registry, &reconciler, &[], &[], &[], None, "run-gaps");
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.code == MigrationExitFindingCode::MissingProducerTelemetry)
        );
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.code == MigrationExitFindingCode::MissingConsumerTelemetry)
        );
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.code == MigrationExitFindingCode::UnresolvedDrift)
        );
    }

    #[test]
    fn migration_exit_reports_quarantine_and_empty_diagnostic_codes() {
        let descriptor = sample_descriptor(
            "content.comments.comment-draft-created",
            "content.comments.create-comment-draft",
            None,
        );
        let registry = registry_with(descriptor);
        let reconciler = CatalogDriftReconciler::new(registry.clone());
        let evidence = EventValidationEvidence {
            contract_id: "content.comments.comment-draft-created".to_string(),
            version: "1.0.0".to_string(),
            diagnostics: Vec::new(),
        };
        let quarantine = EventQuarantineRecord {
            evidence: evidence.clone(),
        };
        let report = evaluate_migration_exit(
            &registry,
            &reconciler,
            std::slice::from_ref(&evidence),
            std::slice::from_ref(&quarantine),
            &[],
            None,
            "run-quarantine",
        );
        assert_eq!(
            report
                .findings
                .iter()
                .filter(|finding| finding.code == MigrationExitFindingCode::InvalidEventFinding)
                .count(),
            2
        );
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.detail.contains("unknown"))
        );
    }

    #[test]
    fn validate_event_product_file_rejects_invalid_fixture() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/ecca-event-products/reject_missing_support_route.json");
        let error = validate_event_product_file(&path).expect_err("invalid fixture");
        assert!(error.contains("MissingSupportRoute"));
    }

    #[test]
    fn fixture_conformance_reports_error_and_mismatch_branches() {
        let root =
            std::env::temp_dir().join(format!("ecca-fixture-branches-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temp fixture dir");

        let valid = fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/ecca-event-products/valid.json"),
        )
        .expect("valid fixture");
        fs::write(root.join("valid.json"), &valid).expect("write valid");
        fs::write(root.join("broken.json"), "{not-json").expect("write broken");
        fs::write(
            root.join("MANIFEST.json"),
            r#"{
              "fixtures": [
                {"file":"valid.json","expect":"accept"},
                {"file":"missing.json","expect":"accept"},
                {"file":"broken.json","expect":"accept"},
                {"file":"valid.json","expect":"reject","error_code":"MissingSupportRoute"},
                {"file":"valid.json","expect":"reject","error_code":"WrongCode"},
                {"file":"valid.json","expect":"weird"}
              ]
            }"#,
        )
        .expect("write manifest");

        // Force accept-path rejection by pointing expect=accept at a reject fixture.
        let reject = fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/ecca-event-products/reject_missing_support_route.json"),
        )
        .expect("reject fixture");
        fs::write(root.join("invalid.json"), reject).expect("write invalid");
        fs::write(
            root.join("MANIFEST.json"),
            r#"{
              "fixtures": [
                {"file":"valid.json","expect":"accept"},
                {"file":"missing.json","expect":"accept"},
                {"file":"broken.json","expect":"accept"},
                {"file":"invalid.json","expect":"accept"},
                {"file":"valid.json","expect":"reject","error_code":"MissingSupportRoute"},
                {"file":"invalid.json","expect":"reject","error_code":"WrongCode"},
                {"file":"valid.json","expect":"weird"}
              ]
            }"#,
        )
        .expect("rewrite manifest");

        let report = run_descriptor_fixture_conformance(&root).expect("runner must return");
        assert!(!report.failures.is_empty());
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.detail.contains("failed to read fixture"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.detail.contains("failed to parse fixture JSON"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.detail.contains("expected accept, got"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.detail.contains("expected reject, got accept"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.detail.contains("expected error_code"))
        );
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.detail.contains("unsupported expect value"))
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn fixture_conformance_rejects_unreadable_manifest() {
        let root = std::env::temp_dir().join(format!(
            "ecca-fixture-missing-manifest-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temp dir");
        let error = run_descriptor_fixture_conformance(&root).expect_err("missing manifest");
        assert!(error.contains("failed to read"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn fixture_conformance_rejects_unparsable_manifest() {
        let root =
            std::env::temp_dir().join(format!("ecca-fixture-bad-manifest-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temp dir");
        fs::write(root.join("MANIFEST.json"), "{not-json").expect("write");
        let error = run_descriptor_fixture_conformance(&root).expect_err("bad manifest");
        assert!(error.contains("failed to parse"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn validate_event_product_file_reports_io_and_parse_errors() {
        let missing = std::env::temp_dir().join(format!(
            "ecca-missing-descriptor-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&missing);
        let io_error = validate_event_product_file(&missing).expect_err("missing file");
        assert!(io_error.contains("failed to read"));

        let broken = std::env::temp_dir().join(format!(
            "ecca-broken-descriptor-{}.json",
            std::process::id()
        ));
        fs::write(&broken, "{not-json").expect("write broken");
        let parse_error = validate_event_product_file(&broken).expect_err("bad json");
        assert!(parse_error.contains("failed to parse"));
        let _ = fs::remove_file(&broken);
    }
}

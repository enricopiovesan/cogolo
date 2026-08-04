//! Deterministic runtime validation for governed event-product envelopes.
//!
//! Governed by approved spec `534-ecca-event-products` (FR-012 through FR-014).

use semver::Version;

use super::TraverseEvent;

/// Runtime policy used at producer and consumer boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventValidationMode {
    /// Record violations without rejecting legacy traffic.
    Migration,
    /// Reject traffic that does not satisfy the governed envelope profile.
    Enforcement,
}

/// Stable, machine-readable diagnostic for an invalid event envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventValidationDiagnostic {
    pub code: &'static str,
    pub path: &'static str,
    pub severity: &'static str,
    pub remediation: &'static str,
    pub contract_id: String,
    pub version: String,
}

/// Result of applying the portable event-product envelope profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventValidationResult {
    pub accepted: bool,
    pub diagnostics: Vec<EventValidationDiagnostic>,
}

/// Sanitized quarantine evidence for a rejected event.
///
/// It retains only contract identity and deterministic diagnostics. Event data,
/// credentials, and other envelope values are deliberately excluded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventValidationEvidence {
    pub contract_id: String,
    pub version: String,
    pub diagnostics: Vec<EventValidationDiagnostic>,
}

impl EventValidationEvidence {
    #[must_use]
    pub fn from_result(result: &EventValidationResult) -> Option<Self> {
        let first = result.diagnostics.first()?;
        Some(Self {
            contract_id: first.contract_id.clone(),
            version: first.version.clone(),
            diagnostics: result.diagnostics.clone(),
        })
    }
}

impl EventValidationResult {
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

/// Validate a `TraverseEvent` without host or transport dependencies.
///
/// Migration mode preserves delivery while surfacing deterministic evidence;
/// enforcement mode rejects the same invalid envelope.
#[must_use]
pub fn validate_event(event: &TraverseEvent, mode: EventValidationMode) -> EventValidationResult {
    let mut diagnostics = Vec::new();
    validate_non_empty(&mut diagnostics, "EVP-001", "/id", "id", &event.id, event);
    validate_non_empty(
        &mut diagnostics,
        "EVP-002",
        "/source",
        "source",
        &event.source,
        event,
    );
    validate_non_empty(
        &mut diagnostics,
        "EVP-003",
        "/datacontenttype",
        "datacontenttype",
        &event.datacontenttype,
        event,
    );
    validate_non_empty(
        &mut diagnostics,
        "EVP-004",
        "/time",
        "time",
        &event.time,
        event,
    );
    validate_non_empty(
        &mut diagnostics,
        "EVP-005",
        "/owner",
        "owner",
        &event.owner,
        event,
    );
    validate_optional_non_empty(
        &mut diagnostics,
        "EVP-008",
        "/deduplicationid",
        "deduplication identity",
        event.deduplication_id.as_deref(),
        event,
    );
    validate_optional_non_empty(
        &mut diagnostics,
        "EVP-009",
        "/orderingscope",
        "ordering scope",
        event.ordering_scope.as_deref(),
        event,
    );
    validate_optional_non_empty(
        &mut diagnostics,
        "EVP-010",
        "/correlationid",
        "correlation id",
        event.correlation_id.as_deref(),
        event,
    );
    validate_optional_non_empty(
        &mut diagnostics,
        "EVP-011",
        "/causationid",
        "causation id",
        event.causation_id.as_deref(),
        event,
    );
    if Version::parse(&event.version).is_err() {
        diagnostics.push(diagnostic(
            "EVP-006",
            "/version",
            "version must be semantic",
            event,
        ));
    }
    if !is_fact_type(&event.event_type) {
        diagnostics.push(diagnostic(
            "EVP-007",
            "/type",
            "type must use a dotted past-tense fact name",
            event,
        ));
    }
    let accepted = mode == EventValidationMode::Migration || diagnostics.is_empty();
    EventValidationResult {
        accepted,
        diagnostics,
    }
}

fn validate_non_empty(
    diagnostics: &mut Vec<EventValidationDiagnostic>,
    code: &'static str,
    path: &'static str,
    field: &'static str,
    value: &str,
    event: &TraverseEvent,
) {
    if value.trim().is_empty() {
        diagnostics.push(diagnostic(code, path, field, event));
    }
}

fn validate_optional_non_empty(
    diagnostics: &mut Vec<EventValidationDiagnostic>,
    code: &'static str,
    path: &'static str,
    field: &'static str,
    value: Option<&str>,
    event: &TraverseEvent,
) {
    if value.is_none_or(|value| value.trim().is_empty()) {
        diagnostics.push(diagnostic(code, path, field, event));
    }
}

fn diagnostic(
    code: &'static str,
    path: &'static str,
    remediation: &'static str,
    event: &TraverseEvent,
) -> EventValidationDiagnostic {
    EventValidationDiagnostic {
        code,
        path,
        severity: "error",
        remediation,
        contract_id: event.event_type.clone(),
        version: event.version.clone(),
    }
}

fn is_fact_type(value: &str) -> bool {
    value.contains('.')
        && value
            .rsplit('.')
            .next()
            .is_some_and(|last| last.ends_with("ed") && !last.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::LifecycleStatus;

    fn valid_event() -> TraverseEvent {
        TraverseEvent {
            id: "evt-1".to_string(),
            source: "capability/orders".to_string(),
            event_type: "orders.order.created".to_string(),
            datacontenttype: "application/json".to_string(),
            time: "2026-07-30T00:00:00Z".to_string(),
            data: serde_json::json!({"order_id":"1"}),
            owner: "orders".to_string(),
            version: "1.0.0".to_string(),
            lifecycle_status: LifecycleStatus::Active,
            deduplication_id: Some("evt-1".to_string()),
            ordering_scope: Some("order/1".to_string()),
            correlation_id: Some("correlation-1".to_string()),
            causation_id: Some("command-1".to_string()),
            subject_id: None,
            actor_id: None,
        }
    }

    #[test]
    fn enforcement_rejects_invalid_envelope_with_stable_diagnostic() {
        let mut event = valid_event();
        event.owner.clear();
        event.version = "not-semver".to_string();
        let result = validate_event(&event, EventValidationMode::Enforcement);
        assert!(!result.accepted);
        assert_eq!(result.diagnostics[0].code, "EVP-005");
        assert_eq!(result.diagnostics[0].severity, "error");
        assert_eq!(result.diagnostics[1].code, "EVP-006");
    }

    #[test]
    fn migration_mode_reports_but_does_not_reject_legacy_gap() {
        let mut event = valid_event();
        event.event_type = "orders.order.create".to_string();
        let result = validate_event(&event, EventValidationMode::Migration);
        assert!(result.accepted);
        assert_eq!(result.diagnostics[0].code, "EVP-007");
    }

    #[test]
    fn enforcement_rejects_missing_delivery_identity() {
        let mut event = valid_event();
        event.deduplication_id = None;

        let result = validate_event(&event, EventValidationMode::Enforcement);

        assert!(!result.accepted);
        assert_eq!(result.diagnostics[0].code, "EVP-008");
        assert_eq!(result.diagnostics[0].path, "/deduplicationid");
    }

    #[test]
    fn valid_result_reports_validity() {
        let result = validate_event(&valid_event(), EventValidationMode::Enforcement);
        assert!(result.is_valid());
    }
}

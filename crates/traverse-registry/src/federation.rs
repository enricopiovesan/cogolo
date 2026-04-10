use crate::{
    CapabilityRegistry, EventRegistry, LookupScope, RegistryScope, ResolvedCapability,
    ResolvedEvent, ResolvedWorkflow, WorkflowRegistry,
};
use std::collections::{BTreeMap, BTreeSet};
use traverse_contracts::{ErrorSeverity, Lifecycle};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FederationRegistryKind {
    Capability,
    Event,
    Workflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FederationApprovalState {
    Approved,
    Draft,
    Deprecated,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FederationTrustState {
    Trusted,
    Pending,
    Blocked,
    Revoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FederationSyncStatus {
    Unknown,
    Success,
    Partial,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FederationInvocationStatus {
    Success,
    Failure,
    RetryableFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FederationConflictResolutionState {
    Open,
    Resolved,
    Escalated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FederationPeer {
    pub peer_id: String,
    pub display_name: String,
    pub trust_state: FederationTrustState,
    pub identity_fingerprint: String,
    pub sync_enabled: bool,
    pub last_sync_at: Option<String>,
    pub last_sync_status: FederationSyncStatus,
    pub visible_registry_scopes: Vec<RegistryScope>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustRecord {
    pub peer_id: String,
    pub trust_model: String,
    pub allowed_scopes: Vec<RegistryScope>,
    pub approved_spec_refs: Vec<String>,
    pub approved_at: String,
    pub revoked_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FederationPeerExport {
    pub peer: FederationPeer,
    pub trust: TrustRecord,
    pub capabilities: Vec<ResolvedCapability>,
    pub events: Vec<ResolvedEvent>,
    pub workflows: Vec<ResolvedWorkflow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FederationSyncSession {
    pub session_id: String,
    pub peer_id: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub status: FederationSyncStatus,
    pub registry_types: Vec<FederationRegistryKind>,
    pub validated_entries: usize,
    pub rejected_entries: usize,
    pub conflict_count: usize,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerRegistrySnapshot {
    pub peer_id: String,
    pub registry_type: FederationRegistryKind,
    pub entry_id: String,
    pub version: String,
    pub scope: RegistryScope,
    pub approval_state: FederationApprovalState,
    pub contract_ref: String,
    pub provenance_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossPeerTraceProvenance {
    pub trace_id: String,
    pub origin_peer_id: String,
    pub owning_peer_id: String,
    pub route_reason: String,
    pub sync_session_ref: Option<String>,
    pub response_status: FederationInvocationStatus,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FederatedInvocation {
    pub invocation_id: String,
    pub origin_peer_id: String,
    pub target_peer_id: String,
    pub capability_id: String,
    pub request_ref: String,
    pub status: FederationInvocationStatus,
    pub response_ref: Option<String>,
    pub trace_provenance: CrossPeerTraceProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictRecord {
    pub conflict_id: String,
    pub peer_ids: Vec<String>,
    pub registry_type: FederationRegistryKind,
    pub entry_key: String,
    pub conflict_reason: String,
    pub resolution_state: FederationConflictResolutionState,
    pub audit_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FederationStatusSummary {
    pub peer_count: usize,
    pub trusted_peer_count: usize,
    pub last_sync_outcome: FederationSyncStatus,
    pub sync_age: Option<String>,
    pub conflict_count: usize,
    pub blocked_entries: usize,
    pub route_failures: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FederationSyncOutcome {
    pub session: FederationSyncSession,
    pub accepted_snapshots: Vec<PeerRegistrySnapshot>,
    pub conflicts: Vec<ConflictRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FederationErrorCode {
    MissingRequiredField,
    DuplicatePeer,
    InvalidTrust,
    PeerNotFound,
    EntryValidationFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FederationError {
    pub code: FederationErrorCode,
    pub target: String,
    pub message: String,
    pub severity: ErrorSeverity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FederationFailure {
    pub errors: Vec<FederationError>,
}

#[derive(Debug, Default)]
pub struct FederationRegistry {
    peers: BTreeMap<String, FederationPeer>,
    trust_records: BTreeMap<String, TrustRecord>,
    snapshots: BTreeMap<(String, FederationRegistryKind, String, String), PeerRegistrySnapshot>,
    sync_sessions: Vec<FederationSyncSession>,
    invocations: Vec<FederatedInvocation>,
    conflicts: Vec<ConflictRecord>,
}

impl FederationRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_peer(
        &mut self,
        peer: FederationPeer,
        trust: TrustRecord,
    ) -> Result<(), FederationFailure> {
        let mut errors = Vec::new();
        if peer.peer_id.trim().is_empty() {
            errors.push(federation_error(
                FederationErrorCode::MissingRequiredField,
                "$.peer.peer_id",
                "peer_id must not be empty",
            ));
        }
        if peer.display_name.trim().is_empty() {
            errors.push(federation_error(
                FederationErrorCode::MissingRequiredField,
                "$.peer.display_name",
                "display_name must not be empty",
            ));
        }
        if peer.identity_fingerprint.trim().is_empty() {
            errors.push(federation_error(
                FederationErrorCode::MissingRequiredField,
                "$.peer.identity_fingerprint",
                "identity_fingerprint must not be empty",
            ));
        }
        if peer.peer_id != trust.peer_id {
            errors.push(federation_error(
                FederationErrorCode::InvalidTrust,
                "$.trust.peer_id",
                "trust record must reference the same peer_id as the peer",
            ));
        }
        if !peer.sync_enabled {
            errors.push(federation_error(
                FederationErrorCode::InvalidTrust,
                "$.peer.sync_enabled",
                "sync_enabled must be true for a trusted federation peer",
            ));
        }
        if !matches!(peer.trust_state, FederationTrustState::Trusted) {
            errors.push(federation_error(
                FederationErrorCode::InvalidTrust,
                "$.peer.trust_state",
                "peer trust_state must be trusted before federation registration",
            ));
        }
        if trust.allowed_scopes.is_empty() {
            errors.push(federation_error(
                FederationErrorCode::InvalidTrust,
                "$.trust.allowed_scopes",
                "allowed_scopes must not be empty",
            ));
        }
        if trust.approved_spec_refs.is_empty() {
            errors.push(federation_error(
                FederationErrorCode::InvalidTrust,
                "$.trust.approved_spec_refs",
                "approved_spec_refs must not be empty",
            ));
        }
        if !errors.is_empty() {
            return Err(FederationFailure { errors });
        }

        match self.peers.get(&peer.peer_id) {
            Some(existing) if existing == &peer && self.trust_records.get(&peer.peer_id) == Some(&trust) => {
                Ok(())
            }
            Some(_) => Err(FederationFailure {
                errors: vec![federation_error(
                    FederationErrorCode::DuplicatePeer,
                    "$.peer.peer_id",
                    "a different federation peer is already registered with this peer_id",
                )],
            }),
            None => {
                self.trust_records.insert(peer.peer_id.clone(), trust);
                self.peers.insert(peer.peer_id.clone(), peer);
                Ok(())
            }
        }
    }

    #[must_use]
    pub fn list_peers(&self) -> Vec<FederationPeer> {
        let mut peers = self.peers.values().cloned().collect::<Vec<_>>();
        peers.sort_by(|left, right| left.peer_id.cmp(&right.peer_id));
        peers
    }

    #[must_use]
    pub fn conflicts(&self) -> &[ConflictRecord] {
        &self.conflicts
    }

    #[must_use]
    pub fn sync_sessions(&self) -> &[FederationSyncSession] {
        &self.sync_sessions
    }

    #[must_use]
    pub fn invocations(&self) -> &[FederatedInvocation] {
        &self.invocations
    }

    #[must_use]
    pub fn status_summary(&self) -> FederationStatusSummary {
        let trusted_peer_count = self
            .peers
            .values()
            .filter(|peer| peer.trust_state == FederationTrustState::Trusted)
            .count();
        let last_session = self.sync_sessions.last();
        FederationStatusSummary {
            peer_count: self.peers.len(),
            trusted_peer_count,
            last_sync_outcome: last_session
                .map(|session| session.status)
                .unwrap_or(FederationSyncStatus::Unknown),
            sync_age: last_session.and_then(|session| session.finished_at.clone()),
            conflict_count: self.conflicts.len(),
            blocked_entries: self.sync_sessions.iter().map(|session| session.rejected_entries).sum(),
            route_failures: self
                .invocations
                .iter()
                .filter(|invocation| {
                    matches!(
                        invocation.status,
                        FederationInvocationStatus::Failure
                            | FederationInvocationStatus::RetryableFailure
                    )
                })
                .count(),
        }
    }

    pub fn sync_peer(
        &mut self,
        export: FederationPeerExport,
        capabilities: &CapabilityRegistry,
        events: &EventRegistry,
        workflows: &WorkflowRegistry,
        started_at: &str,
        finished_at: &str,
        evidence_ref: &str,
    ) -> Result<FederationSyncOutcome, FederationFailure> {
        let mut errors = Vec::new();
        if started_at.trim().is_empty() {
            errors.push(federation_error(
                FederationErrorCode::MissingRequiredField,
                "$.started_at",
                "started_at must not be empty",
            ));
        }
        if finished_at.trim().is_empty() {
            errors.push(federation_error(
                FederationErrorCode::MissingRequiredField,
                "$.finished_at",
                "finished_at must not be empty",
            ));
        }
        if evidence_ref.trim().is_empty() {
            errors.push(federation_error(
                FederationErrorCode::MissingRequiredField,
                "$.evidence_ref",
                "evidence_ref must not be empty",
            ));
        }
        if export.peer.peer_id != export.trust.peer_id {
            errors.push(federation_error(
                FederationErrorCode::InvalidTrust,
                "$.trust.peer_id",
                "export trust record must match the exporting peer id",
            ));
        }

        let Some(registered_peer) = self.peers.get(&export.peer.peer_id) else {
            errors.push(federation_error(
                FederationErrorCode::PeerNotFound,
                "$.peer.peer_id",
                "peer must be registered before it can be synced",
            ));
            return Err(FederationFailure { errors });
        };
        let Some(registered_trust) = self.trust_records.get(&export.peer.peer_id) else {
            errors.push(federation_error(
                FederationErrorCode::InvalidTrust,
                "$.trust.peer_id",
                "peer is missing its approved trust record",
            ));
            return Err(FederationFailure { errors });
        };

        if registered_peer != &export.peer || registered_trust != &export.trust {
            errors.push(federation_error(
                FederationErrorCode::InvalidTrust,
                "$.peer",
                "exported peer metadata must match the registered trusted peer",
            ));
        }
        if !registered_peer.sync_enabled {
            errors.push(federation_error(
                FederationErrorCode::InvalidTrust,
                "$.peer.sync_enabled",
                "sync is disabled for this peer",
            ));
        }
        if registered_peer.trust_state != FederationTrustState::Trusted {
            errors.push(federation_error(
                FederationErrorCode::InvalidTrust,
                "$.peer.trust_state",
                "only trusted peers can participate in federation sync",
            ));
        }
        if !errors.is_empty() {
            return Err(FederationFailure { errors });
        }

        let mut accepted_snapshots = Vec::new();
        let mut conflict_records = Vec::new();

        for capability in &export.capabilities {
            if let Some(snapshot) = validate_capability_snapshot(
                &export.peer,
                &export.trust,
                capabilities,
                capability,
                evidence_ref,
                &mut conflict_records,
            ) {
                accepted_snapshots.push(snapshot);
            }
        }
        for event in &export.events {
            if let Some(snapshot) = validate_event_snapshot(
                &export.peer,
                &export.trust,
                events,
                event,
                evidence_ref,
                &mut conflict_records,
            ) {
                accepted_snapshots.push(snapshot);
            }
        }
        for workflow in &export.workflows {
            if let Some(snapshot) = validate_workflow_snapshot(
                &export.peer,
                &export.trust,
                workflows,
                workflow,
                evidence_ref,
                &mut conflict_records,
            ) {
                accepted_snapshots.push(snapshot);
            }
        }

        for snapshot in &accepted_snapshots {
            let key = (
                snapshot.peer_id.clone(),
                snapshot.registry_type,
                snapshot.entry_id.clone(),
                snapshot.version.clone(),
            );
            self.snapshots.insert(key, snapshot.clone());
        }
        self.conflicts.extend(conflict_records.clone());

        let status = if accepted_snapshots.is_empty() && conflict_records.is_empty() {
            FederationSyncStatus::Failed
        } else if conflict_records.is_empty() {
            FederationSyncStatus::Success
        } else {
            FederationSyncStatus::Partial
        };

        let session = FederationSyncSession {
            session_id: format!("sync_{}_{}", export.peer.peer_id, self.sync_sessions.len() + 1),
            peer_id: export.peer.peer_id.clone(),
            started_at: started_at.to_string(),
            finished_at: Some(finished_at.to_string()),
            status,
            registry_types: synced_registry_types(&accepted_snapshots),
            validated_entries: accepted_snapshots.len(),
            rejected_entries: conflict_records.len(),
            conflict_count: conflict_records.len(),
            evidence_ref: evidence_ref.to_string(),
        };

        if let Some(peer) = self.peers.get_mut(&export.peer.peer_id) {
            peer.last_sync_at = Some(finished_at.to_string());
            peer.last_sync_status = status;
        }

        self.sync_sessions.push(session.clone());

        Ok(FederationSyncOutcome {
            session,
            accepted_snapshots,
            conflicts: conflict_records,
        })
    }

    pub fn route_capability_invocation(
        &mut self,
        origin_peer_id: &str,
        capability_id: &str,
        version: &str,
        request_ref: &str,
        available_peer_ids: &BTreeSet<String>,
        routed_at: &str,
        evidence_ref: &str,
    ) -> Result<FederatedInvocation, FederationFailure> {
        let mut errors = Vec::new();
        if origin_peer_id.trim().is_empty() {
            errors.push(federation_error(
                FederationErrorCode::MissingRequiredField,
                "$.origin_peer_id",
                "origin_peer_id must not be empty",
            ));
        }
        if capability_id.trim().is_empty() {
            errors.push(federation_error(
                FederationErrorCode::MissingRequiredField,
                "$.capability_id",
                "capability_id must not be empty",
            ));
        }
        if version.trim().is_empty() {
            errors.push(federation_error(
                FederationErrorCode::MissingRequiredField,
                "$.version",
                "version must not be empty",
            ));
        }
        if request_ref.trim().is_empty() {
            errors.push(federation_error(
                FederationErrorCode::MissingRequiredField,
                "$.request_ref",
                "request_ref must not be empty",
            ));
        }
        if routed_at.trim().is_empty() {
            errors.push(federation_error(
                FederationErrorCode::MissingRequiredField,
                "$.routed_at",
                "routed_at must not be empty",
            ));
        }
        if evidence_ref.trim().is_empty() {
            errors.push(federation_error(
                FederationErrorCode::MissingRequiredField,
                "$.evidence_ref",
                "evidence_ref must not be empty",
            ));
        }
        if !self.peers.contains_key(origin_peer_id) {
            errors.push(federation_error(
                FederationErrorCode::PeerNotFound,
                "$.origin_peer_id",
                "origin peer must be registered before routing",
            ));
        }
        if !errors.is_empty() {
            return Err(FederationFailure { errors });
        }

        let origin_peer = self.peers.get(origin_peer_id).expect("validated above");
        let trust = self
            .trust_records
            .get(origin_peer_id)
            .expect("validated above");

        let mut candidate: Option<(String, PeerRegistrySnapshot)> = None;
        for snapshot in self.snapshots.values() {
            if snapshot.registry_type != FederationRegistryKind::Capability {
                continue;
            }
            if snapshot.entry_id != capability_id || snapshot.version != version {
                continue;
            }
            if !scope_is_visible(snapshot.scope, trust, origin_peer) {
                continue;
            }
            match &candidate {
                Some((current_peer, _)) if snapshot.peer_id >= *current_peer => {}
                _ => {
                    candidate = Some((snapshot.peer_id.clone(), snapshot.clone()));
                }
            }
        }

        let Some((target_peer_id, target_snapshot)) = candidate else {
            return Err(FederationFailure {
                errors: vec![federation_error(
                    FederationErrorCode::EntryValidationFailed,
                    "$.capability_id",
                    "no synchronized owning peer was found for the requested capability",
                )],
            });
        };

        let available = available_peer_ids.contains(&target_peer_id);
        let sync_session_ref = self
            .sync_sessions
            .iter()
            .rev()
            .find(|session| session.peer_id == target_peer_id)
            .map(|session| session.evidence_ref.clone());
        let trace_id = format!("trace_{}_{}_{}", origin_peer_id, capability_id, version);
        let invocation_id = format!("invocation_{}_{}_{}", origin_peer_id, capability_id, version);
        let (status, response_ref, route_reason) = if available {
            (
                FederationInvocationStatus::Success,
                Some(format!(
                    "response://{}/{}/{}",
                    target_peer_id, capability_id, version
                )),
                format!(
                    "routed to owning peer {} for synchronized capability snapshot",
                    target_peer_id
                ),
            )
        } else {
            (
                FederationInvocationStatus::RetryableFailure,
                None,
                format!(
                    "owning peer {} is not currently reachable for invocation",
                    target_peer_id
                ),
            )
        };

        let invocation = FederatedInvocation {
            invocation_id,
            origin_peer_id: origin_peer_id.to_string(),
            target_peer_id: target_peer_id.clone(),
            capability_id: capability_id.to_string(),
            request_ref: request_ref.to_string(),
            status,
            response_ref,
            trace_provenance: CrossPeerTraceProvenance {
                trace_id,
                origin_peer_id: origin_peer_id.to_string(),
                owning_peer_id: target_snapshot.peer_id,
                route_reason,
                sync_session_ref,
                response_status: status,
                evidence_ref: evidence_ref.to_string(),
            },
        };
        self.invocations.push(invocation.clone());
        Ok(invocation)
    }
}

pub fn export_peer_state(
    peer: FederationPeer,
    trust: TrustRecord,
    capabilities: &CapabilityRegistry,
    events: &EventRegistry,
    workflows: &WorkflowRegistry,
) -> FederationPeerExport {
    FederationPeerExport {
        peer,
        trust,
        capabilities: capabilities.graph_entries(),
        events: events.graph_entries(),
        workflows: workflows.graph_entries(),
    }
}

fn validate_capability_snapshot(
    peer: &FederationPeer,
    trust: &TrustRecord,
    capabilities: &CapabilityRegistry,
    export: &ResolvedCapability,
    evidence_ref: &str,
    conflicts: &mut Vec<ConflictRecord>,
) -> Option<PeerRegistrySnapshot> {
    if !scope_is_allowed(export.record.scope, trust, peer) {
        conflicts.push(build_conflict_record(
            peer.peer_id.as_str(),
            FederationRegistryKind::Capability,
            &export.record.id,
            &export.record.version,
            "peer trust does not authorize the exported scope",
            evidence_ref,
        ));
        return None;
    }

    let lookup_scope = lookup_scope_for(export.record.scope);
    let Some(local) = capabilities.find_exact(lookup_scope, &export.record.id, &export.record.version) else {
        conflicts.push(build_conflict_record(
            peer.peer_id.as_str(),
            FederationRegistryKind::Capability,
            &export.record.id,
            &export.record.version,
            "local approved registry is missing the exported capability",
            evidence_ref,
        ));
        return None;
    };

    if local != *export {
        conflicts.push(build_conflict_record(
            peer.peer_id.as_str(),
            FederationRegistryKind::Capability,
            &export.record.id,
            &export.record.version,
            "local capability record differs from the exported peer record",
            evidence_ref,
        ));
        return None;
    }

    Some(build_snapshot(
        peer,
        FederationRegistryKind::Capability,
        &export.record.id,
        &export.record.version,
        export.record.scope,
        export.record.lifecycle.clone(),
        &export.record.contract_path,
        &format!(
            "{:?}:{}:{}",
            export.record.provenance.source, export.record.provenance.author, export.record.provenance.created_at
        ),
    ))
}

fn validate_event_snapshot(
    peer: &FederationPeer,
    trust: &TrustRecord,
    events: &EventRegistry,
    export: &ResolvedEvent,
    evidence_ref: &str,
    conflicts: &mut Vec<ConflictRecord>,
) -> Option<PeerRegistrySnapshot> {
    if !scope_is_allowed(export.record.scope, trust, peer) {
        conflicts.push(build_conflict_record(
            peer.peer_id.as_str(),
            FederationRegistryKind::Event,
            &export.record.id,
            &export.record.version,
            "peer trust does not authorize the exported scope",
            evidence_ref,
        ));
        return None;
    }

    let lookup_scope = lookup_scope_for(export.record.scope);
    let Some(local) = events.find_exact(lookup_scope, &export.record.id, &export.record.version) else {
        conflicts.push(build_conflict_record(
            peer.peer_id.as_str(),
            FederationRegistryKind::Event,
            &export.record.id,
            &export.record.version,
            "local approved registry is missing the exported event",
            evidence_ref,
        ));
        return None;
    };

    if local != *export {
        conflicts.push(build_conflict_record(
            peer.peer_id.as_str(),
            FederationRegistryKind::Event,
            &export.record.id,
            &export.record.version,
            "local event record differs from the exported peer record",
            evidence_ref,
        ));
        return None;
    }

    Some(build_snapshot(
        peer,
        FederationRegistryKind::Event,
        &export.record.id,
        &export.record.version,
        export.record.scope,
        export.record.lifecycle.clone(),
        &export.record.contract_path,
        &format!(
            "{:?}:{}:{}",
            export.record.provenance.source, export.record.provenance.author, export.record.provenance.created_at
        ),
    ))
}

fn validate_workflow_snapshot(
    peer: &FederationPeer,
    trust: &TrustRecord,
    workflows: &WorkflowRegistry,
    export: &ResolvedWorkflow,
    evidence_ref: &str,
    conflicts: &mut Vec<ConflictRecord>,
) -> Option<PeerRegistrySnapshot> {
    if !scope_is_allowed(export.record.scope, trust, peer) {
        conflicts.push(build_conflict_record(
            peer.peer_id.as_str(),
            FederationRegistryKind::Workflow,
            &export.record.id,
            &export.record.version,
            "peer trust does not authorize the exported scope",
            evidence_ref,
        ));
        return None;
    }

    let lookup_scope = lookup_scope_for(export.record.scope);
    let Some(local) = workflows.find_exact(lookup_scope, &export.record.id, &export.record.version) else {
        conflicts.push(build_conflict_record(
            peer.peer_id.as_str(),
            FederationRegistryKind::Workflow,
            &export.record.id,
            &export.record.version,
            "local approved registry is missing the exported workflow",
            evidence_ref,
        ));
        return None;
    };

    if local != *export {
        conflicts.push(build_conflict_record(
            peer.peer_id.as_str(),
            FederationRegistryKind::Workflow,
            &export.record.id,
            &export.record.version,
            "local workflow record differs from the exported peer record",
            evidence_ref,
        ));
        return None;
    }

    Some(build_snapshot(
        peer,
        FederationRegistryKind::Workflow,
        &export.record.id,
        &export.record.version,
        export.record.scope,
        export.record.lifecycle.clone(),
        &export.record.workflow_path,
        &format!(
            "{}:{}:{}",
            export.record.governing_spec, export.record.validator_version, export.record.registered_at
        ),
    ))
}

fn build_snapshot(
    peer: &FederationPeer,
    registry_type: FederationRegistryKind,
    entry_id: &str,
    version: &str,
    scope: RegistryScope,
    lifecycle: Lifecycle,
    contract_ref: &str,
    provenance_ref: &str,
) -> PeerRegistrySnapshot {
    PeerRegistrySnapshot {
        peer_id: peer.peer_id.clone(),
        registry_type,
        entry_id: entry_id.to_string(),
        version: version.to_string(),
        scope,
        approval_state: approval_state_from_lifecycle(&lifecycle),
        contract_ref: contract_ref.to_string(),
        provenance_ref: provenance_ref.to_string(),
    }
}

fn build_conflict_record(
    peer_id: &str,
    registry_type: FederationRegistryKind,
    entry_id: &str,
    version: &str,
    reason: &str,
    audit_ref: &str,
) -> ConflictRecord {
    ConflictRecord {
        conflict_id: format!("conflict_{}_{}_{}", peer_id, entry_id, version),
        peer_ids: vec![peer_id.to_string()],
        registry_type,
        entry_key: format!("{registry_type:?}:{entry_id}@{version}"),
        conflict_reason: reason.to_string(),
        resolution_state: FederationConflictResolutionState::Open,
        audit_ref: audit_ref.to_string(),
    }
}

fn approval_state_from_lifecycle(lifecycle: &Lifecycle) -> FederationApprovalState {
    match lifecycle {
        Lifecycle::Draft => FederationApprovalState::Draft,
        Lifecycle::Active => FederationApprovalState::Approved,
        Lifecycle::Deprecated => FederationApprovalState::Deprecated,
        Lifecycle::Retired | Lifecycle::Archived => FederationApprovalState::Rejected,
    }
}

fn scope_is_allowed(
    scope: RegistryScope,
    trust: &TrustRecord,
    peer: &FederationPeer,
) -> bool {
    trust.allowed_scopes.contains(&scope) && peer.visible_registry_scopes.contains(&scope)
}

fn scope_is_visible(scope: RegistryScope, trust: &TrustRecord, peer: &FederationPeer) -> bool {
    scope_is_allowed(scope, trust, peer)
}

fn lookup_scope_for(scope: RegistryScope) -> LookupScope {
    match scope {
        RegistryScope::Public => LookupScope::PublicOnly,
        RegistryScope::Private => LookupScope::PreferPrivate,
    }
}

fn synced_registry_types(snapshots: &[PeerRegistrySnapshot]) -> Vec<FederationRegistryKind> {
    let mut kinds = BTreeSet::new();
    for snapshot in snapshots {
        kinds.insert(snapshot.registry_type);
    }
    kinds.into_iter().collect()
}

fn federation_error(code: FederationErrorCode, target: &str, message: &str) -> FederationError {
    FederationError {
        code,
        target: target.to_string(),
        message: message.to_string(),
        severity: ErrorSeverity::Error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ArtifactDigests, BinaryFormat, BinaryReference, CapabilityArtifactRecord,
        CapabilityRegistration, CapabilityRegistry, ComposabilityMetadata, CompositionKind,
        CompositionPattern, EventRegistry, ImplementationKind, RegistryProvenance,
        RegistryScope, SourceKind, SourceReference, WorkflowRegistry, WorkflowRegistration,
        WorkflowDefinition, WorkflowNode, WorkflowNodeInput, WorkflowNodeOutput,
        export_peer_state,
    };
    use serde_json::json;
    use traverse_contracts::{
        CapabilityContract, Entrypoint, EntrypointKind, EventClassification, EventContract,
        EventPayload, EventProvenance, EventProvenanceSource, EventReference, EventType,
        Lifecycle, Owner, PayloadCompatibility, SchemaContainer, SideEffect, SideEffectKind,
    };

    #[test]
    fn registers_trusted_peer_and_reports_status() {
        let mut federation = FederationRegistry::new();
        let peer = peer("peer-a", "Peer A");
        let trust = trust("peer-a", vec![RegistryScope::Public, RegistryScope::Private]);

        federation
            .register_peer(peer.clone(), trust.clone())
            .expect("peer should register");

        assert_eq!(federation.list_peers(), vec![peer]);
        let summary = federation.status_summary();
        assert_eq!(summary.peer_count, 1);
        assert_eq!(summary.trusted_peer_count, 1);
        assert_eq!(summary.last_sync_outcome, FederationSyncStatus::Unknown);
    }

    #[test]
    fn syncs_peer_export_and_routes_invocation_to_owner() {
        let mut local_capabilities = CapabilityRegistry::new();
        let mut local_events = EventRegistry::new();
        let mut local_workflows = WorkflowRegistry::new();
        seed_capabilities(&mut local_capabilities);
        seed_events(&mut local_events);
        seed_workflows(&mut local_workflows, &local_capabilities);

        let peer = peer("peer-b", "Peer B");
        let trust = trust("peer-b", vec![RegistryScope::Public, RegistryScope::Private]);
        let export = export_peer_state(
            peer.clone(),
            trust.clone(),
            &local_capabilities,
            &local_events,
            &local_workflows,
        );

        let mut federation = FederationRegistry::new();
        federation
            .register_peer(peer, trust)
            .expect("peer should register");

        let outcome = federation
            .sync_peer(
                export,
                &local_capabilities,
                &local_events,
                &local_workflows,
                "2026-04-09T20:00:00Z",
                "2026-04-09T20:01:00Z",
                "evidence:sync-001",
            )
            .expect("sync should pass");

        assert_eq!(outcome.session.status, FederationSyncStatus::Success);
        assert!(!outcome.accepted_snapshots.is_empty());
        assert!(outcome.conflicts.is_empty());

        let origin_peer = self::peer("peer-a", "Peer A");
        let origin_trust = self::trust("peer-a", vec![RegistryScope::Public, RegistryScope::Private]);
        federation
            .register_peer(origin_peer, origin_trust)
            .expect("origin peer should register");
        let available = BTreeSet::from([String::from("peer-b")]);
        let invocation = federation
            .route_capability_invocation(
                "peer-a",
                "federation.capability.echo",
                "1.0.0",
                "request:001",
                &available,
                "2026-04-09T20:02:00Z",
                "evidence:route-001",
            )
            .expect("invocation should route");

        assert_eq!(invocation.status, FederationInvocationStatus::Success);
        assert_eq!(invocation.target_peer_id, "peer-b");
        assert_eq!(invocation.trace_provenance.origin_peer_id, "peer-a");
        assert_eq!(invocation.trace_provenance.owning_peer_id, "peer-b");
        assert_eq!(invocation.response_ref.as_deref(), Some("response://peer-b/federation.capability.echo/1.0.0"));
    }

    #[test]
    fn sync_reports_conflicts_for_divergent_private_entries() {
        let mut local_capabilities = CapabilityRegistry::new();
        let mut local_events = EventRegistry::new();
        let mut local_workflows = WorkflowRegistry::new();
        seed_capabilities(&mut local_capabilities);
        seed_events(&mut local_events);
        seed_workflows(&mut local_workflows, &local_capabilities);

        let mut remote_capabilities = CapabilityRegistry::new();
        let mut altered_contract = capability_contract();
        altered_contract.summary = "divergent export".to_string();
        remote_capabilities
            .register(capability_registration(RegistryScope::Private, altered_contract))
            .expect("remote capability should register");
        seed_events(&mut local_events);
        seed_workflows(&mut local_workflows, &local_capabilities);

        let peer = peer("peer-c", "Peer C");
        let trust = trust("peer-c", vec![RegistryScope::Public]);
        let export = export_peer_state(
            peer.clone(),
            trust.clone(),
            &remote_capabilities,
            &local_events,
            &local_workflows,
        );

        let mut federation = FederationRegistry::new();
        federation
            .register_peer(peer, trust)
            .expect("peer should register");

        let outcome = federation
            .sync_peer(
                export,
                &local_capabilities,
                &local_events,
                &local_workflows,
                "2026-04-09T20:10:00Z",
                "2026-04-09T20:11:00Z",
                "evidence:sync-002",
            )
            .expect("sync should report conflicts rather than failing");

        assert_eq!(outcome.session.status, FederationSyncStatus::Partial);
        assert!(!outcome.conflicts.is_empty());
        assert_eq!(federation.conflicts().len(), outcome.conflicts.len());
    }

    fn peer(peer_id: &str, display_name: &str) -> FederationPeer {
        FederationPeer {
            peer_id: peer_id.to_string(),
            display_name: display_name.to_string(),
            trust_state: FederationTrustState::Trusted,
            identity_fingerprint: format!("fingerprint:{peer_id}"),
            sync_enabled: true,
            last_sync_at: None,
            last_sync_status: FederationSyncStatus::Unknown,
            visible_registry_scopes: vec![RegistryScope::Public, RegistryScope::Private],
        }
    }

    fn trust(peer_id: &str, scopes: Vec<RegistryScope>) -> TrustRecord {
        TrustRecord {
            peer_id: peer_id.to_string(),
            trust_model: "shared-api-token".to_string(),
            allowed_scopes: scopes,
            approved_spec_refs: vec!["026-federation-registry-routing".to_string()],
            approved_at: "2026-04-09T19:30:00Z".to_string(),
            revoked_at: None,
        }
    }

    fn seed_capabilities(registry: &mut CapabilityRegistry) {
        registry
            .register(capability_registration(
                RegistryScope::Public,
                capability_contract(),
            ))
            .expect("capability should register");
        registry
            .register(capability_registration(
                RegistryScope::Private,
                private_capability_contract(),
            ))
            .expect("private capability should register");
    }

    fn seed_events(registry: &mut EventRegistry) {
        registry
            .register(event_registration(
                RegistryScope::Public,
                event_contract(),
            ))
            .expect("event should register");
    }

    fn seed_workflows(registry: &mut WorkflowRegistry, capabilities: &CapabilityRegistry) {
        registry
            .register(
                capabilities,
                workflow_registration(RegistryScope::Public, workflow_definition()),
            )
            .expect("workflow should register");
    }

    fn capability_contract() -> CapabilityContract {
        CapabilityContract {
            kind: "capability_contract".to_string(),
            schema_version: "1.0.0".to_string(),
            id: "federation.capability.echo".to_string(),
            namespace: "federation.capability".to_string(),
            name: "echo".to_string(),
            version: "1.0.0".to_string(),
            lifecycle: Lifecycle::Active,
            owner: Owner {
                team: "platform".to_string(),
                contact: "platform@example.com".to_string(),
            },
            summary: "Echo a federated capability call.".to_string(),
            description: "End-to-end federation test capability.".to_string(),
            inputs: SchemaContainer {
                schema: json!({"type":"object"}),
            },
            outputs: SchemaContainer {
                schema: json!({"type":"object"}),
            },
            preconditions: vec![],
            postconditions: vec![],
            side_effects: vec![SideEffect {
                kind: SideEffectKind::EventEmission,
                description: "Emit routing evidence for federation sync.".to_string(),
            }],
            emits: vec![EventReference {
                event_id: "federation.event.routed".to_string(),
                version: "1.0.0".to_string(),
            }],
            consumes: vec![],
            permissions: vec![],
            execution: traverse_contracts::Execution {
                binary_format: traverse_contracts::BinaryFormat::Wasm,
                entrypoint: Entrypoint {
                    kind: EntrypointKind::WasiCommand,
                    command: "echo".to_string(),
                },
                preferred_targets: vec![traverse_contracts::ExecutionTarget::Local],
                constraints: traverse_contracts::ExecutionConstraints {
                    host_api_access: traverse_contracts::HostApiAccess::None,
                    filesystem_access: traverse_contracts::FilesystemAccess::None,
                    network_access: traverse_contracts::NetworkAccess::Forbidden,
                },
            },
            policies: vec![],
            dependencies: vec![],
            provenance: traverse_contracts::Provenance {
                source: traverse_contracts::ProvenanceSource::Greenfield,
                author: "enricopiovesan".to_string(),
                created_at: "2026-04-09T19:00:00Z".to_string(),
                spec_ref: Some("026-federation-registry-routing".to_string()),
                adr_refs: vec![],
                exception_refs: vec![],
            },
            evidence: vec![],
        }
    }

    fn private_capability_contract() -> CapabilityContract {
        let mut contract = capability_contract();
        contract.id = "federation.capability.private-echo".to_string();
        contract.name = "private-echo".to_string();
        contract.summary = "Private federated echo.".to_string();
        contract
    }

    fn event_contract() -> EventContract {
        EventContract {
            kind: "event_contract".to_string(),
            schema_version: "1.0.0".to_string(),
            id: "federation.event.routed".to_string(),
            namespace: "federation.event".to_string(),
            name: "routed".to_string(),
            version: "1.0.0".to_string(),
            lifecycle: Lifecycle::Active,
            owner: Owner {
                team: "platform".to_string(),
                contact: "platform@example.com".to_string(),
            },
            summary: "A federation routing event.".to_string(),
            description: "End-to-end federation event.".to_string(),
            payload: EventPayload {
                schema: json!({"type":"object"}),
                compatibility: PayloadCompatibility::BackwardCompatible,
            },
            classification: EventClassification {
                domain: "federation".to_string(),
                bounded_context: "registry".to_string(),
                event_type: EventType::System,
                tags: vec!["federation".to_string()],
            },
            publishers: vec![traverse_contracts::CapabilityReference {
                capability_id: "federation.capability.echo".to_string(),
                version: "1.0.0".to_string(),
            }],
            subscribers: vec![traverse_contracts::CapabilityReference {
                capability_id: "federation.capability.private-echo".to_string(),
                version: "1.0.0".to_string(),
            }],
            policies: vec![],
            tags: vec!["federation".to_string()],
            provenance: EventProvenance {
                source: EventProvenanceSource::Greenfield,
                author: "enricopiovesan".to_string(),
                created_at: "2026-04-09T19:00:00Z".to_string(),
            },
            evidence: vec![],
        }
    }

    fn workflow_definition() -> WorkflowDefinition {
        WorkflowDefinition {
            kind: "workflow_definition".to_string(),
            schema_version: "1.0.0".to_string(),
            id: "federation.workflow.route".to_string(),
            name: "route".to_string(),
            version: "1.0.0".to_string(),
            lifecycle: Lifecycle::Active,
            owner: Owner {
                team: "platform".to_string(),
                contact: "platform@example.com".to_string(),
            },
            summary: "A federated routing workflow.".to_string(),
            inputs: SchemaContainer {
                schema: json!({"type":"object"}),
            },
            outputs: SchemaContainer {
                schema: json!({"type":"object"}),
            },
            nodes: vec![WorkflowNode {
                node_id: "route-node".to_string(),
                capability_id: "federation.capability.echo".to_string(),
                capability_version: "1.0.0".to_string(),
                input: WorkflowNodeInput {
                    from_workflow_input: vec!["request".to_string()],
                },
                output: WorkflowNodeOutput {
                    to_workflow_state: vec!["response".to_string()],
                },
            }],
            edges: vec![],
            start_node: "route-node".to_string(),
            terminal_nodes: vec!["route-node".to_string()],
            tags: vec!["federation".to_string()],
            governing_spec: "007-workflow-registry-traversal".to_string(),
        }
    }

    fn capability_registration(
        scope: RegistryScope,
        contract: CapabilityContract,
    ) -> CapabilityRegistration {
        CapabilityRegistration {
            scope,
            contract_path: format!(
                "registry/{}/{}/{}{}",
                scope_name(scope),
                contract.id,
                contract.version,
                "/contract.json"
            ),
            artifact: CapabilityArtifactRecord {
                artifact_ref: format!("artifact:{}:{}", contract.name, contract.version),
                implementation_kind: ImplementationKind::Executable,
                source: SourceReference {
                    kind: SourceKind::Git,
                    location: format!("https://example.invalid/{}", contract.name),
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
                    created_at: "2026-04-09T19:00:00Z".to_string(),
                },
            },
            registered_at: "2026-04-09T19:00:00Z".to_string(),
            tags: vec!["federation".to_string()],
            composability: ComposabilityMetadata {
                kind: CompositionKind::Atomic,
                patterns: vec![CompositionPattern::Sequential],
                provides: vec!["federation".to_string()],
                requires: vec!["registry".to_string()],
            },
            governing_spec: "005-capability-registry".to_string(),
            validator_version: "registry-test".to_string(),
            contract,
        }
    }

    fn event_registration(scope: RegistryScope, contract: EventContract) -> crate::EventRegistration {
        crate::EventRegistration {
            scope,
            contract,
            contract_path: format!(
                "registry/{}/{}/{}{}",
                scope_name(scope),
                "federation.event.routed",
                "1.0.0",
                "/contract.json"
            ),
            registered_at: "2026-04-09T19:00:00Z".to_string(),
            governing_spec: "011-event-registry".to_string(),
            validator_version: "registry-test".to_string(),
        }
    }

    fn workflow_registration(
        scope: RegistryScope,
        definition: WorkflowDefinition,
    ) -> WorkflowRegistration {
        WorkflowRegistration {
            scope,
            definition,
            workflow_path: "registry/public/federation.workflow.route/1.0.0/workflow.json".to_string(),
            registered_at: "2026-04-09T19:00:00Z".to_string(),
            validator_version: "registry-test".to_string(),
        }
    }

    fn scope_name(scope: RegistryScope) -> &'static str {
        match scope {
            RegistryScope::Public => "public",
            RegistryScope::Private => "private",
        }
    }
}

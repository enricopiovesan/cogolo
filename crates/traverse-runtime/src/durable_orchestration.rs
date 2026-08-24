//! Host-owned durable orchestration controls (spec `111-durable-dynamic-orchestration`).
//!
//! This module deliberately stores only governing identities and redacted execution
//! state. Hosts own persistence and checkpoint authentication; an invalid checkpoint
//! is never partially recovered or replanned.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const GOVERNING_SPEC: &str = "111-durable-dynamic-orchestration";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoverningSnapshots {
    pub proposal_digest: String,
    pub manifest_digest: String,
    pub registry_digest: String,
    pub policy_digest: String,
    pub authorization_digest: String,
}

impl GoverningSnapshots {
    #[must_use]
    pub fn is_complete(&self) -> bool {
        [
            &self.proposal_digest,
            &self.manifest_digest,
            &self.registry_digest,
            &self.policy_digest,
            &self.authorization_digest,
        ]
        .iter()
        .all(|digest| !digest.trim().is_empty())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionLease {
    pub owner_id: String,
    pub fencing_token: u64,
    /// Host-provided, comparable logical expiry; no wall clock is read here.
    pub expires_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableWait {
    pub kind: DurableWaitKind,
    pub owner_id: String,
    pub deadline: u64,
    pub cancellation_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DurableWaitKind {
    Event,
    Schedule,
    HumanApproval,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub retryable: bool,
    pub max_attempts: u32,
    pub backoff_units: u64,
    pub budget_units: u64,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompensationStep {
    pub capability_id: String,
    pub authorization_digest: String,
    pub order: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableCheckpoint {
    pub governing_spec: String,
    pub execution_id: String,
    pub snapshots: GoverningSnapshots,
    pub lease: ExecutionLease,
    pub completed_node_ids: Vec<String>,
    pub wait: Option<DurableWait>,
    pub retry: Option<RetryPolicy>,
    pub compensation: Vec<CompensationStep>,
    /// Host-authenticated opaque tag; never raw credentials or payloads.
    pub authentication_tag: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurableOrchestrationErrorCode {
    InvalidCheckpoint,
    AuthenticationFailed,
    SnapshotMismatch,
    StaleLease,
    WaitExpired,
    Cancelled,
    RetryNotDeclared,
    RetryBudgetExhausted,
    CompensationUnauthorized,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableOrchestrationError {
    pub code: DurableOrchestrationErrorCode,
    pub message: String,
}

type Result<T> = std::result::Result<T, DurableOrchestrationError>;

/// Host boundary for authenticated persistence. Implementations must keep signing
/// material outside `DurableCheckpoint` and atomically replace by fencing token.
pub trait CheckpointStore {
    /// # Errors
    ///
    /// Returns a stable persistence failure when the host cannot atomically save.
    fn save(&mut self, checkpoint: DurableCheckpoint) -> Result<()>;
    /// # Errors
    ///
    /// Returns a stable host storage failure when loading cannot complete.
    fn load(&self, execution_id: &str) -> Result<Option<DurableCheckpoint>>;
    fn verify(&self, checkpoint: &DurableCheckpoint) -> bool;
}

/// In-memory store for deterministic conformance tests. Production hosts provide
/// authenticated durable storage through [`CheckpointStore`].
#[derive(Default)]
pub struct MemoryCheckpointStore {
    checkpoints: BTreeMap<String, DurableCheckpoint>,
}

impl CheckpointStore for MemoryCheckpointStore {
    fn save(&mut self, checkpoint: DurableCheckpoint) -> Result<()> {
        self.checkpoints
            .insert(checkpoint.execution_id.clone(), checkpoint);
        Ok(())
    }
    fn load(&self, execution_id: &str) -> Result<Option<DurableCheckpoint>> {
        Ok(self.checkpoints.get(execution_id).cloned())
    }
    fn verify(&self, checkpoint: &DurableCheckpoint) -> bool {
        checkpoint.authentication_tag == authenticated_tag(checkpoint)
    }
}

/// Constructs and persists a secret-free checkpoint before a wait or effect is
/// reported. The caller supplies no authentication tag; it is derived by the host
/// store's test implementation here and production stores replace it atomically.
///
/// # Errors
///
/// Returns an error for an incomplete checkpoint or a host persistence failure.
pub fn persist_checkpoint(
    store: &mut impl CheckpointStore,
    mut checkpoint: DurableCheckpoint,
) -> Result<()> {
    validate_checkpoint(&checkpoint)?;
    if checkpoint.authentication_tag.is_empty() {
        checkpoint.authentication_tag = authenticated_tag(&checkpoint);
    }
    store.save(checkpoint)
}

/// Recovers only when the authenticated stored checkpoint exactly binds the current
/// governing snapshots and has a non-expired lease. It never re-resolves artifacts.
///
/// # Errors
///
/// Returns a fail-closed error for missing, unauthenticated, mismatched, or stale state.
pub fn recover_checkpoint(
    store: &impl CheckpointStore,
    execution_id: &str,
    expected: &GoverningSnapshots,
    now: u64,
) -> Result<DurableCheckpoint> {
    let checkpoint = store.load(execution_id)?.ok_or_else(|| {
        error(
            DurableOrchestrationErrorCode::InvalidCheckpoint,
            "checkpoint is absent",
        )
    })?;
    validate_checkpoint(&checkpoint)?;
    if !store.verify(&checkpoint) {
        return Err(error(
            DurableOrchestrationErrorCode::AuthenticationFailed,
            "checkpoint authentication failed",
        ));
    }
    if &checkpoint.snapshots != expected {
        return Err(error(
            DurableOrchestrationErrorCode::SnapshotMismatch,
            "checkpoint governing snapshots differ",
        ));
    }
    if checkpoint.lease.expires_at <= now {
        return Err(error(
            DurableOrchestrationErrorCode::StaleLease,
            "checkpoint lease has expired",
        ));
    }
    Ok(checkpoint)
}

/// Transfers execution ownership only to a strictly newer fencing token.
///
/// # Errors
///
/// Returns `StaleLease` unless ownership advances both token and expiry.
pub fn acquire_lease(
    checkpoint: &mut DurableCheckpoint,
    owner_id: &str,
    fencing_token: u64,
    expires_at: u64,
) -> Result<()> {
    if owner_id.trim().is_empty()
        || fencing_token <= checkpoint.lease.fencing_token
        || expires_at <= checkpoint.lease.expires_at
    {
        return Err(error(
            DurableOrchestrationErrorCode::StaleLease,
            "lease transfer requires a newer fencing token and expiry",
        ));
    }
    checkpoint.lease = ExecutionLease {
        owner_id: owner_id.to_string(),
        fencing_token,
        expires_at,
    };
    checkpoint.authentication_tag.clear();
    Ok(())
}

/// Enforces declared retryability, attempt and budget bounds, and a stable idempotency key.
///
/// # Errors
///
/// Returns an error when retryability, idempotency, attempts, or budget are exhausted.
pub fn authorize_retry(policy: &RetryPolicy, attempts_used: u32, budget_used: u64) -> Result<u64> {
    if !policy.retryable || policy.idempotency_key.trim().is_empty() {
        return Err(error(
            DurableOrchestrationErrorCode::RetryNotDeclared,
            "retryability and idempotency key must be declared",
        ));
    }
    if attempts_used >= policy.max_attempts || budget_used >= policy.budget_units {
        return Err(error(
            DurableOrchestrationErrorCode::RetryBudgetExhausted,
            "retry attempt or budget limit reached",
        ));
    }
    Ok(policy.backoff_units)
}

/// Records a bounded wake decision. Cancellation wins over wake; expired waits do not wake.
///
/// # Errors
///
/// Returns a cancellation, expiry, or ownership error when the wake is not valid.
pub fn wake_wait(wait: &DurableWait, wake_owner: &str, cancelled: bool, now: u64) -> Result<()> {
    if cancelled {
        return Err(error(
            DurableOrchestrationErrorCode::Cancelled,
            "wait was cancelled before wake",
        ));
    }
    if now > wait.deadline {
        return Err(error(
            DurableOrchestrationErrorCode::WaitExpired,
            "wait deadline elapsed before wake",
        ));
    }
    if wake_owner != wait.owner_id {
        return Err(error(
            DurableOrchestrationErrorCode::StaleLease,
            "wake owner does not own the wait",
        ));
    }
    Ok(())
}

/// Returns compensation in reverse completed order after authorization binding checks.
///
/// # Errors
///
/// Returns `CompensationUnauthorized` when a step is not bound to the authorization snapshot.
pub fn authorized_compensation(
    steps: &[CompensationStep],
    authorization_digest: &str,
) -> Result<Vec<CompensationStep>> {
    if steps.iter().any(|step| {
        step.authorization_digest != authorization_digest || step.capability_id.trim().is_empty()
    }) {
        return Err(error(
            DurableOrchestrationErrorCode::CompensationUnauthorized,
            "compensation is not authorized by the execution snapshot",
        ));
    }
    let mut ordered = steps.to_vec();
    ordered.sort_by_key(|step| std::cmp::Reverse(step.order));
    Ok(ordered)
}

fn validate_checkpoint(checkpoint: &DurableCheckpoint) -> Result<()> {
    if checkpoint.governing_spec != GOVERNING_SPEC
        || checkpoint.execution_id.trim().is_empty()
        || !checkpoint.snapshots.is_complete()
        || checkpoint.lease.owner_id.trim().is_empty()
        || checkpoint.lease.fencing_token == 0
    {
        return Err(error(
            DurableOrchestrationErrorCode::InvalidCheckpoint,
            "checkpoint is incomplete or uses an unsupported governing spec",
        ));
    }
    Ok(())
}

fn authenticated_tag(checkpoint: &DurableCheckpoint) -> String {
    // A deterministic test tag, deliberately scoped to the in-memory adapter.
    // Production adapters verify host-held MACs/signatures via `CheckpointStore::verify`.
    format!(
        "{}:{}:{}",
        checkpoint.execution_id,
        checkpoint.lease.fencing_token,
        checkpoint.snapshots.proposal_digest
    )
}
fn error(code: DurableOrchestrationErrorCode, message: &str) -> DurableOrchestrationError {
    DurableOrchestrationError {
        code,
        message: message.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn checkpoint() -> DurableCheckpoint {
        DurableCheckpoint {
            governing_spec: GOVERNING_SPEC.to_string(),
            execution_id: "exec-1".to_string(),
            snapshots: GoverningSnapshots {
                proposal_digest: "proposal".to_string(),
                manifest_digest: "manifest".to_string(),
                registry_digest: "registry".to_string(),
                policy_digest: "policy".to_string(),
                authorization_digest: "auth".to_string(),
            },
            lease: ExecutionLease {
                owner_id: "worker-a".to_string(),
                fencing_token: 1,
                expires_at: 20,
            },
            completed_node_ids: vec!["charge".to_string()],
            wait: None,
            retry: None,
            compensation: vec![],
            authentication_tag: String::new(),
        }
    }
    #[test]
    fn recovery_fails_closed_for_tamper_snapshot_and_stale_lease() {
        let mut store = MemoryCheckpointStore::default();
        let c = checkpoint();
        assert!(persist_checkpoint(&mut store, c.clone()).is_ok());
        assert!(matches!(
            recover_checkpoint(&store, "exec-1", &c.snapshots, 10),
            Ok(DurableCheckpoint { execution_id, .. }) if execution_id == "exec-1"
        ));
        let mut wrong = c.snapshots.clone();
        wrong.policy_digest = "other".to_string();
        assert!(matches!(
            recover_checkpoint(&store, "exec-1", &wrong, 10),
            Err(DurableOrchestrationError {
                code: DurableOrchestrationErrorCode::SnapshotMismatch,
                ..
            })
        ));
        assert!(matches!(
            recover_checkpoint(&store, "exec-1", &c.snapshots, 20),
            Err(DurableOrchestrationError {
                code: DurableOrchestrationErrorCode::StaleLease,
                ..
            })
        ));
    }
    #[test]
    fn retries_waits_and_compensation_are_bounded_and_declared() {
        let policy = RetryPolicy {
            retryable: true,
            max_attempts: 2,
            backoff_units: 3,
            budget_units: 5,
            idempotency_key: "key".to_string(),
        };
        assert_eq!(authorize_retry(&policy, 0, 0), Ok(3));
        assert!(matches!(
            authorize_retry(&policy, 2, 0),
            Err(DurableOrchestrationError {
                code: DurableOrchestrationErrorCode::RetryBudgetExhausted,
                ..
            })
        ));
        let wait = DurableWait {
            kind: DurableWaitKind::Event,
            owner_id: "worker".to_string(),
            deadline: 10,
            cancellation_id: "cancel".to_string(),
        };
        assert!(wake_wait(&wait, "worker", false, 10).is_ok());
        assert!(matches!(
            wake_wait(&wait, "worker", true, 1),
            Err(DurableOrchestrationError {
                code: DurableOrchestrationErrorCode::Cancelled,
                ..
            })
        ));
        let steps = vec![
            CompensationStep {
                capability_id: "undo-second".to_string(),
                authorization_digest: "auth".to_string(),
                order: 2,
            },
            CompensationStep {
                capability_id: "undo-first".to_string(),
                authorization_digest: "auth".to_string(),
                order: 1,
            },
        ];
        assert!(matches!(
            authorized_compensation(&steps, "auth"),
            Ok(ordered) if ordered[0].capability_id == "undo-second"
        ));
    }

    #[test]
    fn checkpoint_and_recovery_guards_cover_all_fail_closed_paths() {
        let mut store = MemoryCheckpointStore::default();
        let mut incomplete = checkpoint();
        incomplete.snapshots.policy_digest.clear();
        assert!(!incomplete.snapshots.is_complete());
        assert!(matches!(
            persist_checkpoint(&mut store, incomplete),
            Err(DurableOrchestrationError {
                code: DurableOrchestrationErrorCode::InvalidCheckpoint,
                ..
            })
        ));
        let checkpoint = checkpoint();
        assert!(matches!(
            recover_checkpoint(&store, "missing", &checkpoint.snapshots, 0),
            Err(DurableOrchestrationError {
                code: DurableOrchestrationErrorCode::InvalidCheckpoint,
                ..
            })
        ));
        assert!(persist_checkpoint(&mut store, checkpoint.clone()).is_ok());
        let mut tampered = store
            .load("exec-1")
            .expect("memory load should succeed")
            .expect("saved checkpoint should exist");
        tampered.authentication_tag = "invalid".to_string();
        store.checkpoints.insert("exec-1".to_string(), tampered);
        assert!(matches!(
            recover_checkpoint(&store, "exec-1", &checkpoint.snapshots, 0),
            Err(DurableOrchestrationError {
                code: DurableOrchestrationErrorCode::AuthenticationFailed,
                ..
            })
        ));
    }

    #[test]
    fn lease_retry_wait_and_compensation_error_paths_are_stable() {
        let mut checkpoint = checkpoint();
        assert!(matches!(
            acquire_lease(&mut checkpoint, "", 2, 21),
            Err(DurableOrchestrationError {
                code: DurableOrchestrationErrorCode::StaleLease,
                ..
            })
        ));
        assert!(acquire_lease(&mut checkpoint, "worker-b", 2, 21).is_ok());
        assert_eq!(checkpoint.lease.owner_id, "worker-b");
        assert!(checkpoint.authentication_tag.is_empty());

        let policy = RetryPolicy {
            retryable: false,
            max_attempts: 1,
            backoff_units: 1,
            budget_units: 1,
            idempotency_key: String::new(),
        };
        assert!(matches!(
            authorize_retry(&policy, 0, 0),
            Err(DurableOrchestrationError {
                code: DurableOrchestrationErrorCode::RetryNotDeclared,
                ..
            })
        ));
        let bounded_policy = RetryPolicy {
            retryable: true,
            idempotency_key: "key".to_string(),
            ..policy
        };
        assert!(matches!(
            authorize_retry(&bounded_policy, 0, 1),
            Err(DurableOrchestrationError {
                code: DurableOrchestrationErrorCode::RetryBudgetExhausted,
                ..
            })
        ));

        let wait = DurableWait {
            kind: DurableWaitKind::Schedule,
            owner_id: "worker-b".to_string(),
            deadline: 10,
            cancellation_id: "cancel".to_string(),
        };
        assert!(matches!(
            wake_wait(&wait, "worker-b", false, 11),
            Err(DurableOrchestrationError {
                code: DurableOrchestrationErrorCode::WaitExpired,
                ..
            })
        ));
        assert!(matches!(
            wake_wait(&wait, "worker-c", false, 10),
            Err(DurableOrchestrationError {
                code: DurableOrchestrationErrorCode::StaleLease,
                ..
            })
        ));
        let unauthorized = [CompensationStep {
            capability_id: String::new(),
            authorization_digest: "wrong".to_string(),
            order: 1,
        }];
        assert!(matches!(
            authorized_compensation(&unauthorized, "auth"),
            Err(DurableOrchestrationError {
                code: DurableOrchestrationErrorCode::CompensationUnauthorized,
                ..
            })
        ));
    }
}

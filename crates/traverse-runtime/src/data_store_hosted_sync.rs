//! Provider-neutral hosted `DataStore` synchronization transport (spec `087`).
//!
//! Portable code depends only on [`HostedSyncTransport`]. The deterministic
//! [`InMemoryHostedSyncTransport`] and optional [`AblyHostedSyncTransport`]
//! share one conformance suite (QG-001). Provider SDKs and native channel
//! names stay inside the Ably adapter edge and never appear on the port.

use crate::events::{LifecycleStatus, TraverseEvent};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const HOSTED_TRANSPORT_SPEC: &str = "087-hosted-datastore-transport";
const SYNC_PROTOCOL_SPEC: &str = "089-datastore-synchronization";
const MIN_REPLAY_WINDOW_MS: u64 = 120_000;
const HEXADECIMAL_DIGITS: &[u8; 16] = b"0123456789abcdef";

/// Opaque, backend-derived synchronization scope (FR-004). Not a capability ID.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SyncScopeId(String);

impl SyncScopeId {
    /// Creates a scope identifier from an opaque backend string.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the opaque scope string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns a non-reversible hash suitable for redacted observability (FR-011).
    #[must_use]
    pub fn hashed(&self) -> String {
        hex_digest(self.0.as_bytes())
    }
}

/// Short-lived, least-privilege credential issued by the application backend (FR-003).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostedSyncCredential {
    pub token: String,
    pub scope: SyncScopeId,
    /// Logical expiry instant supplied by the host (milliseconds).
    pub expires_at_ms: u64,
}

/// Encrypted synchronization operation. The adapter never interprets plaintext (FR-006).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedSyncOperation {
    pub operation_id: String,
    pub synchronization_set_id: String,
    pub writer_id: String,
    pub lamport_clock: u64,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
    pub key_version_id: String,
    /// Application-layer ciphertext; never logged by the adapter.
    pub ciphertext: Vec<u8>,
}

/// Typed connection / synchronization state (FR-005, FR-010).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostedSyncConnectionState {
    Disconnected,
    Connected,
    Degraded { reason: HostedSyncDegradedReason },
    Recovering,
}

/// Why synchronization entered a degraded state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostedSyncDegradedReason {
    RelayUnavailable,
    CredentialExpired,
    CredentialRefreshFailed,
}

/// Stable machine-readable failure codes (FR-005, FR-012).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostedSyncErrorCode {
    UnauthorizedScope,
    CredentialExpired,
    CredentialMismatch,
    InvalidEnvelope,
    KeyMismatch,
    ProviderUnavailable,
    NotConnected,
    ResyncRequired,
}

/// Sanitized transport error with secret-free details.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostedSyncError {
    pub code: HostedSyncErrorCode,
    pub message: String,
    pub details: Value,
}

/// Outcome of a cursor-based replay request (FR-008).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum HostedSyncReplayResult {
    Delivered {
        operations: Vec<EncryptedSyncOperation>,
        cursor: String,
    },
    ResyncRequired {
        oldest_available_cursor: Option<String>,
    },
}

/// Publish acknowledgement with opaque cursor advancement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostedSyncPublishReceipt {
    pub operation_id: String,
    pub cursor: String,
}

/// Redacted observability record (FR-011).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostedSyncObservation {
    pub governing_spec: String,
    pub kind: String,
    pub operation_id: Option<String>,
    pub key_version_id: Option<String>,
    pub hashed_scope: Option<String>,
    pub connection_state: Option<HostedSyncConnectionState>,
    pub outcome: Option<String>,
    pub latency_ms: Option<u64>,
}

/// Declared/observed lineage evidence for one relayed operation (FR-012).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostedSyncLineageEvidence {
    pub governing_spec: String,
    pub protocol_spec: String,
    pub operation_id: String,
    pub synchronization_set_id: String,
    pub writer_id: String,
    pub lamport_clock: u64,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
    pub key_version_id: String,
    pub event_id: String,
    pub observed: bool,
}

/// Provider-neutral hosted synchronization transport port (FR-001).
pub trait HostedSyncTransport {
    /// Connects with an application-issued scoped credential.
    ///
    /// # Errors
    ///
    /// Returns a typed [`HostedSyncError`] when the credential is rejected or
    /// the provider is unavailable.
    fn connect(&mut self, credential: HostedSyncCredential) -> Result<(), HostedSyncError>;

    /// Replaces the active credential before expiry (FR-005).
    ///
    /// # Errors
    ///
    /// Returns a typed [`HostedSyncError`] on mismatch, expiry, or outage.
    fn refresh_credential(
        &mut self,
        credential: HostedSyncCredential,
    ) -> Result<(), HostedSyncError>;

    /// Publishes one encrypted operation without interpreting plaintext (FR-002).
    ///
    /// # Errors
    ///
    /// Returns a typed [`HostedSyncError`] for envelope, auth, key, or outage failures.
    fn publish(
        &mut self,
        operation: EncryptedSyncOperation,
    ) -> Result<HostedSyncPublishReceipt, HostedSyncError>;

    /// Replays retained operations from `cursor`, or from the channel head when `None`.
    ///
    /// # Errors
    ///
    /// Returns a typed [`HostedSyncError`] when disconnected or the provider fails.
    /// Expired cursors return [`HostedSyncReplayResult::ResyncRequired`], not an error.
    fn replay_from(
        &mut self,
        cursor: Option<&str>,
    ) -> Result<HostedSyncReplayResult, HostedSyncError>;

    /// Returns the current connection / degraded / recovering state.
    fn connection_state(&self) -> HostedSyncConnectionState;

    /// Advances the host-supplied logical clock used for expiry and replay windows.
    fn advance_clock(&mut self, now_ms: u64);

    /// Marks the relay available or unavailable (test and host control plane).
    fn set_relay_available(&mut self, available: bool);

    /// Returns redacted observations accumulated since connect (FR-011).
    fn observations(&self) -> &[HostedSyncObservation];

    /// Returns lineage evidence for successfully relayed operations (FR-012).
    fn lineage(&self) -> &[HostedSyncLineageEvidence];

    /// Adapter identity for conformance evidence (`in_memory` or `ably`).
    fn adapter_kind(&self) -> &'static str;
}

/// Deterministic in-memory hosted transport used as the replacement-boundary fixture.
#[derive(Debug)]
pub struct InMemoryHostedSyncTransport {
    inner: SharedRelay,
}

impl InMemoryHostedSyncTransport {
    /// Creates an empty in-memory relay with the minimum two-minute replay window.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: SharedRelay::new(MIN_REPLAY_WINDOW_MS),
        }
    }
}

impl Default for InMemoryHostedSyncTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl HostedSyncTransport for InMemoryHostedSyncTransport {
    fn connect(&mut self, credential: HostedSyncCredential) -> Result<(), HostedSyncError> {
        self.inner.connect(credential)
    }

    fn refresh_credential(
        &mut self,
        credential: HostedSyncCredential,
    ) -> Result<(), HostedSyncError> {
        self.inner.refresh_credential(credential)
    }

    fn publish(
        &mut self,
        operation: EncryptedSyncOperation,
    ) -> Result<HostedSyncPublishReceipt, HostedSyncError> {
        self.inner.publish(operation)
    }

    fn replay_from(
        &mut self,
        cursor: Option<&str>,
    ) -> Result<HostedSyncReplayResult, HostedSyncError> {
        self.inner.replay_from(cursor)
    }

    fn connection_state(&self) -> HostedSyncConnectionState {
        self.inner.connection_state()
    }

    fn advance_clock(&mut self, now_ms: u64) {
        self.inner.advance_clock(now_ms);
    }

    fn set_relay_available(&mut self, available: bool) {
        self.inner.set_relay_available(available);
    }

    fn observations(&self) -> &[HostedSyncObservation] {
        self.inner.observations()
    }

    fn lineage(&self) -> &[HostedSyncLineageEvidence] {
        self.inner.lineage()
    }

    fn adapter_kind(&self) -> &'static str {
        "in_memory"
    }
}

/// Optional Ably-shaped hosted adapter. Core callers depend only on
/// [`HostedSyncTransport`]; Ably channel names stay inside [`AblyRealtimeEdge`].
#[derive(Debug)]
pub struct AblyHostedSyncTransport<E: AblyRealtimeEdge> {
    edge: E,
    session: Option<AblySession>,
    now_ms: u64,
    relay_available: bool,
    accepted_key_versions: BTreeSet<String>,
    seen_operation_ids: BTreeSet<String>,
    observations: Vec<HostedSyncObservation>,
    lineage: Vec<HostedSyncLineageEvidence>,
    replay_window_ms: u64,
}

#[derive(Debug, Clone)]
struct AblySession {
    credential: HostedSyncCredential,
    /// Provider-native channel name; never emitted in observations (FR-011).
    provider_channel: String,
}

/// Injectable Ably edge so the optional adapter stays replaceable and testable.
pub trait AblyRealtimeEdge {
    /// Publishes ciphertext to a provider-native channel using a scoped token.
    ///
    /// # Errors
    ///
    /// Returns [`AblyEdgeError`] when auth fails or the edge is unavailable.
    fn publish(
        &mut self,
        channel: &str,
        token: &str,
        retained_at_ms: u64,
        payload: &[u8],
    ) -> Result<String, AblyEdgeError>;

    /// Returns ordered history after `cursor` for the provider channel.
    ///
    /// # Errors
    ///
    /// Returns [`AblyEdgeError`] on auth/outage failures. Expired cursors use
    /// [`AblyEdgeError::CursorExpired`].
    fn history_from(
        &mut self,
        channel: &str,
        token: &str,
        cursor: Option<&str>,
        now_ms: u64,
        replay_window_ms: u64,
    ) -> Result<AblyHistoryBatch, AblyEdgeError>;

    /// Marks the edge available or unavailable.
    fn set_available(&mut self, available: bool);
}

/// Errors from the Ably edge translation layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AblyEdgeError {
    Unauthorized,
    Unavailable,
    CursorExpired {
        oldest_available_cursor: Option<String>,
    },
}

/// Ordered history batch returned by an Ably edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AblyHistoryBatch {
    pub payloads: Vec<Vec<u8>>,
    pub cursor: String,
}

impl<E: AblyRealtimeEdge> AblyHostedSyncTransport<E> {
    /// Creates an Ably adapter over an application-selected edge.
    #[must_use]
    pub fn new(edge: E) -> Self {
        Self {
            edge,
            session: None,
            now_ms: 0,
            relay_available: true,
            accepted_key_versions: BTreeSet::from([
                "key-active".to_string(),
                "key-previous".to_string(),
            ]),
            seen_operation_ids: BTreeSet::new(),
            observations: Vec::new(),
            lineage: Vec::new(),
            replay_window_ms: MIN_REPLAY_WINDOW_MS,
        }
    }

    /// Restricts accepted key-version identifiers (FR-006).
    pub fn set_accepted_key_versions<I, S>(&mut self, versions: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.accepted_key_versions = versions.into_iter().map(Into::into).collect();
    }

    fn require_session(&self) -> Result<&AblySession, HostedSyncError> {
        self.session.as_ref().ok_or_else(|| {
            hosted_error(
                HostedSyncErrorCode::NotConnected,
                "hosted sync transport is not connected",
                json!({}),
            )
        })
    }

    fn ensure_live_credential(&mut self) -> Result<(), HostedSyncError> {
        let Some(session) = &self.session else {
            return Err(hosted_error(
                HostedSyncErrorCode::NotConnected,
                "hosted sync transport is not connected",
                json!({}),
            ));
        };
        if session.credential.expires_at_ms <= self.now_ms {
            self.session = None;
            let state = HostedSyncConnectionState::Degraded {
                reason: HostedSyncDegradedReason::CredentialExpired,
            };
            self.observe_state("credential_expired", &state);
            return Err(hosted_error(
                HostedSyncErrorCode::CredentialExpired,
                "scoped hosted-relay credential expired",
                json!({}),
            ));
        }
        if !self.relay_available {
            let state = HostedSyncConnectionState::Degraded {
                reason: HostedSyncDegradedReason::RelayUnavailable,
            };
            self.observe_state("relay_unavailable", &state);
            return Err(hosted_error(
                HostedSyncErrorCode::ProviderUnavailable,
                "hosted relay unavailable",
                json!({}),
            ));
        }
        Ok(())
    }

    fn observe_state(&mut self, kind: &str, state: &HostedSyncConnectionState) {
        let hashed_scope = self
            .session
            .as_ref()
            .map(|session| session.credential.scope.hashed());
        self.observations.push(HostedSyncObservation {
            governing_spec: HOSTED_TRANSPORT_SPEC.to_string(),
            kind: kind.to_string(),
            operation_id: None,
            key_version_id: None,
            hashed_scope,
            connection_state: Some(state.clone()),
            outcome: Some(kind.to_string()),
            latency_ms: Some(0),
        });
    }

    fn validate_operation(
        &self,
        operation: &EncryptedSyncOperation,
    ) -> Result<(), HostedSyncError> {
        if operation.operation_id.is_empty()
            || operation.synchronization_set_id.is_empty()
            || operation.writer_id.is_empty()
            || operation.key_version_id.is_empty()
            || operation.ciphertext.is_empty()
        {
            return Err(hosted_error(
                HostedSyncErrorCode::InvalidEnvelope,
                "encrypted sync operation envelope is incomplete",
                json!({}),
            ));
        }
        if !self
            .accepted_key_versions
            .contains(&operation.key_version_id)
        {
            return Err(hosted_error(
                HostedSyncErrorCode::KeyMismatch,
                "key-version is outside the accepted active/previous window",
                json!({ "key_version_id": operation.key_version_id }),
            ));
        }
        Ok(())
    }
}

impl<E: AblyRealtimeEdge> HostedSyncTransport for AblyHostedSyncTransport<E> {
    fn connect(&mut self, credential: HostedSyncCredential) -> Result<(), HostedSyncError> {
        if credential.token.is_empty() || credential.scope.as_str().is_empty() {
            return Err(hosted_error(
                HostedSyncErrorCode::UnauthorizedScope,
                "scoped credential is missing token or scope",
                json!({}),
            ));
        }
        if credential.expires_at_ms <= self.now_ms {
            let state = HostedSyncConnectionState::Degraded {
                reason: HostedSyncDegradedReason::CredentialExpired,
            };
            self.observe_state("connect_rejected_expired", &state);
            return Err(hosted_error(
                HostedSyncErrorCode::CredentialExpired,
                "scoped hosted-relay credential already expired",
                json!({}),
            ));
        }
        if !self.relay_available {
            let state = HostedSyncConnectionState::Degraded {
                reason: HostedSyncDegradedReason::RelayUnavailable,
            };
            self.observe_state("connect_rejected_unavailable", &state);
            return Err(hosted_error(
                HostedSyncErrorCode::ProviderUnavailable,
                "hosted relay unavailable",
                json!({}),
            ));
        }
        let provider_channel = ably_channel_for_scope(&credential.scope);
        let hashed_scope = credential.scope.hashed();
        self.session = Some(AblySession {
            credential,
            provider_channel,
        });
        let state = HostedSyncConnectionState::Connected;
        self.observations.push(HostedSyncObservation {
            governing_spec: HOSTED_TRANSPORT_SPEC.to_string(),
            kind: "connected".to_string(),
            operation_id: None,
            key_version_id: None,
            hashed_scope: Some(hashed_scope),
            connection_state: Some(state),
            outcome: Some("connected".to_string()),
            latency_ms: Some(0),
        });
        Ok(())
    }

    fn refresh_credential(
        &mut self,
        credential: HostedSyncCredential,
    ) -> Result<(), HostedSyncError> {
        let Some(session) = &self.session else {
            return Err(hosted_error(
                HostedSyncErrorCode::NotConnected,
                "hosted sync transport is not connected",
                json!({}),
            ));
        };
        if session.credential.scope != credential.scope {
            return Err(hosted_error(
                HostedSyncErrorCode::CredentialMismatch,
                "credential scope does not match the connected synchronization scope",
                json!({ "hashed_scope": credential.scope.hashed() }),
            ));
        }
        if credential.expires_at_ms <= self.now_ms || credential.token.is_empty() {
            self.session = None;
            let state = HostedSyncConnectionState::Degraded {
                reason: HostedSyncDegradedReason::CredentialRefreshFailed,
            };
            self.observe_state("credential_refresh_failed", &state);
            return Err(hosted_error(
                HostedSyncErrorCode::CredentialExpired,
                "credential refresh failed",
                json!({}),
            ));
        }
        if !self.relay_available {
            self.session = None;
            let state = HostedSyncConnectionState::Degraded {
                reason: HostedSyncDegradedReason::RelayUnavailable,
            };
            self.observe_state("credential_refresh_unavailable", &state);
            return Err(hosted_error(
                HostedSyncErrorCode::ProviderUnavailable,
                "hosted relay unavailable during credential refresh",
                json!({}),
            ));
        }
        let provider_channel = ably_channel_for_scope(&credential.scope);
        self.session = Some(AblySession {
            credential,
            provider_channel,
        });
        let state = HostedSyncConnectionState::Connected;
        self.observe_state("credential_refreshed", &state);
        Ok(())
    }

    fn publish(
        &mut self,
        operation: EncryptedSyncOperation,
    ) -> Result<HostedSyncPublishReceipt, HostedSyncError> {
        self.ensure_live_credential()?;
        self.validate_operation(&operation)?;
        if self.seen_operation_ids.contains(&operation.operation_id) {
            let cursor = format!("cursor:{}", operation.operation_id);
            self.observations.push(HostedSyncObservation {
                governing_spec: HOSTED_TRANSPORT_SPEC.to_string(),
                kind: "publish_deduplicated".to_string(),
                operation_id: Some(operation.operation_id.clone()),
                key_version_id: Some(operation.key_version_id.clone()),
                hashed_scope: self.session.as_ref().map(|s| s.credential.scope.hashed()),
                connection_state: Some(HostedSyncConnectionState::Connected),
                outcome: Some("deduplicated".to_string()),
                latency_ms: Some(0),
            });
            return Ok(HostedSyncPublishReceipt {
                operation_id: operation.operation_id,
                cursor,
            });
        }
        let event = wrap_ecca_event(&operation);
        let payload = serde_json::to_vec(&event).map_err(|error| {
            hosted_error(
                HostedSyncErrorCode::InvalidEnvelope,
                "failed to serialize ECCA sync envelope",
                json!({ "cause": error.to_string() }),
            )
        })?;
        let session = self.require_session()?.clone();
        let cursor = self
            .edge
            .publish(
                &session.provider_channel,
                &session.credential.token,
                self.now_ms,
                &payload,
            )
            .map_err(map_ably_error)?;
        self.seen_operation_ids
            .insert(operation.operation_id.clone());
        self.lineage.push(HostedSyncLineageEvidence {
            governing_spec: HOSTED_TRANSPORT_SPEC.to_string(),
            protocol_spec: SYNC_PROTOCOL_SPEC.to_string(),
            operation_id: operation.operation_id.clone(),
            synchronization_set_id: operation.synchronization_set_id.clone(),
            writer_id: operation.writer_id.clone(),
            lamport_clock: operation.lamport_clock,
            correlation_id: operation.correlation_id.clone(),
            causation_id: operation.causation_id.clone(),
            key_version_id: operation.key_version_id.clone(),
            event_id: event.id,
            observed: true,
        });
        self.observations.push(HostedSyncObservation {
            governing_spec: HOSTED_TRANSPORT_SPEC.to_string(),
            kind: "publish".to_string(),
            operation_id: Some(operation.operation_id.clone()),
            key_version_id: Some(operation.key_version_id.clone()),
            hashed_scope: Some(session.credential.scope.hashed()),
            connection_state: Some(HostedSyncConnectionState::Connected),
            outcome: Some("delivered".to_string()),
            latency_ms: Some(0),
        });
        Ok(HostedSyncPublishReceipt {
            operation_id: operation.operation_id,
            cursor,
        })
    }

    fn replay_from(
        &mut self,
        cursor: Option<&str>,
    ) -> Result<HostedSyncReplayResult, HostedSyncError> {
        self.ensure_live_credential()?;
        let session = self.require_session()?.clone();
        match self.edge.history_from(
            &session.provider_channel,
            &session.credential.token,
            cursor,
            self.now_ms,
            self.replay_window_ms,
        ) {
            Ok(batch) => {
                let mut operations = Vec::new();
                for payload in batch.payloads {
                    let event: TraverseEvent = serde_json::from_slice(&payload).map_err(|_| {
                        hosted_error(
                            HostedSyncErrorCode::InvalidEnvelope,
                            "replay payload is not a valid ECCA envelope",
                            json!({}),
                        )
                    })?;
                    operations.push(operation_from_ecca_event(&event)?);
                }
                self.observations.push(HostedSyncObservation {
                    governing_spec: HOSTED_TRANSPORT_SPEC.to_string(),
                    kind: "replay".to_string(),
                    operation_id: None,
                    key_version_id: None,
                    hashed_scope: Some(session.credential.scope.hashed()),
                    connection_state: Some(HostedSyncConnectionState::Connected),
                    outcome: Some("delivered".to_string()),
                    latency_ms: Some(0),
                });
                Ok(HostedSyncReplayResult::Delivered {
                    operations,
                    cursor: batch.cursor,
                })
            }
            Err(AblyEdgeError::CursorExpired {
                oldest_available_cursor,
            }) => {
                self.observations.push(HostedSyncObservation {
                    governing_spec: HOSTED_TRANSPORT_SPEC.to_string(),
                    kind: "replay".to_string(),
                    operation_id: None,
                    key_version_id: None,
                    hashed_scope: Some(session.credential.scope.hashed()),
                    connection_state: Some(HostedSyncConnectionState::Connected),
                    outcome: Some("resync_required".to_string()),
                    latency_ms: Some(0),
                });
                Ok(HostedSyncReplayResult::ResyncRequired {
                    oldest_available_cursor,
                })
            }
            Err(error) => Err(map_ably_error(error)),
        }
    }

    fn connection_state(&self) -> HostedSyncConnectionState {
        if self.session.is_none() {
            return HostedSyncConnectionState::Disconnected;
        }
        if !self.relay_available {
            return HostedSyncConnectionState::Degraded {
                reason: HostedSyncDegradedReason::RelayUnavailable,
            };
        }
        if self
            .session
            .as_ref()
            .is_some_and(|session| session.credential.expires_at_ms <= self.now_ms)
        {
            return HostedSyncConnectionState::Degraded {
                reason: HostedSyncDegradedReason::CredentialExpired,
            };
        }
        HostedSyncConnectionState::Connected
    }

    fn advance_clock(&mut self, now_ms: u64) {
        if now_ms > self.now_ms {
            self.now_ms = now_ms;
        }
        if self
            .session
            .as_ref()
            .is_some_and(|session| session.credential.expires_at_ms <= self.now_ms)
        {
            let state = HostedSyncConnectionState::Degraded {
                reason: HostedSyncDegradedReason::CredentialExpired,
            };
            self.observe_state("credential_expired", &state);
            self.session = None;
        }
    }

    fn set_relay_available(&mut self, available: bool) {
        self.relay_available = available;
        self.edge.set_available(available);
        if available {
            if self.session.is_some() {
                let state = HostedSyncConnectionState::Recovering;
                self.observe_state("recovering", &state);
                let state = HostedSyncConnectionState::Connected;
                self.observe_state("reconnected", &state);
            }
        } else if self.session.is_some() {
            let state = HostedSyncConnectionState::Degraded {
                reason: HostedSyncDegradedReason::RelayUnavailable,
            };
            self.observe_state("relay_unavailable", &state);
        }
    }

    fn observations(&self) -> &[HostedSyncObservation] {
        &self.observations
    }

    fn lineage(&self) -> &[HostedSyncLineageEvidence] {
        &self.lineage
    }

    fn adapter_kind(&self) -> &'static str {
        "ably"
    }
}

/// Deterministic Ably edge double used by the shared conformance suite.
#[derive(Debug, Default)]
pub struct InMemoryAblyEdge {
    available: bool,
    /// channel -> tokens authorized for that channel
    tokens: BTreeMap<String, BTreeSet<String>>,
    channels: BTreeMap<String, VecDeque<RetainedMessage>>,
}

impl InMemoryAblyEdge {
    /// Creates an available edge with no retained history.
    #[must_use]
    pub fn new() -> Self {
        Self {
            available: true,
            tokens: BTreeMap::new(),
            channels: BTreeMap::new(),
        }
    }

    /// Authorizes `token` for `channel` (application-backend stand-in).
    pub fn authorize(&mut self, channel: &str, token: &str) {
        self.tokens
            .entry(channel.to_string())
            .or_default()
            .insert(token.to_string());
    }

    fn token_allowed(&self, channel: &str, token: &str) -> bool {
        self.tokens
            .get(channel)
            .is_some_and(|tokens| tokens.contains(token))
    }
}

impl AblyRealtimeEdge for InMemoryAblyEdge {
    fn publish(
        &mut self,
        channel: &str,
        token: &str,
        retained_at_ms: u64,
        payload: &[u8],
    ) -> Result<String, AblyEdgeError> {
        if !self.available {
            return Err(AblyEdgeError::Unavailable);
        }
        if !self.token_allowed(channel, token) {
            return Err(AblyEdgeError::Unauthorized);
        }
        let queue = self.channels.entry(channel.to_string()).or_default();
        let seq = queue.len().saturating_add(1);
        let cursor = format!("{channel}:{seq}");
        queue.push_back(RetainedMessage {
            cursor: cursor.clone(),
            retained_at_ms,
            payload: payload.to_vec(),
        });
        Ok(cursor)
    }

    fn history_from(
        &mut self,
        channel: &str,
        token: &str,
        cursor: Option<&str>,
        now_ms: u64,
        replay_window_ms: u64,
    ) -> Result<AblyHistoryBatch, AblyEdgeError> {
        if !self.available {
            return Err(AblyEdgeError::Unavailable);
        }
        if !self.token_allowed(channel, token) {
            return Err(AblyEdgeError::Unauthorized);
        }
        let Some(queue) = self.channels.get(channel) else {
            return Ok(AblyHistoryBatch {
                payloads: Vec::new(),
                cursor: cursor.unwrap_or("0").to_string(),
            });
        };
        let oldest_retained_ms = now_ms.saturating_sub(replay_window_ms);
        let retained: Vec<&RetainedMessage> = queue
            .iter()
            .filter(|message| message.retained_at_ms >= oldest_retained_ms)
            .collect();
        let oldest_available_cursor = retained.first().map(|message| message.cursor.clone());
        if let Some(cursor) = cursor {
            let known = queue.iter().any(|message| message.cursor == cursor);
            let still_retained = retained.iter().any(|message| message.cursor == cursor);
            if known && !still_retained {
                return Err(AblyEdgeError::CursorExpired {
                    oldest_available_cursor,
                });
            }
            if !known && !cursor.is_empty() && cursor != "0" {
                return Err(AblyEdgeError::CursorExpired {
                    oldest_available_cursor,
                });
            }
        }
        let start = cursor
            .and_then(|cursor| {
                retained
                    .iter()
                    .position(|message| message.cursor == cursor)
                    .map(|index| index.saturating_add(1))
            })
            .unwrap_or(0);
        let slice = &retained[start.min(retained.len())..];
        let payloads = slice
            .iter()
            .map(|message| message.payload.clone())
            .collect();
        let next_cursor = slice
            .last()
            .map(|message| message.cursor.clone())
            .or_else(|| cursor.map(str::to_string))
            .unwrap_or_else(|| "0".to_string());
        Ok(AblyHistoryBatch {
            payloads,
            cursor: next_cursor,
        })
    }

    fn set_available(&mut self, available: bool) {
        self.available = available;
    }
}

#[derive(Debug, Clone)]
struct RetainedMessage {
    cursor: String,
    retained_at_ms: u64,
    payload: Vec<u8>,
}

#[derive(Debug)]
struct SharedRelay {
    now_ms: u64,
    replay_window_ms: u64,
    relay_available: bool,
    session: Option<HostedSyncCredential>,
    accepted_key_versions: BTreeSet<String>,
    seen_operation_ids: BTreeSet<String>,
    messages: VecDeque<RetainedMessage>,
    operations_by_cursor: BTreeMap<String, EncryptedSyncOperation>,
    observations: Vec<HostedSyncObservation>,
    lineage: Vec<HostedSyncLineageEvidence>,
}

impl SharedRelay {
    fn new(replay_window_ms: u64) -> Self {
        Self {
            now_ms: 0,
            replay_window_ms,
            relay_available: true,
            session: None,
            accepted_key_versions: BTreeSet::from([
                "key-active".to_string(),
                "key-previous".to_string(),
            ]),
            seen_operation_ids: BTreeSet::new(),
            messages: VecDeque::new(),
            operations_by_cursor: BTreeMap::new(),
            observations: Vec::new(),
            lineage: Vec::new(),
        }
    }

    fn connect(&mut self, credential: HostedSyncCredential) -> Result<(), HostedSyncError> {
        if credential.token.is_empty() || credential.scope.as_str().is_empty() {
            return Err(hosted_error(
                HostedSyncErrorCode::UnauthorizedScope,
                "scoped credential is missing token or scope",
                json!({}),
            ));
        }
        if credential.expires_at_ms <= self.now_ms {
            let state = HostedSyncConnectionState::Degraded {
                reason: HostedSyncDegradedReason::CredentialExpired,
            };
            self.observe(
                "connect_rejected_expired",
                None,
                None,
                Some(state),
                "rejected",
            );
            return Err(hosted_error(
                HostedSyncErrorCode::CredentialExpired,
                "scoped hosted-relay credential already expired",
                json!({}),
            ));
        }
        if !self.relay_available {
            let state = HostedSyncConnectionState::Degraded {
                reason: HostedSyncDegradedReason::RelayUnavailable,
            };
            self.observe(
                "connect_rejected_unavailable",
                None,
                None,
                Some(state),
                "rejected",
            );
            return Err(hosted_error(
                HostedSyncErrorCode::ProviderUnavailable,
                "hosted relay unavailable",
                json!({}),
            ));
        }
        let hashed_scope = credential.scope.hashed();
        self.session = Some(credential);
        self.observe(
            "connected",
            None,
            None,
            Some(HostedSyncConnectionState::Connected),
            "connected",
        );
        if let Some(last) = self.observations.last_mut() {
            last.hashed_scope = Some(hashed_scope);
        }
        Ok(())
    }

    fn refresh_credential(
        &mut self,
        credential: HostedSyncCredential,
    ) -> Result<(), HostedSyncError> {
        let Some(session) = &self.session else {
            return Err(hosted_error(
                HostedSyncErrorCode::NotConnected,
                "hosted sync transport is not connected",
                json!({}),
            ));
        };
        if session.scope != credential.scope {
            return Err(hosted_error(
                HostedSyncErrorCode::CredentialMismatch,
                "credential scope does not match the connected synchronization scope",
                json!({ "hashed_scope": credential.scope.hashed() }),
            ));
        }
        if credential.expires_at_ms <= self.now_ms || credential.token.is_empty() {
            self.session = None;
            let state = HostedSyncConnectionState::Degraded {
                reason: HostedSyncDegradedReason::CredentialRefreshFailed,
            };
            self.observe(
                "credential_refresh_failed",
                None,
                None,
                Some(state),
                "degraded",
            );
            return Err(hosted_error(
                HostedSyncErrorCode::CredentialExpired,
                "credential refresh failed",
                json!({}),
            ));
        }
        if !self.relay_available {
            self.session = None;
            let state = HostedSyncConnectionState::Degraded {
                reason: HostedSyncDegradedReason::RelayUnavailable,
            };
            self.observe(
                "credential_refresh_unavailable",
                None,
                None,
                Some(state),
                "degraded",
            );
            return Err(hosted_error(
                HostedSyncErrorCode::ProviderUnavailable,
                "hosted relay unavailable during credential refresh",
                json!({}),
            ));
        }
        self.session = Some(credential);
        self.observe(
            "credential_refreshed",
            None,
            None,
            Some(HostedSyncConnectionState::Connected),
            "refreshed",
        );
        Ok(())
    }

    fn publish(
        &mut self,
        operation: EncryptedSyncOperation,
    ) -> Result<HostedSyncPublishReceipt, HostedSyncError> {
        self.ensure_live_credential()?;
        self.validate_operation(&operation)?;
        if self.seen_operation_ids.contains(&operation.operation_id) {
            let cursor = format!("cursor:{}", operation.operation_id);
            self.observe(
                "publish_deduplicated",
                Some(operation.operation_id.clone()),
                Some(operation.key_version_id.clone()),
                Some(HostedSyncConnectionState::Connected),
                "deduplicated",
            );
            return Ok(HostedSyncPublishReceipt {
                operation_id: operation.operation_id,
                cursor,
            });
        }
        let event = wrap_ecca_event(&operation);
        let seq = self.messages.len().saturating_add(1);
        let cursor = format!("cursor:{seq}");
        self.messages.push_back(RetainedMessage {
            cursor: cursor.clone(),
            retained_at_ms: self.now_ms,
            payload: Vec::new(),
        });
        self.operations_by_cursor
            .insert(cursor.clone(), operation.clone());
        self.seen_operation_ids
            .insert(operation.operation_id.clone());
        self.lineage.push(HostedSyncLineageEvidence {
            governing_spec: HOSTED_TRANSPORT_SPEC.to_string(),
            protocol_spec: SYNC_PROTOCOL_SPEC.to_string(),
            operation_id: operation.operation_id.clone(),
            synchronization_set_id: operation.synchronization_set_id.clone(),
            writer_id: operation.writer_id.clone(),
            lamport_clock: operation.lamport_clock,
            correlation_id: operation.correlation_id.clone(),
            causation_id: operation.causation_id.clone(),
            key_version_id: operation.key_version_id.clone(),
            event_id: event.id,
            observed: true,
        });
        self.observe(
            "publish",
            Some(operation.operation_id.clone()),
            Some(operation.key_version_id.clone()),
            Some(HostedSyncConnectionState::Connected),
            "delivered",
        );
        Ok(HostedSyncPublishReceipt {
            operation_id: operation.operation_id,
            cursor,
        })
    }

    fn replay_from(
        &mut self,
        cursor: Option<&str>,
    ) -> Result<HostedSyncReplayResult, HostedSyncError> {
        self.ensure_live_credential()?;
        self.prune_expired();
        let oldest_available_cursor = self.messages.front().map(|message| message.cursor.clone());
        if let Some(cursor) = cursor {
            let known = self.operations_by_cursor.contains_key(cursor)
                || self.messages.iter().any(|message| message.cursor == cursor);
            let still_retained = self.messages.iter().any(|message| message.cursor == cursor);
            if (known && !still_retained) || (!known && cursor != "0" && !cursor.is_empty()) {
                self.observe(
                    "replay",
                    None,
                    None,
                    Some(HostedSyncConnectionState::Connected),
                    "resync_required",
                );
                return Ok(HostedSyncReplayResult::ResyncRequired {
                    oldest_available_cursor,
                });
            }
        }
        let start = cursor
            .and_then(|cursor| {
                self.messages
                    .iter()
                    .position(|message| message.cursor == cursor)
                    .map(|index| index.saturating_add(1))
            })
            .unwrap_or(0);
        let operations: Vec<EncryptedSyncOperation> = self
            .messages
            .iter()
            .skip(start)
            .filter_map(|message| self.operations_by_cursor.get(&message.cursor).cloned())
            .collect();
        let last_index = start.saturating_add(operations.len().saturating_sub(1));
        let next_cursor = self
            .messages
            .get(last_index)
            .map(|message| message.cursor.clone())
            .or_else(|| cursor.map(str::to_string))
            .unwrap_or_else(|| "0".to_string());
        self.observe(
            "replay",
            None,
            None,
            Some(HostedSyncConnectionState::Connected),
            "delivered",
        );
        Ok(HostedSyncReplayResult::Delivered {
            operations,
            cursor: next_cursor,
        })
    }

    fn connection_state(&self) -> HostedSyncConnectionState {
        if self.session.is_none() {
            return HostedSyncConnectionState::Disconnected;
        }
        if !self.relay_available {
            return HostedSyncConnectionState::Degraded {
                reason: HostedSyncDegradedReason::RelayUnavailable,
            };
        }
        if self
            .session
            .as_ref()
            .is_some_and(|credential| credential.expires_at_ms <= self.now_ms)
        {
            return HostedSyncConnectionState::Degraded {
                reason: HostedSyncDegradedReason::CredentialExpired,
            };
        }
        HostedSyncConnectionState::Connected
    }

    fn advance_clock(&mut self, now_ms: u64) {
        if now_ms > self.now_ms {
            self.now_ms = now_ms;
        }
        self.prune_expired();
        if self
            .session
            .as_ref()
            .is_some_and(|credential| credential.expires_at_ms <= self.now_ms)
        {
            let state = HostedSyncConnectionState::Degraded {
                reason: HostedSyncDegradedReason::CredentialExpired,
            };
            self.observe("credential_expired", None, None, Some(state), "degraded");
            self.session = None;
        }
    }

    fn set_relay_available(&mut self, available: bool) {
        self.relay_available = available;
        if available {
            if self.session.is_some() {
                self.observe(
                    "recovering",
                    None,
                    None,
                    Some(HostedSyncConnectionState::Recovering),
                    "recovering",
                );
                self.observe(
                    "reconnected",
                    None,
                    None,
                    Some(HostedSyncConnectionState::Connected),
                    "connected",
                );
            }
        } else if self.session.is_some() {
            self.observe(
                "relay_unavailable",
                None,
                None,
                Some(HostedSyncConnectionState::Degraded {
                    reason: HostedSyncDegradedReason::RelayUnavailable,
                }),
                "degraded",
            );
        }
    }

    fn observations(&self) -> &[HostedSyncObservation] {
        &self.observations
    }

    fn lineage(&self) -> &[HostedSyncLineageEvidence] {
        &self.lineage
    }

    fn ensure_live_credential(&mut self) -> Result<(), HostedSyncError> {
        let Some(session) = &self.session else {
            return Err(hosted_error(
                HostedSyncErrorCode::NotConnected,
                "hosted sync transport is not connected",
                json!({}),
            ));
        };
        if session.expires_at_ms <= self.now_ms {
            self.session = None;
            let state = HostedSyncConnectionState::Degraded {
                reason: HostedSyncDegradedReason::CredentialExpired,
            };
            self.observe("credential_expired", None, None, Some(state), "degraded");
            return Err(hosted_error(
                HostedSyncErrorCode::CredentialExpired,
                "scoped hosted-relay credential expired",
                json!({}),
            ));
        }
        if !self.relay_available {
            let state = HostedSyncConnectionState::Degraded {
                reason: HostedSyncDegradedReason::RelayUnavailable,
            };
            self.observe("relay_unavailable", None, None, Some(state), "degraded");
            return Err(hosted_error(
                HostedSyncErrorCode::ProviderUnavailable,
                "hosted relay unavailable",
                json!({}),
            ));
        }
        Ok(())
    }

    fn validate_operation(
        &self,
        operation: &EncryptedSyncOperation,
    ) -> Result<(), HostedSyncError> {
        if operation.operation_id.is_empty()
            || operation.synchronization_set_id.is_empty()
            || operation.writer_id.is_empty()
            || operation.key_version_id.is_empty()
            || operation.ciphertext.is_empty()
        {
            return Err(hosted_error(
                HostedSyncErrorCode::InvalidEnvelope,
                "encrypted sync operation envelope is incomplete",
                json!({}),
            ));
        }
        if !self
            .accepted_key_versions
            .contains(&operation.key_version_id)
        {
            return Err(hosted_error(
                HostedSyncErrorCode::KeyMismatch,
                "key-version is outside the accepted active/previous window",
                json!({ "key_version_id": operation.key_version_id }),
            ));
        }
        Ok(())
    }

    fn prune_expired(&mut self) {
        let oldest_retained_ms = self.now_ms.saturating_sub(self.replay_window_ms);
        while self
            .messages
            .front()
            .is_some_and(|message| message.retained_at_ms < oldest_retained_ms)
        {
            if let Some(message) = self.messages.pop_front() {
                self.operations_by_cursor.remove(&message.cursor);
            }
        }
    }

    fn observe(
        &mut self,
        kind: &str,
        operation_id: Option<String>,
        key_version_id: Option<String>,
        connection_state: Option<HostedSyncConnectionState>,
        outcome: &str,
    ) {
        let hashed_scope = self
            .session
            .as_ref()
            .map(|credential| credential.scope.hashed());
        self.observations.push(HostedSyncObservation {
            governing_spec: HOSTED_TRANSPORT_SPEC.to_string(),
            kind: kind.to_string(),
            operation_id,
            key_version_id,
            hashed_scope,
            connection_state,
            outcome: Some(outcome.to_string()),
            latency_ms: Some(0),
        });
    }
}

fn wrap_ecca_event(operation: &EncryptedSyncOperation) -> TraverseEvent {
    TraverseEvent {
        id: format!("evt-{}", operation.operation_id),
        source: "traverse-runtime/hosted-sync".to_string(),
        event_type: "dev.traverse.datastore.sync.operation".to_string(),
        datacontenttype: "application/json".to_string(),
        time: "1970-01-01T00:00:00Z".to_string(),
        data: json!({
            "operation_id": operation.operation_id,
            "synchronization_set_id": operation.synchronization_set_id,
            "writer_id": operation.writer_id,
            "lamport_clock": operation.lamport_clock,
            "key_version_id": operation.key_version_id,
            "ciphertext_sha256": hex_digest(&operation.ciphertext),
            "ciphertext": bytes_to_hex(&operation.ciphertext),
        }),
        owner: "hosted-sync".to_string(),
        version: SYNC_PROTOCOL_SPEC.to_string(),
        lifecycle_status: LifecycleStatus::Active,
        deduplication_id: Some(operation.operation_id.clone()),
        ordering_scope: Some(operation.synchronization_set_id.clone()),
        correlation_id: operation.correlation_id.clone(),
        causation_id: operation.causation_id.clone(),
        subject_id: None,
        actor_id: Some(operation.writer_id.clone()),
    }
}

fn operation_from_ecca_event(
    event: &TraverseEvent,
) -> Result<EncryptedSyncOperation, HostedSyncError> {
    let ciphertext_hex = event
        .data
        .get("ciphertext")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            hosted_error(
                HostedSyncErrorCode::InvalidEnvelope,
                "ECCA sync envelope missing ciphertext",
                json!({}),
            )
        })?;
    let ciphertext = hex_decode(ciphertext_hex).ok_or_else(|| {
        hosted_error(
            HostedSyncErrorCode::InvalidEnvelope,
            "ECCA sync envelope ciphertext is not valid hex",
            json!({}),
        )
    })?;
    Ok(EncryptedSyncOperation {
        operation_id: required_string(&event.data, "operation_id")?,
        synchronization_set_id: required_string(&event.data, "synchronization_set_id")?,
        writer_id: required_string(&event.data, "writer_id")?,
        lamport_clock: event
            .data
            .get("lamport_clock")
            .and_then(Value::as_u64)
            .ok_or_else(|| {
                hosted_error(
                    HostedSyncErrorCode::InvalidEnvelope,
                    "ECCA sync envelope missing lamport_clock",
                    json!({}),
                )
            })?,
        correlation_id: event.correlation_id.clone(),
        causation_id: event.causation_id.clone(),
        key_version_id: required_string(&event.data, "key_version_id")?,
        ciphertext,
    })
}

fn required_string(value: &Value, field: &str) -> Result<String, HostedSyncError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            hosted_error(
                HostedSyncErrorCode::InvalidEnvelope,
                "ECCA sync envelope missing required field",
                json!({ "field": field }),
            )
        })
}

fn ably_channel_for_scope(scope: &SyncScopeId) -> String {
    // Provider-native name stays inside the adapter; observability uses hashed scope only.
    format!("ably.sync.{}", hex_digest(scope.as_str().as_bytes()))
}

fn map_ably_error(error: AblyEdgeError) -> HostedSyncError {
    match error {
        AblyEdgeError::Unauthorized => hosted_error(
            HostedSyncErrorCode::UnauthorizedScope,
            "provider rejected scoped credential or channel binding",
            json!({}),
        ),
        AblyEdgeError::Unavailable => hosted_error(
            HostedSyncErrorCode::ProviderUnavailable,
            "hosted relay unavailable",
            json!({}),
        ),
        AblyEdgeError::CursorExpired {
            oldest_available_cursor,
        } => hosted_error(
            HostedSyncErrorCode::ResyncRequired,
            "cursor expired; application sync authority catch-up required",
            json!({ "oldest_available_cursor": oldest_available_cursor }),
        ),
    }
}

fn hosted_error(code: HostedSyncErrorCode, message: &str, details: Value) -> HostedSyncError {
    HostedSyncError {
        code,
        message: message.to_string(),
        details,
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    bytes_to_hex(&digest)
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        hex.push(HEXADECIMAL_DIGITS[(byte >> 4) as usize] as char);
        hex.push(HEXADECIMAL_DIGITS[(byte & 0x0f) as usize] as char);
    }
    hex
}

fn hex_decode(input: &str) -> Option<Vec<u8>> {
    if !input.len().is_multiple_of(2) {
        return None;
    }
    let mut bytes = Vec::with_capacity(input.len() / 2);
    let chars: Vec<char> = input.chars().collect();
    for chunk in chars.chunks(2) {
        let hi = hex_nibble(chunk[0])?;
        let lo = hex_nibble(chunk[1])?;
        bytes.push((hi << 4) | lo);
    }
    Some(bytes)
}

fn hex_nibble(value: char) -> Option<u8> {
    match value {
        '0'..='9' => Some((value as u8) - b'0'),
        'a'..='f' => Some((value as u8) - b'a' + 10),
        'A'..='F' => Some((value as u8) - b'A' + 10),
        _ => None,
    }
}

fn sample_operation() -> EncryptedSyncOperation {
    EncryptedSyncOperation {
        operation_id: "op-1".to_string(),
        synchronization_set_id: "sync-set-1".to_string(),
        writer_id: "writer-a".to_string(),
        lamport_clock: 3,
        correlation_id: Some("corr-1".to_string()),
        causation_id: Some("cause-1".to_string()),
        key_version_id: "key-active".to_string(),
        ciphertext: b"ciphertext-one".to_vec(),
    }
}

fn expect_error_code<T>(
    result: &Result<T, HostedSyncError>,
    expected: HostedSyncErrorCode,
    message: &str,
) -> Result<(), String> {
    if result.as_ref().err().map(|error| error.code) == Some(expected) {
        Ok(())
    } else {
        Err(message.to_string())
    }
}

fn conformance_publish_and_auth(
    transport: &mut dyn HostedSyncTransport,
    scope: &SyncScopeId,
    operation: &EncryptedSyncOperation,
) -> Result<(), String> {
    let receipt = transport
        .publish(operation.clone())
        .map_err(|error| format!("publish failed: {:?}", error.code))?;
    if receipt.operation_id != "op-1" {
        return Err("publish receipt operation id mismatch".to_string());
    }
    let duplicate = transport
        .publish(operation.clone())
        .map_err(|error| format!("idempotent publish failed: {:?}", error.code))?;
    if duplicate.operation_id != "op-1" {
        return Err("deduplicated publish changed operation id".to_string());
    }
    expect_error_code(
        &transport.publish(EncryptedSyncOperation {
            operation_id: "op-stale-key".to_string(),
            key_version_id: "key-retired".to_string(),
            ..operation.clone()
        }),
        HostedSyncErrorCode::KeyMismatch,
        "stale key must fail with KeyMismatch",
    )?;
    expect_error_code(
        &transport.refresh_credential(HostedSyncCredential {
            token: "token-other".to_string(),
            scope: SyncScopeId::new("tenant-b/user-9/device-group-y"),
            expires_at_ms: 90_000,
        }),
        HostedSyncErrorCode::CredentialMismatch,
        "foreign scope refresh must fail with CredentialMismatch",
    )?;
    transport
        .refresh_credential(HostedSyncCredential {
            token: "token-refresh".to_string(),
            scope: scope.clone(),
            expires_at_ms: 180_000,
        })
        .map_err(|error| format!("refresh failed: {:?}", error.code))?;
    Ok(())
}

fn conformance_outage_and_replay(
    transport: &mut dyn HostedSyncTransport,
    scope: &SyncScopeId,
    operation: &EncryptedSyncOperation,
) -> Result<(), String> {
    transport.set_relay_available(false);
    if !matches!(
        transport.connection_state(),
        HostedSyncConnectionState::Degraded {
            reason: HostedSyncDegradedReason::RelayUnavailable
        }
    ) {
        return Err("relay outage must enter degraded state".to_string());
    }
    expect_error_code(
        &transport.publish(EncryptedSyncOperation {
            operation_id: "op-during-outage".to_string(),
            ..operation.clone()
        }),
        HostedSyncErrorCode::ProviderUnavailable,
        "publish during outage must be ProviderUnavailable",
    )?;
    transport.set_relay_available(true);

    let replay = transport
        .replay_from(None)
        .map_err(|error| format!("replay failed: {:?}", error.code))?;
    let HostedSyncReplayResult::Delivered {
        operations,
        cursor: first_cursor,
    } = replay
    else {
        return Err("initial replay must deliver retained operations".to_string());
    };
    if operations.len() != 1 || operations[0] != *operation {
        return Err("replay must preserve the encrypted portable envelope".to_string());
    }

    transport.advance_clock(1_000 + MIN_REPLAY_WINDOW_MS + 1);
    if matches!(
        transport.connection_state(),
        HostedSyncConnectionState::Disconnected | HostedSyncConnectionState::Degraded { .. }
    ) {
        transport
            .connect(HostedSyncCredential {
                token: "token-live".to_string(),
                scope: scope.clone(),
                expires_at_ms: 1_000 + MIN_REPLAY_WINDOW_MS + 60_000,
            })
            .map_err(|error| format!("reconnect after window failed: {:?}", error.code))?;
    }
    let expired = transport
        .replay_from(Some(&first_cursor))
        .map_err(|error| format!("expired replay failed: {:?}", error.code))?;
    if !matches!(expired, HostedSyncReplayResult::ResyncRequired { .. }) {
        return Err("expired cursor must return resync_required without a snapshot".to_string());
    }
    Ok(())
}

fn conformance_evidence(transport: &dyn HostedSyncTransport) -> Result<(), String> {
    for observation in transport.observations() {
        assert_observation_redacted(observation)?;
    }
    let Some(lineage) = transport.lineage().first() else {
        return Err("lineage evidence must be recorded for delivered operations".to_string());
    };
    if lineage.governing_spec != HOSTED_TRANSPORT_SPEC
        || lineage.protocol_spec != SYNC_PROTOCOL_SPEC
        || lineage.operation_id != "op-1"
        || !lineage.observed
    {
        return Err("lineage evidence missing required Spec 087/089 fields".to_string());
    }
    Ok(())
}

/// Runs the Spec 087 hosted-transport conformance suite against any adapter.
///
/// # Errors
///
/// Returns a descriptive failure when an assertion does not hold.
pub fn run_hosted_sync_conformance(transport: &mut dyn HostedSyncTransport) -> Result<(), String> {
    let scope = SyncScopeId::new("tenant-a/user-1/device-group-x");
    transport.advance_clock(1_000);
    transport
        .connect(HostedSyncCredential {
            token: "token-live".to_string(),
            scope: scope.clone(),
            expires_at_ms: 60_000,
        })
        .map_err(|error| format!("connect failed: {:?}", error.code))?;
    let operation = sample_operation();
    conformance_publish_and_auth(transport, &scope, &operation)?;
    conformance_outage_and_replay(transport, &scope, &operation)?;
    conformance_evidence(transport)
}

fn assert_observation_redacted(observation: &HostedSyncObservation) -> Result<(), String> {
    let rendered = serde_json::to_string(observation)
        .map_err(|error| format!("observation serialize failed: {error}"))?;
    for forbidden in [
        "ciphertext-one",
        "token-live",
        "token-refresh",
        "ably.sync.",
        "tenant-a/user-1",
    ] {
        if rendered.contains(forbidden) {
            return Err(format!(
                "observation leaked forbidden material: {forbidden}"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic)]

    use super::*;

    fn authorize_edge(edge: &mut InMemoryAblyEdge, scope: &SyncScopeId) {
        edge.authorize(&ably_channel_for_scope(scope), "token-live");
        edge.authorize(&ably_channel_for_scope(scope), "token-refresh");
    }

    #[test]
    fn in_memory_adapter_passes_hosted_sync_conformance() {
        let mut transport = InMemoryHostedSyncTransport::new();
        run_hosted_sync_conformance(&mut transport).expect("in-memory conformance");
    }

    #[test]
    fn ably_adapter_passes_identical_hosted_sync_conformance() {
        let mut edge = InMemoryAblyEdge::new();
        let scope = SyncScopeId::new("tenant-a/user-1/device-group-x");
        authorize_edge(&mut edge, &scope);
        let mut transport = AblyHostedSyncTransport::new(edge);
        run_hosted_sync_conformance(&mut transport).expect("ably conformance");
    }

    #[test]
    fn adapter_replacement_preserves_portable_envelope() {
        let operation = EncryptedSyncOperation {
            operation_id: "op-portable".to_string(),
            synchronization_set_id: "sync-set".to_string(),
            writer_id: "writer-b".to_string(),
            lamport_clock: 9,
            correlation_id: Some("c".to_string()),
            causation_id: None,
            key_version_id: "key-previous".to_string(),
            ciphertext: b"secret-bytes".to_vec(),
        };

        let mut memory = InMemoryHostedSyncTransport::new();
        memory.advance_clock(10);
        memory
            .connect(HostedSyncCredential {
                token: "token-live".to_string(),
                scope: SyncScopeId::new("scope-1"),
                expires_at_ms: 50_000,
            })
            .expect("memory connect");
        memory.publish(operation.clone()).expect("memory publish");
        let memory_replay = memory.replay_from(None).expect("memory replay");

        let mut edge = InMemoryAblyEdge::new();
        let scope = SyncScopeId::new("scope-1");
        edge.authorize(&ably_channel_for_scope(&scope), "token-live");
        let mut ably = AblyHostedSyncTransport::new(edge);
        ably.advance_clock(10);
        ably.connect(HostedSyncCredential {
            token: "token-live".to_string(),
            scope,
            expires_at_ms: 50_000,
        })
        .expect("ably connect");
        ably.publish(operation.clone()).expect("ably publish");
        let ably_replay = ably.replay_from(None).expect("ably replay");

        match (memory_replay, ably_replay) {
            (
                HostedSyncReplayResult::Delivered {
                    operations: left, ..
                },
                HostedSyncReplayResult::Delivered {
                    operations: right, ..
                },
            ) => {
                assert_eq!(left, right);
                assert_eq!(left, vec![operation]);
            }
            _ => panic!("both adapters must deliver the portable envelope"),
        }
    }

    #[test]
    fn tenancy_isolation_rejects_foreign_scope_credentials() {
        let mut transport = InMemoryHostedSyncTransport::new();
        transport.advance_clock(1);
        transport
            .connect(HostedSyncCredential {
                token: "token-a".to_string(),
                scope: SyncScopeId::new("tenant-a/scope"),
                expires_at_ms: 10_000,
            })
            .expect("connect");
        let error = transport
            .refresh_credential(HostedSyncCredential {
                token: "token-b".to_string(),
                scope: SyncScopeId::new("tenant-b/scope"),
                expires_at_ms: 10_000,
            })
            .expect_err("foreign scope must fail");
        assert_eq!(error.code, HostedSyncErrorCode::CredentialMismatch);
    }

    #[test]
    fn observations_never_include_plaintext_or_provider_channels() {
        let mut edge = InMemoryAblyEdge::new();
        let scope = SyncScopeId::new("tenant-a/user-1/device-group-x");
        edge.authorize(&ably_channel_for_scope(&scope), "token-live");
        let mut transport = AblyHostedSyncTransport::new(edge);
        transport.advance_clock(5);
        transport
            .connect(HostedSyncCredential {
                token: "token-live".to_string(),
                scope,
                expires_at_ms: 9_000,
            })
            .expect("connect");
        transport
            .publish(EncryptedSyncOperation {
                operation_id: "op-obs".to_string(),
                synchronization_set_id: "set".to_string(),
                writer_id: "w".to_string(),
                lamport_clock: 1,
                correlation_id: None,
                causation_id: None,
                key_version_id: "key-active".to_string(),
                ciphertext: b"plaintext-must-not-leak".to_vec(),
            })
            .expect("publish");
        for observation in transport.observations() {
            assert_observation_redacted(observation).expect("redaction");
            let text = serde_json::to_string(observation).expect("json");
            assert!(!text.contains("plaintext-must-not-leak"));
            assert!(!text.contains("ably.sync."));
        }
    }

    #[test]
    fn degraded_sync_is_typed_and_does_not_claim_local_store_failure() {
        // FR-010: relay outage degrades synchronization only; local DataStore
        // durability remains a separate host concern outside this port.
        let mut transport = InMemoryHostedSyncTransport::new();
        transport.advance_clock(1);
        transport
            .connect(HostedSyncCredential {
                token: "token-live".to_string(),
                scope: SyncScopeId::new("scope"),
                expires_at_ms: 10_000,
            })
            .expect("connect");
        transport.set_relay_available(false);
        assert!(matches!(
            transport.connection_state(),
            HostedSyncConnectionState::Degraded {
                reason: HostedSyncDegradedReason::RelayUnavailable
            }
        ));
        let error = transport
            .publish(EncryptedSyncOperation {
                operation_id: "op-deg".to_string(),
                synchronization_set_id: "set".to_string(),
                writer_id: "w".to_string(),
                lamport_clock: 1,
                correlation_id: None,
                causation_id: None,
                key_version_id: "key-active".to_string(),
                ciphertext: b"x".to_vec(),
            })
            .expect_err("sync publish fails while degraded");
        assert_eq!(error.code, HostedSyncErrorCode::ProviderUnavailable);
        assert_ne!(error.code, HostedSyncErrorCode::InvalidEnvelope);
    }
}

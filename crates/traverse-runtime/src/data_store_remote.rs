//! Host-injected, provider-neutral remote key-value `DataStore` adapter.
//!
//! Governed by spec `094-remote-key-value-datastore` (provider-neutral
//! contract, ADR-0024) and spec `095-s3-compatible-remote-datastore`
//! (S3-compatible conformance profile, ADR-0029). The host supplies a
//! [`RemoteDataStoreBackend`]; Traverse never selects a provider, endpoint,
//! tenant, or credential and never issues a hidden retry.

use super::{
    DataStore, DataStoreError, DataStoreErrorCode, LocalDataClassification, StateRecord,
    data_store_error, digest_for_record, serialization_error, validate_key,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::cell::RefCell;

const REMOTE_DATA_STORE_FORMAT: &str = "remote-datastore/1";
const REMOTE_CONSISTENCY_MODE: &str = "read-your-write";

/// Opaque, provider-issued concurrency token (e.g. an S3 `ETag`). Traverse
/// never interprets its contents or treats it as an integrity digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteVersionToken(pub String);

/// One host-fetched remote object: its stored bytes and current version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteObject {
    pub bytes: Vec<u8>,
    pub version: RemoteVersionToken,
}

/// Stable, secret-free host-backend failure categories (spec 094 FR-007 /
/// spec 095 FR-008, minus the outcomes that are not backend failures).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteBackendFailure {
    Unavailable,
    Timeout,
    Unauthorized,
    ScopeDenied,
    BackendFailed,
}

/// Outcome of a conditional write or delete against the host backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteWriteOutcome {
    /// The provider confirmed the conditional operation.
    Acknowledged { retry_count: u32 },
    /// The precondition (expected version) did not match (spec 095 FR-003).
    Conflict,
    /// A timeout or connection loss occurred after possible submission; the
    /// operation may or may not have reached the provider (spec 094 FR-003).
    Unknown { retry_count: u32 },
    /// The backend reported one of the stable failure categories.
    Failed(RemoteBackendFailure),
}

/// Host-provided, provider-specific transport. The host owns endpoint
/// selection, authentication, tenant mapping, and retry policy; Traverse
/// only validates envelope/integrity semantics and projects safe outcomes.
pub trait RemoteDataStoreBackend {
    /// Reads the current stored object and its opaque version, if any.
    ///
    /// # Errors
    ///
    /// Returns [`RemoteBackendFailure`] when the backend cannot complete the
    /// read.
    fn get(&self, key: &str) -> Result<Option<RemoteObject>, RemoteBackendFailure>;

    /// Writes `bytes` for `key` conditioned on `expected_version` (`None`
    /// means the key was absent at the time of the preceding read).
    fn put_conditional(
        &mut self,
        key: &str,
        bytes: Vec<u8>,
        expected_version: Option<RemoteVersionToken>,
    ) -> RemoteWriteOutcome;

    /// Deletes `key` conditioned on `expected_version` (`None` means the key
    /// was absent at the time of the preceding read).
    fn delete_conditional(
        &mut self,
        key: &str,
        expected_version: Option<RemoteVersionToken>,
    ) -> RemoteWriteOutcome;
}

/// Safe evidence for one remote operation (spec 094 FR-009): operation,
/// outcome, classification, consistency mode, retry count, and stable
/// failure code only. Never a key, value, credential, endpoint, or tenant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteOperationEvidence {
    pub operation: &'static str,
    pub outcome: &'static str,
    pub classification: LocalDataClassification,
    pub consistency_mode: &'static str,
    pub retry_count: u32,
    pub code: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RemoteDataStoreEnvelope {
    format: String,
    classification: LocalDataClassification,
    digest: String,
    record: StateRecord,
}

/// Provider-neutral remote key-value `DataStore` adapter (spec 094),
/// conformant to the S3-compatible profile (spec 095) when `B` implements
/// conditional writes/deletes over an S3-compatible backend.
pub struct RemoteKeyValueDataStore<B: RemoteDataStoreBackend> {
    backend: B,
    classification: LocalDataClassification,
    last_evidence: RefCell<Option<RemoteOperationEvidence>>,
}

impl<B: RemoteDataStoreBackend> RemoteKeyValueDataStore<B> {
    #[must_use]
    pub fn new(backend: B, classification: LocalDataClassification) -> Self {
        Self {
            backend,
            classification,
            last_evidence: RefCell::new(None),
        }
    }

    /// Safe evidence for the most recently completed operation, if any.
    #[must_use]
    pub fn last_evidence(&self) -> Option<RemoteOperationEvidence> {
        self.last_evidence.borrow().clone()
    }

    fn record_evidence(
        &self,
        operation: &'static str,
        outcome: &'static str,
        retry_count: u32,
        code: Option<&'static str>,
    ) {
        *self.last_evidence.borrow_mut() = Some(RemoteOperationEvidence {
            operation,
            outcome,
            classification: self.classification,
            consistency_mode: REMOTE_CONSISTENCY_MODE,
            retry_count,
            code,
        });
    }

    fn decode_envelope(&self, bytes: &[u8]) -> Result<StateRecord, DataStoreError> {
        let envelope: RemoteDataStoreEnvelope = serde_json::from_slice(bytes)
            .map_err(|_| remote_integrity_error("malformed_envelope"))?;
        if envelope.format != REMOTE_DATA_STORE_FORMAT {
            return Err(remote_integrity_error("unknown_format_version"));
        }
        if envelope.classification != self.classification {
            return Err(remote_integrity_error("classification_mismatch"));
        }
        let expected = digest_for_record(&envelope.record)?;
        if expected != envelope.digest {
            return Err(remote_integrity_error("digest_mismatch"));
        }
        Ok(envelope.record)
    }

    fn current_version(
        &self,
        operation: &'static str,
        key: &str,
    ) -> Result<Option<RemoteVersionToken>, DataStoreError> {
        match self.backend.get(key) {
            Ok(object) => Ok(object.map(|object| object.version)),
            Err(failure) => {
                let code = remote_failure_code(failure);
                self.record_evidence(operation, "failed", 0, Some(code));
                Err(remote_backend_error(failure))
            }
        }
    }

    fn handle_write_outcome(
        &self,
        operation: &'static str,
        outcome: RemoteWriteOutcome,
    ) -> Result<(), DataStoreError> {
        match outcome {
            RemoteWriteOutcome::Acknowledged { retry_count } => {
                self.record_evidence(operation, "acknowledged", retry_count, None);
                Ok(())
            }
            RemoteWriteOutcome::Conflict => {
                self.record_evidence(operation, "conflict", 0, Some("remote_conflict"));
                Err(remote_conflict_error())
            }
            RemoteWriteOutcome::Unknown { retry_count } => {
                self.record_evidence(
                    operation,
                    "outcome_unknown",
                    retry_count,
                    Some("remote_outcome_unknown"),
                );
                Err(remote_outcome_unknown_error(retry_count))
            }
            RemoteWriteOutcome::Failed(failure) => {
                let code = remote_failure_code(failure);
                self.record_evidence(operation, "failed", 0, Some(code));
                Err(remote_backend_error(failure))
            }
        }
    }
}

impl<B: RemoteDataStoreBackend> DataStore for RemoteKeyValueDataStore<B> {
    fn read(&self, key: &str) -> Result<Option<StateRecord>, DataStoreError> {
        validate_key(key)?;
        match self.backend.get(key) {
            Ok(None) => {
                self.record_evidence("read", "not_found", 0, None);
                Ok(None)
            }
            Ok(Some(object)) => match self.decode_envelope(&object.bytes) {
                Ok(record) => {
                    self.record_evidence("read", "acknowledged", 0, None);
                    Ok(Some(record))
                }
                Err(error) => {
                    self.record_evidence(
                        "read",
                        "integrity_failed",
                        0,
                        Some("remote_integrity_failed"),
                    );
                    Err(error)
                }
            },
            Err(failure) => {
                let code = remote_failure_code(failure);
                self.record_evidence("read", "failed", 0, Some(code));
                Err(remote_backend_error(failure))
            }
        }
    }

    fn write(&mut self, record: StateRecord) -> Result<(), DataStoreError> {
        validate_key(&record.key)?;
        let expected_version = self.current_version("write", &record.key)?;
        let digest = digest_for_record(&record)?;
        let envelope = RemoteDataStoreEnvelope {
            format: REMOTE_DATA_STORE_FORMAT.to_string(),
            classification: self.classification,
            digest,
            record: record.clone(),
        };
        let bytes = serde_json::to_vec(&envelope)
            .map_err(|error| serialization_error("serialize remote envelope", &error))?;
        let outcome = self
            .backend
            .put_conditional(&record.key, bytes, expected_version);
        self.handle_write_outcome("write", outcome)
    }

    fn delete(&mut self, key: &str) -> Result<(), DataStoreError> {
        validate_key(key)?;
        let expected_version = self.current_version("delete", key)?;
        let outcome = self.backend.delete_conditional(key, expected_version);
        self.handle_write_outcome("delete", outcome)
    }

    fn list_keys(&self) -> Result<Vec<String>, DataStoreError> {
        self.record_evidence("list_keys", "unsupported", 0, Some("remote_backend_failed"));
        Err(data_store_error(
            DataStoreErrorCode::RemoteBackendFailed,
            "remote_backend_failed",
            json!({ "reason": "remote_scan_not_supported_in_v1" }),
        ))
    }
}

fn remote_failure_code(failure: RemoteBackendFailure) -> &'static str {
    match failure {
        RemoteBackendFailure::Unavailable => "remote_unavailable",
        RemoteBackendFailure::Timeout => "remote_timeout",
        RemoteBackendFailure::Unauthorized => "remote_unauthorized",
        RemoteBackendFailure::ScopeDenied => "remote_scope_denied",
        RemoteBackendFailure::BackendFailed => "remote_backend_failed",
    }
}

fn remote_backend_error(failure: RemoteBackendFailure) -> DataStoreError {
    let code = match failure {
        RemoteBackendFailure::Unavailable => DataStoreErrorCode::RemoteUnavailable,
        RemoteBackendFailure::Timeout => DataStoreErrorCode::RemoteTimeout,
        RemoteBackendFailure::Unauthorized => DataStoreErrorCode::RemoteUnauthorized,
        RemoteBackendFailure::ScopeDenied => DataStoreErrorCode::RemoteScopeDenied,
        RemoteBackendFailure::BackendFailed => DataStoreErrorCode::RemoteBackendFailed,
    };
    data_store_error(code, remote_failure_code(failure), json!({}))
}

fn remote_conflict_error() -> DataStoreError {
    data_store_error(
        DataStoreErrorCode::RemoteConflict,
        "remote_conflict",
        json!({}),
    )
}

fn remote_outcome_unknown_error(retry_count: u32) -> DataStoreError {
    data_store_error(
        DataStoreErrorCode::RemoteOutcomeUnknown,
        "remote_outcome_unknown",
        json!({ "retry_count": retry_count }),
    )
}

fn remote_integrity_error(reason: &str) -> DataStoreError {
    data_store_error(
        DataStoreErrorCode::RemoteIntegrityFailed,
        "remote_integrity_failed",
        json!({ "reason": reason }),
    )
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[derive(Debug, Clone)]
    struct StoredObject {
        bytes: Vec<u8>,
        version: u64,
    }

    /// An in-memory, S3-compatible-shaped test double: version tokens are
    /// monotonically increasing per key, conditional writes/deletes compare
    /// the caller's expected token against the current one, and each of
    /// `get`/`put_conditional`/`delete_conditional` can be independently
    /// scripted to fail (outage, timeout, denial) or return an ambiguous
    /// outcome.
    #[derive(Default)]
    struct FakeS3Backend {
        objects: BTreeMap<String, StoredObject>,
        next_version: u64,
        fail_get: Option<RemoteBackendFailure>,
        fail_put: Option<RemoteBackendFailure>,
        fail_delete: Option<RemoteBackendFailure>,
        force_conflict: bool,
        force_unknown_retry_count: Option<u32>,
    }

    impl FakeS3Backend {
        fn version_token(version: u64) -> RemoteVersionToken {
            RemoteVersionToken(format!("v{version}"))
        }

        fn current_token(&self, key: &str) -> Option<RemoteVersionToken> {
            self.objects
                .get(key)
                .map(|object| Self::version_token(object.version))
        }
    }

    impl RemoteDataStoreBackend for FakeS3Backend {
        fn get(&self, key: &str) -> Result<Option<RemoteObject>, RemoteBackendFailure> {
            if let Some(failure) = self.fail_get {
                return Err(failure);
            }
            Ok(self.objects.get(key).map(|object| RemoteObject {
                bytes: object.bytes.clone(),
                version: Self::version_token(object.version),
            }))
        }

        fn put_conditional(
            &mut self,
            key: &str,
            bytes: Vec<u8>,
            expected_version: Option<RemoteVersionToken>,
        ) -> RemoteWriteOutcome {
            if let Some(failure) = self.fail_put {
                return RemoteWriteOutcome::Failed(failure);
            }
            if self.force_conflict || expected_version != self.current_token(key) {
                self.force_conflict = false;
                return RemoteWriteOutcome::Conflict;
            }
            if let Some(retry_count) = self.force_unknown_retry_count.take() {
                return RemoteWriteOutcome::Unknown { retry_count };
            }
            self.next_version += 1;
            let version = self.next_version;
            self.objects
                .insert(key.to_string(), StoredObject { bytes, version });
            RemoteWriteOutcome::Acknowledged { retry_count: 0 }
        }

        fn delete_conditional(
            &mut self,
            key: &str,
            expected_version: Option<RemoteVersionToken>,
        ) -> RemoteWriteOutcome {
            if let Some(failure) = self.fail_delete {
                return RemoteWriteOutcome::Failed(failure);
            }
            if self.force_conflict || expected_version != self.current_token(key) {
                self.force_conflict = false;
                return RemoteWriteOutcome::Conflict;
            }
            self.objects.remove(key);
            RemoteWriteOutcome::Acknowledged { retry_count: 0 }
        }
    }

    fn record(key: &str, value: &str) -> StateRecord {
        StateRecord {
            key: key.to_string(),
            value: json!({ "v": value }),
            lamport_clock: 1,
            writer_id: "writer-a".to_string(),
        }
    }

    fn store_with_backend(
        backend: FakeS3Backend,
        classification: LocalDataClassification,
    ) -> RemoteKeyValueDataStore<FakeS3Backend> {
        RemoteKeyValueDataStore::new(backend, classification)
    }

    #[test]
    fn write_then_read_your_write_round_trips() {
        let mut store =
            store_with_backend(FakeS3Backend::default(), LocalDataClassification::Public);
        store.write(record("alpha", "one")).expect("write succeeds");
        let read = store.read("alpha").expect("read succeeds");
        assert_eq!(read, Some(record("alpha", "one")));
        let evidence = store.last_evidence().expect("evidence recorded");
        assert_eq!(evidence.operation, "read");
        assert_eq!(evidence.outcome, "acknowledged");
        assert_eq!(evidence.consistency_mode, "read-your-write");
    }

    #[test]
    fn read_of_missing_key_returns_none_without_error() {
        let store = store_with_backend(FakeS3Backend::default(), LocalDataClassification::Public);
        assert_eq!(store.read("missing").expect("read succeeds"), None);
        assert_eq!(store.last_evidence().unwrap().outcome, "not_found");
    }

    #[test]
    fn ambiguous_write_outcome_is_reported_as_outcome_unknown_not_success() {
        let backend = FakeS3Backend {
            force_unknown_retry_count: Some(2),
            ..Default::default()
        };
        let mut store = store_with_backend(backend, LocalDataClassification::Public);
        let error = store
            .write(record("alpha", "one"))
            .expect_err("ambiguous outcome fails");
        assert_eq!(error.code, DataStoreErrorCode::RemoteOutcomeUnknown);
        assert_eq!(error.message, "remote_outcome_unknown");
        let evidence = store.last_evidence().unwrap();
        assert_eq!(evidence.outcome, "outcome_unknown");
        assert_eq!(evidence.retry_count, 2);
        assert_eq!(evidence.code, Some("remote_outcome_unknown"));
        assert_eq!(store.read("alpha").expect("read succeeds"), None);
    }

    #[test]
    fn concurrent_write_conflict_never_overwrites_or_retries() {
        let mut store =
            store_with_backend(FakeS3Backend::default(), LocalDataClassification::Public);
        store
            .write(record("alpha", "one"))
            .expect("first write succeeds");
        store.backend.force_conflict = true;
        let error = store
            .write(record("alpha", "two"))
            .expect_err("conflicting write fails closed");
        assert_eq!(error.code, DataStoreErrorCode::RemoteConflict);
        assert_eq!(
            store.read("alpha").expect("read succeeds"),
            Some(record("alpha", "one")),
            "a conflicting write must never overwrite the existing value"
        );
    }

    #[test]
    fn delete_conflict_is_reported_and_leaves_the_record_intact() {
        let mut store =
            store_with_backend(FakeS3Backend::default(), LocalDataClassification::Public);
        store.write(record("alpha", "one")).expect("write succeeds");
        store.backend.force_conflict = true;
        let error = store
            .delete("alpha")
            .expect_err("conflicting delete fails closed");
        assert_eq!(error.code, DataStoreErrorCode::RemoteConflict);
        assert!(store.read("alpha").expect("read succeeds").is_some());
    }

    #[test]
    fn delete_removes_the_record_and_subsequent_read_returns_none() {
        let mut store =
            store_with_backend(FakeS3Backend::default(), LocalDataClassification::Public);
        store.write(record("alpha", "one")).expect("write succeeds");
        store.delete("alpha").expect("delete succeeds");
        let evidence = store.last_evidence().expect("evidence recorded");
        assert_eq!(evidence.operation, "delete");
        assert_eq!(evidence.outcome, "acknowledged");
        assert_eq!(store.read("alpha").expect("read succeeds"), None);
    }

    #[test]
    fn denied_scope_and_unauthorized_map_to_stable_codes() {
        let scope_denied = FakeS3Backend {
            fail_get: Some(RemoteBackendFailure::ScopeDenied),
            ..Default::default()
        };
        let scope_store = store_with_backend(scope_denied, LocalDataClassification::Public);
        let error = scope_store.read("alpha").expect_err("denied scope fails");
        assert_eq!(error.code, DataStoreErrorCode::RemoteScopeDenied);
        assert_eq!(error.message, "remote_scope_denied");

        let unauthorized = FakeS3Backend {
            fail_get: Some(RemoteBackendFailure::Unauthorized),
            ..Default::default()
        };
        let auth_store = store_with_backend(unauthorized, LocalDataClassification::Public);
        let error = auth_store.read("alpha").expect_err("unauthorized fails");
        assert_eq!(error.code, DataStoreErrorCode::RemoteUnauthorized);
        assert_eq!(error.message, "remote_unauthorized");
    }

    #[test]
    fn outage_and_timeout_map_to_stable_codes() {
        let unavailable = FakeS3Backend {
            fail_get: Some(RemoteBackendFailure::Unavailable),
            ..Default::default()
        };
        let store = store_with_backend(unavailable, LocalDataClassification::Public);
        assert_eq!(
            store.read("alpha").expect_err("outage fails").code,
            DataStoreErrorCode::RemoteUnavailable
        );

        let timeout = FakeS3Backend {
            fail_get: Some(RemoteBackendFailure::Timeout),
            ..Default::default()
        };
        let store = store_with_backend(timeout, LocalDataClassification::Public);
        assert_eq!(
            store.read("alpha").expect_err("timeout fails").code,
            DataStoreErrorCode::RemoteTimeout
        );
    }

    #[test]
    fn backend_failure_during_write_precondition_lookup_is_reported() {
        let backend = FakeS3Backend {
            fail_get: Some(RemoteBackendFailure::BackendFailed),
            ..Default::default()
        };
        let mut store = store_with_backend(backend, LocalDataClassification::Public);
        let error = store
            .write(record("alpha", "one"))
            .expect_err("precondition lookup failure surfaces");
        assert_eq!(error.code, DataStoreErrorCode::RemoteBackendFailed);
    }

    #[test]
    fn backend_failure_during_put_after_precondition_check_is_reported() {
        let mut store =
            store_with_backend(FakeS3Backend::default(), LocalDataClassification::Public);
        store.backend.fail_put = Some(RemoteBackendFailure::BackendFailed);
        let error = store
            .write(record("alpha", "one"))
            .expect_err("backend failure during put surfaces");
        assert_eq!(error.code, DataStoreErrorCode::RemoteBackendFailed);
        let evidence = store.last_evidence().unwrap();
        assert_eq!(evidence.operation, "write");
        assert_eq!(evidence.outcome, "failed");
    }

    #[test]
    fn backend_failure_during_delete_after_precondition_check_is_reported() {
        let mut store =
            store_with_backend(FakeS3Backend::default(), LocalDataClassification::Public);
        store.write(record("alpha", "one")).expect("write succeeds");
        store.backend.fail_delete = Some(RemoteBackendFailure::Unavailable);
        let error = store
            .delete("alpha")
            .expect_err("backend failure during delete surfaces");
        assert_eq!(error.code, DataStoreErrorCode::RemoteUnavailable);
    }

    #[test]
    fn malformed_remote_bytes_fail_closed() {
        let mut store =
            store_with_backend(FakeS3Backend::default(), LocalDataClassification::Public);
        store.write(record("alpha", "one")).expect("write succeeds");
        if let Some(object) = store.backend.objects.get_mut("alpha") {
            object.bytes = b"not json".to_vec();
        }
        let error = store
            .read("alpha")
            .expect_err("malformed bytes fail closed");
        assert_eq!(error.code, DataStoreErrorCode::RemoteIntegrityFailed);
    }

    #[test]
    fn unknown_envelope_format_fails_closed() {
        let mut store =
            store_with_backend(FakeS3Backend::default(), LocalDataClassification::Public);
        store.write(record("alpha", "one")).expect("write succeeds");
        let envelope = RemoteDataStoreEnvelope {
            format: "remote-datastore/999".to_string(),
            classification: LocalDataClassification::Public,
            digest: "sha256:00".to_string(),
            record: record("alpha", "one"),
        };
        let bytes = serde_json::to_vec(&envelope).expect("serialize envelope");
        if let Some(object) = store.backend.objects.get_mut("alpha") {
            object.bytes = bytes;
        }
        let error = store
            .read("alpha")
            .expect_err("unknown format fails closed");
        assert_eq!(error.code, DataStoreErrorCode::RemoteIntegrityFailed);
    }

    #[test]
    fn tampered_remote_bytes_fail_closed_with_integrity_error() {
        let mut store =
            store_with_backend(FakeS3Backend::default(), LocalDataClassification::Public);
        store.write(record("alpha", "one")).expect("write succeeds");
        let envelope = RemoteDataStoreEnvelope {
            format: REMOTE_DATA_STORE_FORMAT.to_string(),
            classification: LocalDataClassification::Public,
            digest: "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
            record: record("alpha", "tampered"),
        };
        let bytes = serde_json::to_vec(&envelope).expect("serialize envelope");
        if let Some(object) = store.backend.objects.get_mut("alpha") {
            object.bytes = bytes;
        }
        let error = store.read("alpha").expect_err("tampered bytes fail closed");
        assert_eq!(error.code, DataStoreErrorCode::RemoteIntegrityFailed);
    }

    #[test]
    fn classification_mismatch_fails_closed() {
        let mut store =
            store_with_backend(FakeS3Backend::default(), LocalDataClassification::Private);
        store.write(record("alpha", "one")).expect("write succeeds");
        let backend = std::mem::take(&mut store.backend);
        let mismatched = store_with_backend(backend, LocalDataClassification::Public);
        let error = mismatched
            .read("alpha")
            .expect_err("classification mismatch fails closed");
        assert_eq!(error.code, DataStoreErrorCode::RemoteIntegrityFailed);
    }

    #[test]
    fn list_keys_is_explicitly_unsupported_in_v1() {
        let store = store_with_backend(FakeS3Backend::default(), LocalDataClassification::Public);
        let error = store.list_keys().expect_err("scan is out of scope for v1");
        assert_eq!(error.code, DataStoreErrorCode::RemoteBackendFailed);
    }

    #[test]
    fn evidence_never_contains_keys_values_or_credentials() {
        let mut store =
            store_with_backend(FakeS3Backend::default(), LocalDataClassification::Public);
        store
            .write(record("super-secret-key", "super-secret-value"))
            .expect("write succeeds");
        let evidence = store.last_evidence().expect("evidence recorded");
        let debug = format!("{evidence:?}");
        assert!(!debug.contains("super-secret-key"));
        assert!(!debug.contains("super-secret-value"));
    }

    #[test]
    fn invalid_key_is_rejected_before_touching_the_backend() {
        let store = store_with_backend(FakeS3Backend::default(), LocalDataClassification::Public);
        let error = store.read("has a space").expect_err("invalid key rejected");
        assert_eq!(error.code, DataStoreErrorCode::InvalidKey);
    }
}

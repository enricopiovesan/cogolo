//! Portable state access for Traverse capabilities.
//!
//! General operations are governed by spec `032-universal-data-access`; the
//! local-file adapter durability boundary is governed by spec
//! `518-durable-local-datastore`.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::Write;
use std::path::PathBuf;
use traverse_contracts::CapabilityContract;

const DATA_STORE_SPEC: &str = "032-universal-data-access";
const LOCAL_DATA_STORE_FORMAT: &str = "local-datastore/1";
const LOCAL_DATA_STORE_LOCK_FILE: &str = ".traverse-datastore.lock";
const HEXADECIMAL_DIGITS: &[u8; 16] = b"0123456789abcdef";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateRecord {
    pub key: String,
    pub value: Value,
    pub lamport_clock: u64,
    pub writer_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeDecision {
    pub key: String,
    pub winning_writer_id: String,
    pub winning_lamport_clock: u64,
    pub resolution_rule: ConflictResolutionRule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolutionRule {
    OnlyLocal,
    OnlyRemote,
    HigherLamportClock,
    WriterIdentityTieBreak,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncReport {
    pub governing_spec: String,
    pub decisions: Vec<MergeDecision>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataStoreError {
    pub code: DataStoreErrorCode,
    pub message: String,
    pub details: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataStoreErrorCode {
    SchemaValidationError,
    NoStateSchemaDeclared,
    LamportClockOverflow,
    InvalidKey,
    IoFailure,
    SerializationFailure,
    SyncFailure,
    IntegrityCheckFailed,
    StoreLocked,
    DurabilityCommitFailed,
}

/// Classification recorded with each locally durable record.
///
/// This is metadata only. Encryption and key management are intentionally
/// deferred to a successor decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalDataClassification {
    Public,
    Private,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct LocalDataStoreEnvelope {
    format: String,
    classification: LocalDataClassification,
    record: StateRecord,
    digest: String,
}

pub trait DataStore {
    /// Reads a stored state record.
    ///
    /// # Errors
    ///
    /// Returns [`DataStoreError`] when the adapter cannot read the key.
    fn read(&self, key: &str) -> Result<Option<StateRecord>, DataStoreError>;

    /// Writes a stamped state record.
    ///
    /// # Errors
    ///
    /// Returns [`DataStoreError`] when the adapter cannot persist the record.
    fn write(&mut self, record: StateRecord) -> Result<(), DataStoreError>;

    /// Deletes a stored state record.
    ///
    /// # Errors
    ///
    /// Returns [`DataStoreError`] when the adapter cannot delete the key.
    fn delete(&mut self, key: &str) -> Result<(), DataStoreError>;

    /// Lists stored state keys.
    ///
    /// # Errors
    ///
    /// Returns [`DataStoreError`] when the adapter cannot enumerate keys.
    fn list_keys(&self) -> Result<Vec<String>, DataStoreError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LamportClock {
    writer_id: String,
    value: u64,
}

impl LamportClock {
    #[must_use]
    pub fn new(writer_id: impl Into<String>) -> Self {
        Self {
            writer_id: writer_id.into(),
            value: 0,
        }
    }

    #[must_use]
    pub fn with_value(writer_id: impl Into<String>, value: u64) -> Self {
        Self {
            writer_id: writer_id.into(),
            value,
        }
    }

    fn next(&mut self) -> Result<u64, DataStoreError> {
        let next = self.value.checked_add(1).ok_or_else(|| {
            data_store_error(
                DataStoreErrorCode::LamportClockOverflow,
                "lamport clock overflow",
                json!({ "writer_id": self.writer_id }),
            )
        })?;
        self.value = next;
        Ok(next)
    }
}

pub struct RuntimeDataStore<A> {
    adapter: A,
    clock: LamportClock,
}

impl<A: DataStore> RuntimeDataStore<A> {
    #[must_use]
    pub fn new(adapter: A, writer_id: impl Into<String>) -> Self {
        Self {
            adapter,
            clock: LamportClock::new(writer_id),
        }
    }

    #[must_use]
    pub fn with_clock(adapter: A, clock: LamportClock) -> Self {
        Self { adapter, clock }
    }

    /// Reads and validates a state value by key.
    ///
    /// # Errors
    ///
    /// Returns [`DataStoreError`] when the key is invalid, the adapter cannot
    /// read the key, or the stored value violates the contract state schema.
    pub fn read(
        &self,
        contract: &CapabilityContract,
        key: &str,
    ) -> Result<Option<Value>, DataStoreError> {
        validate_key(key)?;
        if contract.state_schema.is_none() {
            return Ok(None);
        }
        self.adapter.read(key).and_then(|record| {
            record
                .map(|record| {
                    validate_state_write(contract, key, &record.value)?;
                    Ok(record.value)
                })
                .transpose()
        })
    }

    /// Validates, stamps, and writes a state value for a capability contract.
    ///
    /// # Errors
    ///
    /// Returns [`DataStoreError`] when the key is invalid, no state schema is
    /// declared, schema validation fails, the Lamport clock overflows, or the
    /// adapter cannot persist the stamped record.
    pub fn write(
        &mut self,
        contract: &CapabilityContract,
        key: &str,
        value: Value,
    ) -> Result<StateRecord, DataStoreError> {
        validate_state_write(contract, key, &value)?;
        let record = StateRecord {
            key: key.to_string(),
            value,
            lamport_clock: self.clock.next()?,
            writer_id: self.clock.writer_id.clone(),
        };
        self.adapter.write(record.clone())?;
        Ok(record)
    }

    /// Deletes a state value by key.
    ///
    /// # Errors
    ///
    /// Returns [`DataStoreError`] when the adapter cannot delete the key.
    pub fn delete(&mut self, key: &str) -> Result<(), DataStoreError> {
        self.adapter.delete(key)
    }

    /// Lists state keys.
    ///
    /// # Errors
    ///
    /// Returns [`DataStoreError`] when the adapter cannot enumerate keys.
    pub fn list_keys(&self) -> Result<Vec<String>, DataStoreError> {
        self.adapter.list_keys()
    }

    /// Triggers explicit sync after a reconnect event.
    ///
    /// # Errors
    ///
    /// Returns [`DataStoreError`] when either adapter cannot read, write, list,
    /// or restore state during sync.
    pub fn sync_on_reconnect(
        &mut self,
        remote: &mut dyn DataStore,
    ) -> Result<SyncReport, DataStoreError> {
        sync_adapters(&mut self.adapter, remote)
    }

    pub fn into_inner(self) -> A {
        self.adapter
    }
}

#[derive(Debug)]
pub struct LocalFileDataStore {
    root: PathBuf,
    classification: LocalDataClassification,
    _lock_file: File,
}

impl LocalFileDataStore {
    /// Creates a local filesystem-backed data store rooted at `root`.
    ///
    /// # Errors
    ///
    /// Returns [`DataStoreError`] when the root directory cannot be created or
    /// another process owns the store.
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, DataStoreError> {
        Self::with_classification(root, LocalDataClassification::Private)
    }

    /// Creates a local filesystem-backed data store with explicit persisted
    /// record classification.
    ///
    /// # Errors
    ///
    /// Returns [`DataStoreError`] when the root directory cannot be created or
    /// another process owns the store.
    pub fn with_classification(
        root: impl Into<PathBuf>,
        classification: LocalDataClassification,
    ) -> Result<Self, DataStoreError> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(|error| io_error("create data store root", &error))?;
        let lock_path = root.join(LOCAL_DATA_STORE_LOCK_FILE);
        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|error| io_error("open data store lock", &error))?;
        lock_file.try_lock().map_err(lock_error)?;
        Ok(Self {
            root,
            classification,
            _lock_file: lock_file,
        })
    }

    fn path_for_key(&self, key: &str) -> Result<PathBuf, DataStoreError> {
        validate_key(key)?;
        Ok(self.root.join(format!("{key}.json")))
    }

    fn temporary_path_for_key(&self, key: &str) -> PathBuf {
        self.root.join(format!(".{key}.{}.tmp", std::process::id()))
    }

    fn sync_root(&self) -> Result<(), DataStoreError> {
        File::open(&self.root)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| durability_error(&self.root, "parent_directory", &error))
    }
}

impl DataStore for LocalFileDataStore {
    fn read(&self, key: &str) -> Result<Option<StateRecord>, DataStoreError> {
        let path = self.path_for_key(key)?;
        if !path.exists() {
            return Ok(None);
        }
        let text =
            fs::read_to_string(&path).map_err(|error| io_error("read state record", &error))?;
        let value: Value =
            serde_json::from_str(&text).map_err(|_| integrity_error("malformed_envelope"))?;
        if value.get("format").is_none() {
            return Err(integrity_error("legacy_unverified"));
        }
        let envelope: LocalDataStoreEnvelope =
            serde_json::from_value(value).map_err(|_| integrity_error("malformed_envelope"))?;
        if envelope.format != LOCAL_DATA_STORE_FORMAT {
            return Err(integrity_error("unknown_format_version"));
        }
        let expected_digest = digest_for_record(&envelope.record)?;
        if envelope.digest != expected_digest {
            return Err(integrity_error("digest_mismatch"));
        }
        Ok(Some(envelope.record))
    }

    fn write(&mut self, record: StateRecord) -> Result<(), DataStoreError> {
        let path = self.path_for_key(&record.key)?;
        let envelope = LocalDataStoreEnvelope {
            format: LOCAL_DATA_STORE_FORMAT.to_string(),
            classification: self.classification,
            digest: digest_for_record(&record)?,
            record,
        };
        let text = serde_json::to_vec(&envelope)
            .map_err(|error| serialization_error("serialize state record envelope", &error))?;
        let temporary_path = self.temporary_path_for_key(&envelope.record.key);
        let write_result = (|| {
            let mut temporary_file = File::create(&temporary_path)
                .map_err(|error| io_error("create temporary state record", &error))?;
            temporary_file
                .write_all(&text)
                .map_err(|error| io_error("write temporary state record", &error))?;
            temporary_file
                .sync_all()
                .map_err(|error| durability_error(&self.root, "temporary_file", &error))?;
            fs::rename(&temporary_path, &path)
                .map_err(|error| io_error("atomically commit state record", &error))?;
            self.sync_root()
        })();
        discard_temporary_record(&temporary_path);
        write_result
    }

    fn delete(&mut self, key: &str) -> Result<(), DataStoreError> {
        let path = self.path_for_key(key)?;
        match fs::remove_file(path) {
            Ok(()) => self.sync_root(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(io_error("delete state record", &error)),
        }
    }

    fn list_keys(&self) -> Result<Vec<String>, DataStoreError> {
        let mut keys = Vec::new();
        for entry in
            fs::read_dir(&self.root).map_err(|error| io_error("list state keys", &error))?
        {
            let entry = entry.map_err(|error| io_error("read state key entry", &error))?;
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }
            if let Some(key) = path.file_stem().and_then(|stem| stem.to_str()) {
                keys.push(key.to_string());
            }
        }
        keys.sort();
        Ok(keys)
    }
}

fn digest_for_record(record: &StateRecord) -> Result<String, DataStoreError> {
    let canonical = serde_json::to_vec(record)
        .map_err(|error| serialization_error("serialize canonical state record", &error))?;
    let digest = Sha256::digest(canonical);
    let mut hexadecimal = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hexadecimal.push(char::from(HEXADECIMAL_DIGITS[usize::from(byte >> 4)]));
        hexadecimal.push(char::from(HEXADECIMAL_DIGITS[usize::from(byte & 0x0f)]));
    }
    Ok(format!("sha256:{hexadecimal}"))
}

fn lock_error(error: TryLockError) -> DataStoreError {
    match error {
        TryLockError::WouldBlock => data_store_error(
            DataStoreErrorCode::StoreLocked,
            "store_locked",
            json!({ "reason": "exclusive_owner_active" }),
        ),
        TryLockError::Error(error) if error.kind() == std::io::ErrorKind::Unsupported => {
            data_store_error(
                DataStoreErrorCode::IoFailure,
                "storage_io_failed",
                json!({ "operation": "acquire_lock", "reason": "locking_unsupported" }),
            )
        }
        TryLockError::Error(_) => data_store_error(
            DataStoreErrorCode::IoFailure,
            "storage_io_failed",
            json!({ "operation": "acquire_lock", "reason": "lock_acquisition_failed" }),
        ),
    }
}

fn durability_error(root: &PathBuf, stage: &str, error: &std::io::Error) -> DataStoreError {
    data_store_error(
        DataStoreErrorCode::DurabilityCommitFailed,
        "durability_commit_failed",
        json!({ "root": root, "stage": stage, "reason": error.to_string() }),
    )
}

fn discard_temporary_record(path: &PathBuf) {
    if path.exists() {
        let _ignored = fs::remove_file(path).is_ok();
    }
}

fn integrity_error(reason: &str) -> DataStoreError {
    data_store_error(
        DataStoreErrorCode::IntegrityCheckFailed,
        "integrity_check_failed",
        json!({ "reason": reason }),
    )
}

/// Validates a capability state write against the contract-declared state schema.
///
/// # Errors
///
/// Returns [`DataStoreError`] when the key is invalid, the contract does not
/// declare a state schema, the key is not declared by the schema, or the value
/// does not match the declared key schema.
pub fn validate_state_write(
    contract: &CapabilityContract,
    key: &str,
    value: &Value,
) -> Result<(), DataStoreError> {
    validate_key(key)?;
    let schema = contract.state_schema.as_ref().ok_or_else(|| {
        data_store_error(
            DataStoreErrorCode::NoStateSchemaDeclared,
            "no_state_schema_declared",
            json!({ "capability_id": contract.id, "key": key }),
        )
    })?;
    let property_schema = schema
        .get("properties")
        .and_then(Value::as_object)
        .and_then(|properties| properties.get(key))
        .ok_or_else(|| {
            data_store_error(
                DataStoreErrorCode::SchemaValidationError,
                "schema_validation_error",
                json!({ "key": key, "reason": "state key is not declared in schema" }),
            )
        })?;
    let mut violations = Vec::new();
    crate::validate_value_against_schema(value, property_schema, "$", &mut violations);
    if violations.is_empty() {
        Ok(())
    } else {
        Err(data_store_error(
            DataStoreErrorCode::SchemaValidationError,
            "schema_validation_error",
            json!({ "key": key, "violations": violations }),
        ))
    }
}

fn sync_adapters(
    local: &mut dyn DataStore,
    remote: &mut dyn DataStore,
) -> Result<SyncReport, DataStoreError> {
    let keys = merged_keys(local.list_keys()?, remote.list_keys()?);
    let mut decisions = Vec::new();
    let mut snapshots = BTreeMap::new();

    for key in keys {
        let local_record = local.read(&key)?;
        let remote_record = remote.read(&key)?;
        snapshots.insert(key.clone(), local_record.clone());
        let Some((winner, rule)) = merge_records(local_record.as_ref(), remote_record.as_ref())
        else {
            continue;
        };
        apply_winner(local, remote, &key, &winner).map_err(|error| {
            rollback_local(local, &snapshots);
            data_store_error(
                DataStoreErrorCode::SyncFailure,
                "sync failed; local state restored",
                json!({ "key": key, "cause": error.message }),
            )
        })?;
        decisions.push(MergeDecision {
            key,
            winning_writer_id: winner.writer_id,
            winning_lamport_clock: winner.lamport_clock,
            resolution_rule: rule,
        });
    }

    Ok(SyncReport {
        governing_spec: DATA_STORE_SPEC.to_string(),
        decisions,
    })
}

fn merged_keys(local_keys: Vec<String>, remote_keys: Vec<String>) -> Vec<String> {
    local_keys
        .into_iter()
        .chain(remote_keys)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn merge_records(
    local: Option<&StateRecord>,
    remote: Option<&StateRecord>,
) -> Option<(StateRecord, ConflictResolutionRule)> {
    match (local, remote) {
        (Some(record), None) => Some((record.clone(), ConflictResolutionRule::OnlyLocal)),
        (None, Some(record)) => Some((record.clone(), ConflictResolutionRule::OnlyRemote)),
        (Some(local), Some(remote)) => Some(select_conflict_winner(local, remote)),
        (None, None) => None,
    }
}

fn select_conflict_winner(
    local: &StateRecord,
    remote: &StateRecord,
) -> (StateRecord, ConflictResolutionRule) {
    if local.lamport_clock > remote.lamport_clock {
        return (local.clone(), ConflictResolutionRule::HigherLamportClock);
    }
    if remote.lamport_clock > local.lamport_clock {
        return (remote.clone(), ConflictResolutionRule::HigherLamportClock);
    }
    if local.writer_id >= remote.writer_id {
        (
            local.clone(),
            ConflictResolutionRule::WriterIdentityTieBreak,
        )
    } else {
        (
            remote.clone(),
            ConflictResolutionRule::WriterIdentityTieBreak,
        )
    }
}

fn apply_winner(
    local: &mut dyn DataStore,
    remote: &mut dyn DataStore,
    key: &str,
    winner: &StateRecord,
) -> Result<(), DataStoreError> {
    if local.read(key)?.as_ref() != Some(winner) {
        local.write(winner.clone())?;
    }
    if remote.read(key)?.as_ref() != Some(winner) {
        remote.write(winner.clone())?;
    }
    Ok(())
}

fn rollback_local(local: &mut dyn DataStore, snapshots: &BTreeMap<String, Option<StateRecord>>) {
    for (key, snapshot) in snapshots {
        let result = match snapshot {
            Some(record) => local.write(record.clone()),
            None => local.delete(key),
        };
        let _ignored = result.is_ok();
    }
}

fn validate_key(key: &str) -> Result<(), DataStoreError> {
    let valid = !key.is_empty()
        && key
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'));
    if valid {
        Ok(())
    } else {
        Err(data_store_error(
            DataStoreErrorCode::InvalidKey,
            "state key must be non-empty and contain only ASCII letters, numbers, '_' or '-'",
            json!({ "key": key }),
        ))
    }
}

fn data_store_error(code: DataStoreErrorCode, message: &str, details: Value) -> DataStoreError {
    DataStoreError {
        code,
        message: message.to_string(),
        details,
    }
}

fn io_error(action: &str, error: &std::io::Error) -> DataStoreError {
    data_store_error(
        DataStoreErrorCode::IoFailure,
        "storage_io_failed",
        json!({ "action": action, "reason": error.to_string() }),
    )
}

fn serialization_error(action: &str, error: &serde_json::Error) -> DataStoreError {
    data_store_error(
        DataStoreErrorCode::SerializationFailure,
        action,
        json!({ "error": error.to_string() }),
    )
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::cell::Cell;
    use std::process::{Child, Command, Stdio};
    use std::thread;
    use std::time::Duration;
    use traverse_contracts::{
        BinaryFormat, CapabilityContract, Condition, DependencyReference, Entrypoint,
        EntrypointKind, EventReference, Execution, ExecutionConstraints, ExecutionTarget,
        FilesystemAccess, HostApiAccess, IdReference, Lifecycle, NetworkAccess, Owner, Provenance,
        ProvenanceSource, SchemaContainer, ServiceType, SideEffect, SideEffectKind,
        ValidationEvidence,
    };
    use uuid::Uuid;

    #[derive(Debug, Clone, Default)]
    struct MemoryDataStore {
        records: BTreeMap<String, StateRecord>,
        fail_writes: Cell<bool>,
    }

    #[derive(Debug, Clone, Default)]
    struct PhantomKeyStore;

    impl DataStore for MemoryDataStore {
        fn read(&self, key: &str) -> Result<Option<StateRecord>, DataStoreError> {
            Ok(self.records.get(key).cloned())
        }

        fn write(&mut self, record: StateRecord) -> Result<(), DataStoreError> {
            if self.fail_writes.get() {
                return Err(data_store_error(
                    DataStoreErrorCode::IoFailure,
                    "forced write failure",
                    json!({ "key": record.key }),
                ));
            }
            self.records.insert(record.key.clone(), record);
            Ok(())
        }

        fn delete(&mut self, key: &str) -> Result<(), DataStoreError> {
            self.records.remove(key);
            Ok(())
        }

        fn list_keys(&self) -> Result<Vec<String>, DataStoreError> {
            Ok(self.records.keys().cloned().collect())
        }
    }

    impl DataStore for PhantomKeyStore {
        fn read(&self, _key: &str) -> Result<Option<StateRecord>, DataStoreError> {
            Ok(None)
        }

        fn write(&mut self, _record: StateRecord) -> Result<(), DataStoreError> {
            Ok(())
        }

        fn delete(&mut self, _key: &str) -> Result<(), DataStoreError> {
            Ok(())
        }

        fn list_keys(&self) -> Result<Vec<String>, DataStoreError> {
            Ok(vec!["phantom".to_string()])
        }
    }

    #[test]
    fn runtime_data_store_validates_writes_and_reads_from_local_file_adapter() {
        let root = temp_root("valid");
        let adapter = LocalFileDataStore::new(&root).expect("local adapter should initialize");
        let mut store = RuntimeDataStore::new(adapter, "writer-a");
        let contract = stateful_contract(Some(json!({
            "type": "object",
            "properties": {
                "draft": {"type": "string"}
            }
        })));

        let record = store
            .write(&contract, "draft", json!("ready"))
            .expect("valid state write should succeed");

        assert_eq!(record.lamport_clock, 1);
        assert_eq!(
            store.read(&contract, "draft").expect("read should succeed"),
            Some(json!("ready"))
        );
        assert_eq!(
            store.list_keys().expect("list should succeed"),
            vec!["draft".to_string()]
        );
        store.delete("draft").expect("delete should succeed");
        assert_eq!(
            store.read(&contract, "draft").expect("read should succeed"),
            None
        );
    }

    #[test]
    fn runtime_data_store_rejects_missing_schema_bad_keys_and_schema_violations() {
        let adapter = MemoryDataStore::default();
        let mut store = RuntimeDataStore::new(adapter, "writer-a");
        let no_schema = stateful_contract(None);
        let schema = stateful_contract(Some(json!({
            "type": "object",
            "properties": {
                "count": {"type": "integer"}
            }
        })));

        let missing = store
            .write(&no_schema, "count", json!(1))
            .expect_err("missing state schema should fail");
        assert_eq!(missing.code, DataStoreErrorCode::NoStateSchemaDeclared);

        let invalid_key = store
            .write(&schema, "bad.key", json!(1))
            .expect_err("invalid key should fail");
        assert_eq!(invalid_key.code, DataStoreErrorCode::InvalidKey);

        let undeclared = store
            .write(&schema, "other", json!(1))
            .expect_err("undeclared state key should fail");
        assert_eq!(undeclared.code, DataStoreErrorCode::SchemaValidationError);

        let wrong_type = store
            .write(&schema, "count", json!("one"))
            .expect_err("wrong state type should fail");
        assert_eq!(wrong_type.code, DataStoreErrorCode::SchemaValidationError);

        let no_schema_read = store
            .read(&no_schema, "count")
            .expect("no-schema read should succeed");
        assert_eq!(no_schema_read, None);

        let bad_read_key = store
            .read(&schema, "bad.key")
            .expect_err("invalid read key should fail");
        assert_eq!(bad_read_key.code, DataStoreErrorCode::InvalidKey);
    }

    #[test]
    fn lamport_clock_overflow_is_rejected_before_adapter_write() {
        let adapter = MemoryDataStore::default();
        let clock = LamportClock::with_value("writer-a", u64::MAX);
        let mut store = RuntimeDataStore::with_clock(adapter, clock);
        let contract = stateful_contract(Some(json!({
            "type": "object",
            "properties": {
                "draft": {"type": "string"}
            }
        })));

        let error = store
            .write(&contract, "draft", json!("ready"))
            .expect_err("overflow should fail");

        assert_eq!(error.code, DataStoreErrorCode::LamportClockOverflow);
        assert!(store.into_inner().records.is_empty());
    }

    #[test]
    fn runtime_data_store_validates_reads_before_returning_stored_values() {
        let mut adapter = MemoryDataStore::default();
        adapter
            .write(record("count", "writer-a", 1, json!("not an integer")))
            .expect("seed should succeed");
        let store = RuntimeDataStore::new(adapter, "writer-a");
        let contract = stateful_contract(Some(json!({
            "type": "object",
            "properties": {
                "count": {"type": "integer"}
            }
        })));

        let error = store
            .read(&contract, "count")
            .expect_err("invalid stored value should fail");

        assert_eq!(error.code, DataStoreErrorCode::SchemaValidationError);
    }

    #[test]
    fn reconnect_sync_merges_only_local_only_remote_clock_winner_and_writer_tie_breaks() {
        let mut local = MemoryDataStore::default();
        let mut remote = MemoryDataStore::default();
        local
            .write(record("local_only", "local-a", 1, json!("local")))
            .expect("local write should succeed");
        remote
            .write(record("remote_only", "remote-a", 1, json!("remote")))
            .expect("remote write should succeed");
        local
            .write(record("clock", "local-a", 2, json!("old")))
            .expect("local write should succeed");
        remote
            .write(record("clock", "remote-a", 3, json!("new")))
            .expect("remote write should succeed");
        local
            .write(record("tie", "writer-z", 4, json!("winner")))
            .expect("local write should succeed");
        remote
            .write(record("tie", "writer-a", 4, json!("loser")))
            .expect("remote write should succeed");

        let report = sync_adapters(&mut local, &mut remote).expect("sync should succeed");

        assert_eq!(report.governing_spec, "032-universal-data-access");
        assert_eq!(report.decisions.len(), 4);
        assert_eq!(
            local.read("remote_only").expect("read should succeed"),
            remote.read("remote_only").expect("read should succeed")
        );
        assert_eq!(
            local.read("clock").expect("read should succeed"),
            Some(record("clock", "remote-a", 3, json!("new")))
        );
        assert_eq!(
            remote.read("tie").expect("read should succeed"),
            Some(record("tie", "writer-z", 4, json!("winner")))
        );
        assert!(
            report
                .decisions
                .iter()
                .any(|decision| decision.resolution_rule
                    == ConflictResolutionRule::WriterIdentityTieBreak)
        );
    }

    #[test]
    fn sync_failure_restores_local_snapshot() {
        let mut local = MemoryDataStore::default();
        let mut remote = MemoryDataStore::default();
        local
            .write(record("shared", "local-a", 2, json!("local")))
            .expect("local write should succeed");
        remote
            .write(record("shared", "remote-a", 1, json!("remote")))
            .expect("remote write should succeed");
        remote.fail_writes.set(true);

        let error = sync_adapters(&mut local, &mut remote).expect_err("sync should fail");

        assert_eq!(error.code, DataStoreErrorCode::SyncFailure);
        assert_eq!(
            local.read("shared").expect("read should succeed"),
            Some(record("shared", "local-a", 2, json!("local")))
        );
    }

    #[test]
    fn local_file_adapter_reports_bad_keys_and_bad_json() {
        let root = temp_root("bad-json");
        let adapter = LocalFileDataStore::new(&root).expect("local adapter should initialize");
        let invalid = adapter
            .read("bad.key")
            .expect_err("invalid key should fail");
        assert_eq!(invalid.code, DataStoreErrorCode::InvalidKey);

        fs::write(root.join("broken.json"), "{").expect("bad json fixture should write");
        let invalid_json = adapter
            .read("broken")
            .expect_err("invalid json should fail");
        assert_eq!(invalid_json.code, DataStoreErrorCode::IntegrityCheckFailed);
        assert_eq!(invalid_json.message, "integrity_check_failed");
    }

    #[test]
    fn helper_paths_cover_remaining_datastore_branches() {
        let mut local = RuntimeDataStore::new(MemoryDataStore::default(), "local-a");
        let mut remote = MemoryDataStore::default();
        remote
            .write(record("remote_only", "remote-a", 1, json!("remote")))
            .expect("remote seed should succeed");

        let report = local
            .sync_on_reconnect(&mut remote)
            .expect("public reconnect sync should succeed");
        assert_eq!(report.decisions.len(), 1);

        assert!(merge_records(None, None).is_none());
        let (_winner, rule) = select_conflict_winner(
            &record("tie", "writer-a", 1, json!("local")),
            &record("tie", "writer-z", 1, json!("remote")),
        );
        assert_eq!(rule, ConflictResolutionRule::WriterIdentityTieBreak);

        let mut failing_local = MemoryDataStore::default();
        failing_local.fail_writes.set(true);
        let mut seeded_remote = MemoryDataStore::default();
        seeded_remote
            .write(record("missing_local", "remote-a", 1, json!("remote")))
            .expect("remote seed should succeed");
        let error =
            sync_adapters(&mut failing_local, &mut seeded_remote).expect_err("sync should fail");
        assert_eq!(error.code, DataStoreErrorCode::SyncFailure);
        assert_eq!(
            failing_local
                .delete("missing_local")
                .expect("delete should succeed"),
            ()
        );

        let mut phantom_local = PhantomKeyStore;
        let mut phantom_remote = PhantomKeyStore;
        assert!(
            sync_adapters(&mut phantom_local, &mut phantom_remote)
                .expect("phantom sync should succeed")
                .decisions
                .is_empty()
        );
        phantom_local
            .write(record("phantom", "writer-a", 1, json!("value")))
            .expect("phantom write should succeed");
        phantom_local
            .delete("phantom")
            .expect("phantom delete should succeed");

        let root = temp_root("listing");
        fs::create_dir_all(&root).expect("root should be created");
        fs::write(root.join("skip.txt"), "not state").expect("non-json fixture should write");
        let adapter = LocalFileDataStore::new(&root).expect("local adapter should initialize");
        assert!(adapter.list_keys().expect("list should succeed").is_empty());
        drop(adapter);
        let mut delete_missing =
            LocalFileDataStore::new(&root).expect("local adapter should initialize");
        delete_missing
            .delete("missing")
            .expect("missing delete should succeed");
        fs::create_dir(root.join("cant_delete.json")).expect("directory fixture should write");
        let delete_failure = delete_missing
            .delete("cant_delete")
            .expect_err("directory delete should fail");
        assert_eq!(delete_failure.code, DataStoreErrorCode::IoFailure);

        let file_root = temp_root("file-root");
        fs::write(&file_root, "not a directory").expect("file root fixture should write");
        let io_failure = LocalFileDataStore::new(&file_root).expect_err("file root should fail");
        assert_eq!(io_failure.code, DataStoreErrorCode::IoFailure);
    }

    #[test]
    fn local_file_adapter_writes_integrity_envelope_and_reopens() {
        let root = temp_root("integrity-envelope");
        let record = record("draft", "writer-a", 1, json!("ready"));
        let mut adapter =
            LocalFileDataStore::with_classification(&root, LocalDataClassification::Public)
                .expect("local adapter should initialize");
        adapter.write(record.clone()).expect("write should succeed");

        let envelope: LocalDataStoreEnvelope = serde_json::from_slice(
            &fs::read(root.join("draft.json")).expect("envelope should be present"),
        )
        .expect("envelope should deserialize");
        assert_eq!(envelope.format, LOCAL_DATA_STORE_FORMAT);
        assert_eq!(envelope.classification, LocalDataClassification::Public);
        assert_eq!(
            envelope.digest,
            digest_for_record(&record).expect("digest should compute")
        );

        drop(adapter);
        let reopened = LocalFileDataStore::new(&root).expect("reopen should acquire lock");
        assert_eq!(
            reopened.read("draft").expect("read should succeed"),
            Some(record)
        );
    }

    #[test]
    fn local_file_adapter_rejects_tampered_and_legacy_records() {
        let root = temp_root("tampered");
        let mut adapter = LocalFileDataStore::new(&root).expect("local adapter should initialize");
        adapter
            .write(record("draft", "writer-a", 1, json!("ready")))
            .expect("write should succeed");

        let mut envelope: Value = serde_json::from_slice(
            &fs::read(root.join("draft.json")).expect("envelope should be present"),
        )
        .expect("fixture should deserialize");
        envelope["record"]["value"] = json!("tampered");
        fs::write(
            root.join("draft.json"),
            serde_json::to_vec(&envelope).expect("fixture should serialize"),
        )
        .expect("tampered fixture should write");
        let tampered = adapter.read("draft").expect_err("tampering must fail");
        assert_eq!(tampered.code, DataStoreErrorCode::IntegrityCheckFailed);
        assert_eq!(tampered.details["reason"], "digest_mismatch");

        fs::write(
            root.join("legacy.json"),
            serde_json::to_vec(&record("legacy", "writer-a", 1, json!("old")))
                .expect("legacy fixture should serialize"),
        )
        .expect("legacy fixture should write");
        let legacy = adapter.read("legacy").expect_err("legacy must fail closed");
        assert_eq!(legacy.code, DataStoreErrorCode::IntegrityCheckFailed);
        assert_eq!(legacy.details["reason"], "legacy_unverified");
    }

    #[test]
    fn local_file_adapter_ignores_temporary_records_and_rejects_second_owner() {
        let root = temp_root("temporary-and-lock");
        let mut adapter = LocalFileDataStore::new(&root).expect("local adapter should initialize");
        adapter
            .write(record("draft", "writer-a", 1, json!("committed")))
            .expect("write should succeed");
        fs::write(root.join(".draft.temporary.tmp"), "incomplete")
            .expect("temporary fixture should write");

        assert_eq!(
            adapter.list_keys().expect("listing should succeed"),
            vec!["draft".to_string()]
        );
        assert_eq!(
            adapter
                .read("draft")
                .expect("committed read should succeed"),
            Some(record("draft", "writer-a", 1, json!("committed")))
        );
        let second_owner = LocalFileDataStore::new(&root).expect_err("second owner must fail");
        assert_eq!(second_owner.code, DataStoreErrorCode::StoreLocked);
        assert_eq!(second_owner.message, "store_locked");
        assert_eq!(
            second_owner.details,
            json!({ "reason": "exclusive_owner_active" })
        );
    }

    #[test]
    fn local_file_adapter_lock_child() {
        let Ok(root) = std::env::var("TRAVERSE_DATA_STORE_LOCK_CHILD_ROOT") else {
            return;
        };
        let root = PathBuf::from(root);
        let _adapter = LocalFileDataStore::new(&root).expect("child should acquire lock");
        fs::write(lock_child_ready_path(&root), "ready").expect("child should signal readiness");
        for _ in 0..500 {
            if lock_child_release_path(&root).exists() {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("parent did not release the lock child");
    }

    #[test]
    fn local_file_adapter_rejects_cross_process_owner_and_recovers_after_exit() {
        let root = temp_root("cross-process-lock");
        let mut initial_owner = LocalFileDataStore::new(&root).expect("initial owner should open");
        let committed = record("draft", "writer-a", 1, json!("committed"));
        initial_owner
            .write(committed.clone())
            .expect("initial write should succeed");
        drop(initial_owner);

        let mut child = start_lock_child(&root);
        wait_for_lock_child(&root);
        let blocked = LocalFileDataStore::new(&root).expect_err("second process must be blocked");
        assert_eq!(blocked.code, DataStoreErrorCode::StoreLocked);
        assert_eq!(
            blocked.details,
            json!({ "reason": "exclusive_owner_active" })
        );

        fs::write(lock_child_release_path(&root), "release").expect("parent should release child");
        assert!(child.wait().expect("child should exit").success());
        let reopened = LocalFileDataStore::new(&root).expect("released lock should reopen");
        assert_eq!(
            reopened
                .read("draft")
                .expect("committed record should remain readable"),
            Some(committed)
        );
    }

    #[test]
    fn local_file_adapter_recovers_after_lock_owner_crash() {
        let root = temp_root("owner-crash-lock");
        let mut initial_owner = LocalFileDataStore::new(&root).expect("initial owner should open");
        initial_owner
            .write(record("draft", "writer-a", 1, json!("committed")))
            .expect("initial write should succeed");
        drop(initial_owner);

        let mut child = start_lock_child(&root);
        wait_for_lock_child(&root);
        child.kill().expect("parent should terminate child");
        child.wait().expect("terminated child should exit");

        let reopened = LocalFileDataStore::new(&root).expect("crashed owner lock should release");
        assert_eq!(
            reopened
                .read("draft")
                .expect("committed record should remain readable"),
            Some(record("draft", "writer-a", 1, json!("committed")))
        );
    }

    #[test]
    fn local_file_adapter_reports_unknown_version_and_helper_failures_stably() {
        let root = temp_root("helper-failures");
        let mut adapter = LocalFileDataStore::new(&root).expect("local adapter should initialize");
        adapter
            .write(record("draft", "writer-a", 1, json!("ready")))
            .expect("write should succeed");

        let mut envelope: Value = serde_json::from_slice(
            &fs::read(root.join("draft.json")).expect("envelope should be present"),
        )
        .expect("fixture should deserialize");
        envelope["format"] = json!("local-datastore/unsupported");
        fs::write(
            root.join("draft.json"),
            serde_json::to_vec(&envelope).expect("fixture should serialize"),
        )
        .expect("unknown-version fixture should write");
        let unknown_version = adapter
            .read("draft")
            .expect_err("unknown format version must fail");
        assert_eq!(
            unknown_version.code,
            DataStoreErrorCode::IntegrityCheckFailed
        );
        assert_eq!(unknown_version.details["reason"], "unknown_format_version");

        let lock_io = lock_error(TryLockError::Error(std::io::Error::other(
            "lock device failure",
        )));
        assert_eq!(lock_io.code, DataStoreErrorCode::IoFailure);
        assert_eq!(lock_io.message, "storage_io_failed");
        assert_eq!(
            lock_io.details,
            json!({ "operation": "acquire_lock", "reason": "lock_acquisition_failed" })
        );

        let unsupported_lock = lock_error(TryLockError::Error(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "locking unavailable",
        )));
        assert_eq!(unsupported_lock.code, DataStoreErrorCode::IoFailure);
        assert_eq!(
            unsupported_lock.details,
            json!({ "operation": "acquire_lock", "reason": "locking_unsupported" })
        );

        let durability = durability_error(
            &root,
            "temporary_file",
            &std::io::Error::other("sync failure"),
        );
        assert_eq!(durability.code, DataStoreErrorCode::DurabilityCommitFailed);
        assert_eq!(durability.message, "durability_commit_failed");

        let temporary = root.join(".orphan.tmp");
        fs::write(&temporary, "incomplete").expect("temporary fixture should write");
        discard_temporary_record(&temporary);
        assert!(!temporary.exists());
        discard_temporary_record(&temporary);

        let parse_error = serde_json::from_str::<Value>("{")
            .expect_err("invalid fixture must produce a serialization error");
        let serialization = serialization_error("deserialize fixture", &parse_error);
        assert_eq!(serialization.code, DataStoreErrorCode::SerializationFailure);
    }

    fn record(key: &str, writer_id: &str, lamport_clock: u64, value: Value) -> StateRecord {
        StateRecord {
            key: key.to_string(),
            value,
            lamport_clock,
            writer_id: writer_id.to_string(),
        }
    }

    fn temp_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("traverse-data-store-{name}-{}", Uuid::new_v4()))
    }

    fn lock_child_ready_path(root: &PathBuf) -> PathBuf {
        root.join(".lock-child-ready")
    }

    fn lock_child_release_path(root: &PathBuf) -> PathBuf {
        root.join(".lock-child-release")
    }

    fn start_lock_child(root: &PathBuf) -> Child {
        Command::new(std::env::current_exe().expect("test binary path should resolve"))
            .args([
                "--exact",
                "data_store::tests::local_file_adapter_lock_child",
                "--nocapture",
            ])
            .env("TRAVERSE_DATA_STORE_LOCK_CHILD_ROOT", root)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("lock child should start")
    }

    fn wait_for_lock_child(root: &PathBuf) {
        for _ in 0..500 {
            if lock_child_ready_path(root).exists() {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("lock child did not become ready");
    }

    fn stateful_contract(state_schema: Option<Value>) -> CapabilityContract {
        CapabilityContract {
            kind: "capability_contract".to_string(),
            schema_version: "1.0.0".to_string(),
            id: "stateful.example".to_string(),
            namespace: "stateful".to_string(),
            name: "example".to_string(),
            version: "1.0.0".to_string(),
            lifecycle: Lifecycle::Active,
            owner: Owner {
                team: "runtime".to_string(),
                contact: "runtime@example.com".to_string(),
            },
            summary: "Stateful test capability".to_string(),
            description: "Stateful test capability".to_string(),
            inputs: SchemaContainer {
                schema: json!({"type": "object"}),
            },
            outputs: SchemaContainer {
                schema: json!({"type": "object"}),
            },
            preconditions: Vec::<Condition>::new(),
            postconditions: Vec::<Condition>::new(),
            side_effects: vec![SideEffect {
                kind: SideEffectKind::StateChange,
                description: "writes capability state".to_string(),
            }],
            emits: Vec::<EventReference>::new(),
            consumes: Vec::<EventReference>::new(),
            permissions: Vec::<IdReference>::new(),
            execution: Execution {
                binary_format: BinaryFormat::Wasm,
                constraints: ExecutionConstraints {
                    network_access: NetworkAccess::Forbidden,
                    filesystem_access: FilesystemAccess::SandboxOnly,
                    host_api_access: HostApiAccess::None,
                },
                entrypoint: Entrypoint {
                    kind: EntrypointKind::WasiCommand,
                    command: "run".to_string(),
                },
                preferred_targets: vec![ExecutionTarget::Local],
            },
            policies: Vec::<IdReference>::new(),
            dependencies: Vec::<DependencyReference>::new(),
            provenance: Provenance {
                source: ProvenanceSource::Greenfield,
                author: "Codex".to_string(),
                created_at: "2026-04-19T00:00:00Z".to_string(),
                spec_ref: Some("032-universal-data-access".to_string()),
                adr_refs: Vec::new(),
                exception_refs: Vec::new(),
            },
            evidence: Vec::<ValidationEvidence>::new(),
            service_type: ServiceType::Stateful,
            permitted_targets: vec![ExecutionTarget::Local],
            event_trigger: None,
            connector_requirements: Vec::new(),
            state_schema,
        }
    }
}

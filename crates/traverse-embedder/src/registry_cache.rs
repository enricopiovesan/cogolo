//! Host-owned verified registry dependency cache (Spec `080-embedded-registry-cache`).
//!
//! Separates network-capable [`prepare`] from offline [`HostRegistryCache`]
//! resolution used at embedder `init`. The host chooses the cache root and
//! supplies the artifact fetcher; Traverse never synthesizes a production
//! path or performs network I/O outside `prepare`.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use traverse_registry::{
    PublicRegistryCapabilityRecord, RegistryReference, SyncedPublicRegistryState,
};

/// Stable, secret-free registry-cache failure codes (Spec 080 FR-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryCacheErrorCode {
    /// No synced index snapshot was supplied to prepare.
    RegistrySyncMissing,
    /// No non-yanked version satisfies the requested range.
    RegistryVersionNotFound,
    /// Matching versions exist but every candidate is yanked/deprecated.
    RegistryDependencyYanked,
    /// Host fetch or cache persistence failed during prepare.
    RegistryPrepareFailed,
    /// Declared digest does not match fetched or stored bytes.
    RegistryArtifactDigestMismatch,
    /// Offline resolve found no verified cache entry for the reference.
    RegistryCacheEntryMissing,
}

impl RegistryCacheErrorCode {
    /// Stable `snake_case` wire representation.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RegistrySyncMissing => "registry_sync_missing",
            Self::RegistryVersionNotFound => "registry_version_not_found",
            Self::RegistryDependencyYanked => "registry_dependency_yanked",
            Self::RegistryPrepareFailed => "registry_prepare_failed",
            Self::RegistryArtifactDigestMismatch => "registry_artifact_digest_mismatch",
            Self::RegistryCacheEntryMissing => "registry_cache_entry_missing",
        }
    }
}

/// Secret-free registry-cache failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryCacheError {
    /// Stable machine-readable code.
    pub code: RegistryCacheErrorCode,
    /// Human-readable explanation without paths, credentials, or bytes.
    pub message: String,
}

impl RegistryCacheError {
    fn new(code: RegistryCacheErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// Host-supplied network fetch used only by [`prepare`].
pub trait RegistryArtifactFetcher {
    /// Fetch raw bytes for a published registry URL.
    ///
    /// # Errors
    ///
    /// Returns a secret-free message when the host cannot retrieve the asset.
    fn fetch(&self, url: &str) -> Result<Vec<u8>, String>;
}

/// Host-chosen content-addressed registry cache root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostRegistryCache {
    root: PathBuf,
}

impl HostRegistryCache {
    /// Bind Traverse to a host-selected cache directory.
    ///
    /// The host owns creation, permissions, backup, and eviction policy.
    /// Traverse never invents a default production cache path.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Host-selected cache root.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Remove one verified entry by artifact digest (`sha256:…`).
    ///
    /// # Errors
    ///
    /// Returns [`RegistryCacheErrorCode::RegistryPrepareFailed`] when the
    /// entry cannot be removed. Missing entries succeed without error.
    pub fn evict(&self, artifact_digest: &str) -> Result<(), RegistryCacheError> {
        let digest = normalize_digest(artifact_digest).ok_or_else(|| {
            RegistryCacheError::new(
                RegistryCacheErrorCode::RegistryPrepareFailed,
                "artifact digest must be sha256: followed by 64 hex characters",
            )
        })?;
        let artifact = self.artifact_path(&digest);
        let meta = self.meta_path(&digest);
        if artifact.exists() {
            fs::remove_file(&artifact).map_err(|error| {
                RegistryCacheError::new(
                    RegistryCacheErrorCode::RegistryPrepareFailed,
                    format!("failed to evict verified registry artifact: {error}"),
                )
            })?;
        }
        if meta.exists() {
            fs::remove_file(&meta).map_err(|error| {
                RegistryCacheError::new(
                    RegistryCacheErrorCode::RegistryPrepareFailed,
                    format!("failed to evict registry cache metadata: {error}"),
                )
            })?;
        }
        Ok(())
    }

    /// Remove every verified entry under this cache root.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryCacheErrorCode::RegistryPrepareFailed`] when the
    /// cache directory cannot be cleared.
    pub fn evict_all(&self) -> Result<(), RegistryCacheError> {
        for sub in ["sha256", "meta", "refs"] {
            let path = self.root.join(sub);
            if path.exists() {
                fs::remove_dir_all(&path).map_err(|error| {
                    RegistryCacheError::new(
                        RegistryCacheErrorCode::RegistryPrepareFailed,
                        format!("failed to clear host registry cache: {error}"),
                    )
                })?;
            }
        }
        Ok(())
    }

    fn artifact_path(&self, digest_hex: &str) -> PathBuf {
        self.root.join("sha256").join(digest_hex)
    }

    fn meta_path(&self, digest_hex: &str) -> PathBuf {
        self.root.join("meta").join(format!("{digest_hex}.json"))
    }

    fn ref_path(&self, reference: &RegistryReference) -> PathBuf {
        let key = sha256_hex(
            format!(
                "{}:{}:{}",
                reference.namespace, reference.id, reference.version_range
            )
            .as_bytes(),
        );
        self.root.join("refs").join(format!("{key}.json"))
    }
}

/// FR-008 resolution evidence retained after a successful prepare.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryPrepareEvidence {
    /// Registry namespace.
    pub namespace: String,
    /// Capability id.
    pub id: String,
    /// Selected concrete version.
    pub selected_version: String,
    /// Requested semver range.
    pub version_range: String,
    /// Source release tag from the synced index snapshot.
    pub source_release: String,
    /// Digest of the synced index snapshot supplied to prepare.
    pub index_digest: String,
    /// Published artifact digest.
    pub artifact_digest: String,
    /// Unix-epoch seconds when the entry was verified.
    pub verified_at: u64,
    /// Outcome label (`prepared`).
    pub outcome: &'static str,
}

impl RegistryPrepareEvidence {
    /// Secret-free JSON projection of prepare evidence.
    #[must_use]
    pub fn as_value(&self) -> Value {
        json!({
            "namespace": self.namespace,
            "id": self.id,
            "selected_version": self.selected_version,
            "version_range": self.version_range,
            "source_release": self.source_release,
            "index_digest": self.index_digest,
            "artifact_digest": self.artifact_digest,
            "verified_at": self.verified_at,
            "outcome": self.outcome,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct CacheEntryMeta {
    namespace: String,
    id: String,
    selected_version: String,
    version_range: String,
    source_release: String,
    index_digest: String,
    artifact_digest: String,
    contract_digest: String,
    verified_at: u64,
}

/// Offline lookup of a previously prepared `registry_ref`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedRegistryDependency {
    /// Absolute path to digest-verified WASM bytes.
    pub wasm_binary_path: PathBuf,
    /// Absolute path to digest-verified contract JSON.
    pub contract_path: PathBuf,
    /// Published artifact digest.
    pub wasm_digest: String,
    /// Prepare evidence for the selected record.
    pub evidence: RegistryPrepareEvidence,
}

/// Prepare one `registry_ref` into a host-owned verified cache.
///
/// Only this function may call `fetcher`. Partial writes never become
/// executable entries.
///
/// # Errors
///
/// Returns a Spec 080 FR-007 code when the snapshot is empty, version
/// selection fails, fetch fails, or digest verification fails.
pub fn prepare(
    cache: &HostRegistryCache,
    snapshot: &SyncedPublicRegistryState,
    reference: &RegistryReference,
    fetcher: &dyn RegistryArtifactFetcher,
) -> Result<RegistryPrepareEvidence, RegistryCacheError> {
    if snapshot.capabilities.is_empty() {
        return Err(RegistryCacheError::new(
            RegistryCacheErrorCode::RegistrySyncMissing,
            "synced registry index snapshot contains no capabilities",
        ));
    }
    let record = select_highest_active(snapshot, reference)?;
    let index_digest = index_snapshot_digest(snapshot);
    let artifact_bytes = fetcher.fetch(&record.artifact_url).map_err(|message| {
        RegistryCacheError::new(
            RegistryCacheErrorCode::RegistryPrepareFailed,
            format!("host registry artifact fetch failed: {message}"),
        )
    })?;
    verify_digest(&record.digest, &artifact_bytes)?;
    let contract_bytes = fetcher.fetch(&record.contract_url).map_err(|message| {
        RegistryCacheError::new(
            RegistryCacheErrorCode::RegistryPrepareFailed,
            format!("host registry contract fetch failed: {message}"),
        )
    })?;
    verify_digest(&record.contract_digest, &contract_bytes)?;

    let artifact_hex = normalize_digest(&record.digest).ok_or_else(|| {
        RegistryCacheError::new(
            RegistryCacheErrorCode::RegistryPrepareFailed,
            "artifact digest must be sha256: followed by 64 hex characters",
        )
    })?;
    let contract_hex = normalize_digest(&record.contract_digest).ok_or_else(|| {
        RegistryCacheError::new(
            RegistryCacheErrorCode::RegistryPrepareFailed,
            "contract digest must be sha256: followed by 64 hex characters",
        )
    })?;
    write_verified_bytes(cache, &artifact_hex, &artifact_bytes)?;
    write_verified_bytes(cache, &contract_hex, &contract_bytes)?;

    let verified_at = unix_seconds();
    let meta = CacheEntryMeta {
        namespace: record.namespace.clone(),
        id: record.id.clone(),
        selected_version: record.version.clone(),
        version_range: reference.version_range.clone(),
        source_release: snapshot.release_tag.clone(),
        index_digest: index_digest.clone(),
        artifact_digest: record.digest.clone(),
        contract_digest: record.contract_digest.clone(),
        verified_at,
    };
    write_json_atomic(&cache.meta_path(&artifact_hex), &meta)?;
    write_json_atomic(
        &cache.ref_path(reference),
        &json!({
            "artifact_digest": record.digest,
            "contract_digest": record.contract_digest,
        }),
    )?;

    Ok(RegistryPrepareEvidence {
        namespace: record.namespace,
        id: record.id,
        selected_version: record.version,
        version_range: reference.version_range.clone(),
        source_release: snapshot.release_tag.clone(),
        index_digest,
        artifact_digest: record.digest,
        verified_at,
        outcome: "prepared",
    })
}

/// Resolve a `registry_ref` from verified local cache entries only.
///
/// # Errors
///
/// Returns [`RegistryCacheErrorCode::RegistryCacheEntryMissing`] when no
/// verified prepare result exists, or digest mismatch when stored bytes were
/// tampered with.
pub fn resolve_offline(
    cache: &HostRegistryCache,
    reference: &RegistryReference,
) -> Result<VerifiedRegistryDependency, RegistryCacheError> {
    let ref_path = cache.ref_path(reference);
    let ref_bytes = fs::read(&ref_path).map_err(|_| {
        RegistryCacheError::new(
            RegistryCacheErrorCode::RegistryCacheEntryMissing,
            "verified registry cache entry is missing for registry_ref",
        )
    })?;
    let pointer: Value = serde_json::from_slice(&ref_bytes).map_err(|_| {
        RegistryCacheError::new(
            RegistryCacheErrorCode::RegistryCacheEntryMissing,
            "verified registry cache entry is missing for registry_ref",
        )
    })?;
    let artifact_digest = pointer
        .get("artifact_digest")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            RegistryCacheError::new(
                RegistryCacheErrorCode::RegistryCacheEntryMissing,
                "verified registry cache entry is missing for registry_ref",
            )
        })?;
    let contract_digest = pointer
        .get("contract_digest")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            RegistryCacheError::new(
                RegistryCacheErrorCode::RegistryCacheEntryMissing,
                "verified registry cache entry is missing for registry_ref",
            )
        })?;
    let artifact_hex = normalize_digest(artifact_digest).ok_or_else(|| {
        RegistryCacheError::new(
            RegistryCacheErrorCode::RegistryCacheEntryMissing,
            "verified registry cache entry is missing for registry_ref",
        )
    })?;
    let contract_hex = normalize_digest(contract_digest).ok_or_else(|| {
        RegistryCacheError::new(
            RegistryCacheErrorCode::RegistryCacheEntryMissing,
            "verified registry cache entry is missing for registry_ref",
        )
    })?;
    let wasm_binary_path = read_verified(cache, &artifact_hex, artifact_digest)?;
    let contract_path = read_verified(cache, &contract_hex, contract_digest)?;
    let meta_bytes = fs::read(cache.meta_path(&artifact_hex)).map_err(|_| {
        RegistryCacheError::new(
            RegistryCacheErrorCode::RegistryCacheEntryMissing,
            "verified registry cache entry is missing for registry_ref",
        )
    })?;
    let meta: CacheEntryMeta = serde_json::from_slice(&meta_bytes).map_err(|_| {
        RegistryCacheError::new(
            RegistryCacheErrorCode::RegistryCacheEntryMissing,
            "verified registry cache entry is missing for registry_ref",
        )
    })?;
    Ok(VerifiedRegistryDependency {
        wasm_binary_path,
        contract_path,
        wasm_digest: artifact_digest.to_string(),
        evidence: RegistryPrepareEvidence {
            namespace: meta.namespace,
            id: meta.id,
            selected_version: meta.selected_version,
            version_range: meta.version_range,
            source_release: meta.source_release,
            index_digest: meta.index_digest,
            artifact_digest: meta.artifact_digest,
            verified_at: meta.verified_at,
            outcome: "resolved",
        },
    })
}

fn select_highest_active(
    snapshot: &SyncedPublicRegistryState,
    reference: &RegistryReference,
) -> Result<PublicRegistryCapabilityRecord, RegistryCacheError> {
    let requirement = semver::VersionReq::parse(&reference.version_range).map_err(|error| {
        RegistryCacheError::new(
            RegistryCacheErrorCode::RegistryVersionNotFound,
            format!(
                "invalid registry_ref version_range {}: {error}",
                reference.version_range
            ),
        )
    })?;
    let matching = snapshot
        .capabilities
        .iter()
        .filter_map(|record| {
            if record.namespace != reference.namespace || record.id != reference.id {
                return None;
            }
            semver::Version::parse(&record.version)
                .ok()
                .filter(|version| requirement.matches(version))
                .map(|version| (version, record.clone()))
        })
        .collect::<Vec<_>>();
    let mut active = matching
        .iter()
        .filter(|(_, record)| !record.deprecated)
        .cloned()
        .collect::<Vec<_>>();
    active.sort_by(|left, right| right.0.cmp(&left.0));
    if let Some((_, record)) = active.into_iter().next() {
        return Ok(record);
    }
    if matching.is_empty() {
        Err(RegistryCacheError::new(
            RegistryCacheErrorCode::RegistryVersionNotFound,
            format!(
                "no synced public registry version for {}:{} satisfies {}",
                reference.namespace, reference.id, reference.version_range
            ),
        ))
    } else {
        Err(RegistryCacheError::new(
            RegistryCacheErrorCode::RegistryDependencyYanked,
            format!(
                "only yanked public registry versions for {}:{} satisfy {}",
                reference.namespace, reference.id, reference.version_range
            ),
        ))
    }
}

fn index_snapshot_digest(snapshot: &SyncedPublicRegistryState) -> String {
    let encoded = serde_json::to_vec(snapshot).unwrap_or_default();
    format!("sha256:{}", sha256_hex(&encoded))
}

fn verify_digest(declared: &str, bytes: &[u8]) -> Result<(), RegistryCacheError> {
    let expected = normalize_digest(declared).ok_or_else(|| {
        RegistryCacheError::new(
            RegistryCacheErrorCode::RegistryArtifactDigestMismatch,
            "registry digest must be sha256: followed by 64 hex characters",
        )
    })?;
    if sha256_hex(bytes) == expected {
        Ok(())
    } else {
        Err(RegistryCacheError::new(
            RegistryCacheErrorCode::RegistryArtifactDigestMismatch,
            "registry artifact bytes do not match the published digest",
        ))
    }
}

fn write_verified_bytes(
    cache: &HostRegistryCache,
    digest_hex: &str,
    bytes: &[u8],
) -> Result<PathBuf, RegistryCacheError> {
    let path = cache.artifact_path(digest_hex);
    if path.exists() {
        let cached = fs::read(&path).map_err(|error| {
            RegistryCacheError::new(
                RegistryCacheErrorCode::RegistryPrepareFailed,
                format!("failed to read existing verified cache entry: {error}"),
            )
        })?;
        if sha256_hex(&cached) != digest_hex {
            return Err(RegistryCacheError::new(
                RegistryCacheErrorCode::RegistryArtifactDigestMismatch,
                "existing registry cache entry digest mismatch",
            ));
        }
        return Ok(path);
    }
    let parent = cache.root.join("sha256");
    fs::create_dir_all(&parent).map_err(|error| {
        RegistryCacheError::new(
            RegistryCacheErrorCode::RegistryPrepareFailed,
            format!("failed to create host registry cache directory: {error}"),
        )
    })?;
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, bytes).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        RegistryCacheError::new(
            RegistryCacheErrorCode::RegistryPrepareFailed,
            format!("failed to write registry cache entry: {error}"),
        )
    })?;
    fs::rename(&temporary, &path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        RegistryCacheError::new(
            RegistryCacheErrorCode::RegistryPrepareFailed,
            format!("failed to commit registry cache entry: {error}"),
        )
    })?;
    Ok(path)
}

fn read_verified(
    cache: &HostRegistryCache,
    digest_hex: &str,
    declared: &str,
) -> Result<PathBuf, RegistryCacheError> {
    let path = cache.artifact_path(digest_hex);
    let bytes = fs::read(&path).map_err(|_| {
        RegistryCacheError::new(
            RegistryCacheErrorCode::RegistryCacheEntryMissing,
            "verified registry cache entry is missing for registry_ref",
        )
    })?;
    verify_digest(declared, &bytes)?;
    Ok(path)
}

fn write_json_atomic(path: &Path, value: &impl serde::Serialize) -> Result<(), RegistryCacheError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            RegistryCacheError::new(
                RegistryCacheErrorCode::RegistryPrepareFailed,
                format!("failed to create host registry cache directory: {error}"),
            )
        })?;
    }
    let bytes = serde_json::to_vec(value).map_err(|error| {
        RegistryCacheError::new(
            RegistryCacheErrorCode::RegistryPrepareFailed,
            format!("failed to encode registry cache metadata: {error}"),
        )
    })?;
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, bytes).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        RegistryCacheError::new(
            RegistryCacheErrorCode::RegistryPrepareFailed,
            format!("failed to write registry cache metadata: {error}"),
        )
    })?;
    fs::rename(&temporary, path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        RegistryCacheError::new(
            RegistryCacheErrorCode::RegistryPrepareFailed,
            format!("failed to commit registry cache metadata: {error}"),
        )
    })?;
    Ok(())
}

fn normalize_digest(digest: &str) -> Option<String> {
    let digest = digest.strip_prefix("sha256:")?;
    if digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Some(digest.to_ascii_lowercase())
    } else {
        None
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut value = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(value, "{byte:02x}");
    }
    value
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct MapFetcher {
        assets: HashMap<String, Vec<u8>>,
    }

    impl RegistryArtifactFetcher for MapFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            self.assets
                .get(url)
                .cloned()
                .ok_or_else(|| "missing asset".to_string())
        }
    }

    fn digest_for(bytes: &[u8]) -> String {
        format!("sha256:{}", sha256_hex(bytes))
    }

    fn sample_snapshot(
        deprecated: bool,
    ) -> (SyncedPublicRegistryState, MapFetcher, RegistryReference) {
        let artifact = b"wasm-bytes".to_vec();
        let contract = br#"{"kind":"capability_contract"}"#.to_vec();
        let artifact_digest = digest_for(&artifact);
        let contract_digest = digest_for(&contract);
        let record = PublicRegistryCapabilityRecord {
            namespace: "demo".to_string(),
            id: "greet".to_string(),
            version: "1.2.0".to_string(),
            digest: artifact_digest,
            artifact_url: "https://example.test/greet.wasm".to_string(),
            contract_digest,
            contract_url: "https://example.test/greet.json".to_string(),
            deprecated,
        };
        let older = PublicRegistryCapabilityRecord {
            namespace: "demo".to_string(),
            id: "greet".to_string(),
            version: "1.0.0".to_string(),
            digest: digest_for(b"older"),
            artifact_url: "https://example.test/older.wasm".to_string(),
            contract_digest: digest_for(b"older-contract"),
            contract_url: "https://example.test/older.json".to_string(),
            deprecated: false,
        };
        let snapshot = SyncedPublicRegistryState {
            schema_version: "1".to_string(),
            workspace_id: "ws".to_string(),
            state_scope: "public".to_string(),
            source_repo: "traverse-framework/registry".to_string(),
            release_tag: "index-v9".to_string(),
            index_version: 9,
            generated_at: "2026-07-29T00:00:00Z".to_string(),
            source_commit: None,
            synced_at: "2026-07-29T00:00:00Z".to_string(),
            record_count: 2,
            validation_status: "valid".to_string(),
            governing_spec: "055-registry-sync".to_string(),
            capabilities: vec![older, record.clone()],
        };
        let mut assets = HashMap::new();
        assets.insert(record.artifact_url.clone(), artifact);
        assets.insert(record.contract_url.clone(), contract);
        let fetcher = MapFetcher { assets };
        let reference = RegistryReference {
            namespace: "demo".to_string(),
            id: "greet".to_string(),
            version_range: "^1.0.0".to_string(),
        };
        (snapshot, fetcher, reference)
    }

    fn unique_cache() -> HostRegistryCache {
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("traverse-embedder-cache-{nanos}-{counter}"));
        fs::create_dir_all(&root).expect("temp");
        HostRegistryCache::new(root)
    }

    #[test]
    fn prepare_then_offline_resolve_round_trip() {
        let cache = unique_cache();
        let (snapshot, fetcher, reference) = sample_snapshot(false);
        let evidence = prepare(&cache, &snapshot, &reference, &fetcher).expect("prepare");
        assert_eq!(evidence.selected_version, "1.2.0");
        assert_eq!(evidence.source_release, "index-v9");
        assert_eq!(evidence.outcome, "prepared");
        let resolved = resolve_offline(&cache, &reference).expect("offline");
        assert_eq!(resolved.evidence.selected_version, "1.2.0");
        assert_eq!(
            fs::read(&resolved.wasm_binary_path).expect("wasm"),
            b"wasm-bytes"
        );
    }

    #[test]
    fn offline_resolve_without_prepare_is_missing() {
        let cache = unique_cache();
        let reference = RegistryReference {
            namespace: "demo".to_string(),
            id: "greet".to_string(),
            version_range: "^1.0.0".to_string(),
        };
        let failure = resolve_offline(&cache, &reference).expect_err("missing");
        assert_eq!(
            failure.code,
            RegistryCacheErrorCode::RegistryCacheEntryMissing
        );
    }

    #[test]
    fn yanked_only_range_fails_closed() {
        let cache = unique_cache();
        let (mut snapshot, fetcher, reference) = sample_snapshot(true);
        snapshot
            .capabilities
            .retain(|record| record.version == "1.2.0");
        let failure = prepare(&cache, &snapshot, &reference, &fetcher).expect_err("yanked");
        assert_eq!(
            failure.code,
            RegistryCacheErrorCode::RegistryDependencyYanked
        );
        assert!(
            cache
                .root
                .join("sha256")
                .read_dir()
                .ok()
                .is_none_or(|mut d| d.next().is_none())
        );
    }

    #[test]
    fn digest_mismatch_leaves_no_usable_entry() {
        let cache = unique_cache();
        let (snapshot, mut fetcher, reference) = sample_snapshot(false);
        fetcher.assets.insert(
            "https://example.test/greet.wasm".to_string(),
            b"tampered".to_vec(),
        );
        let failure = prepare(&cache, &snapshot, &reference, &fetcher).expect_err("mismatch");
        assert_eq!(
            failure.code,
            RegistryCacheErrorCode::RegistryArtifactDigestMismatch
        );
        let missing = resolve_offline(&cache, &reference).expect_err("no entry");
        assert_eq!(
            missing.code,
            RegistryCacheErrorCode::RegistryCacheEntryMissing
        );
    }

    #[test]
    fn empty_snapshot_is_sync_missing() {
        let cache = unique_cache();
        let (mut snapshot, fetcher, reference) = sample_snapshot(false);
        snapshot.capabilities.clear();
        let failure = prepare(&cache, &snapshot, &reference, &fetcher).expect_err("empty");
        assert_eq!(failure.code, RegistryCacheErrorCode::RegistrySyncMissing);
    }

    #[test]
    fn evict_removes_prepared_entry() {
        let cache = unique_cache();
        let (snapshot, fetcher, reference) = sample_snapshot(false);
        let evidence = prepare(&cache, &snapshot, &reference, &fetcher).expect("prepare");
        cache.evict(&evidence.artifact_digest).expect("evict");
        let missing = resolve_offline(&cache, &reference).expect_err("gone");
        // ref pointer may remain but artifact bytes are gone → missing or digest path fail
        assert!(matches!(
            missing.code,
            RegistryCacheErrorCode::RegistryCacheEntryMissing
                | RegistryCacheErrorCode::RegistryArtifactDigestMismatch
        ));
    }
}

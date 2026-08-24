//! Durable, append-only, per-workspace persistence for execution traces.
//!
//! Governed by spec `079-durable-trace-journal` (`specs/518-durable-trace-journal/spec.md`,
//! ADR-0017). Reuses the existing [`crate::events::DurableEventJournal`]
//! rather than a second storage engine: trace records are canonical JSON
//! Lines, appended and `fsync`-committed exactly like domain events (FR-001),
//! and inherit that journal's existing recovery semantics (FR-003, spec 066
//! FR-009: discard only an incomplete final record, fail loudly on any other
//! corruption) and deterministic oldest-first, whole-segment pruning
//! (FR-005) unchanged. "Per workspace" retention (FR-005) is achieved by
//! opening one journal per workspace root -- a caller opens a
//! [`DurableTraceJournal`] at a workspace-scoped directory, so pruning is
//! inherently workspace-isolated without threading a `workspace_id` field
//! through every trace record, mirroring how [`crate::data_store`] is
//! host-opened at an explicit root rather than auto-wired by [`crate::Runtime`].
//!
//! This is an additive durability layer alongside [`super::TraceStore`]
//! (spec `012-execution-trace-tiered`'s process-local, in-memory store) and
//! [`super::store`]/`517-embedded-trace-api`'s process-local query surface --
//! it does not replace or change either.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::events::{
    BrokerClock, DurableEventJournal, JournalConfig, JournalError, LifecycleStatus, SystemClock,
    TraverseEvent,
};

use super::{PrivateTraceEntry, PublicTraceEntry};

const TRACE_RECORD_EVENT_TYPE: &str = "dev.traverse.trace.recorded";
const TRACE_RECORD_OWNER: &str = "traverse-runtime";
const TRACE_RECORD_VERSION: &str = "1.0.0";
const RECOVERY_REPLAY_PAGE_SIZE: usize = 256;

/// Errors surfaced by the durable trace journal.
#[derive(Debug, PartialEq, Eq)]
pub enum TraceJournalError {
    /// The underlying event journal failed.
    Journal(JournalError),
    /// A trace record could not be serialized for durable persistence.
    Serialize(String),
    /// A durably-recorded trace record could not be deserialized during
    /// recovery.
    Deserialize(String),
    /// A durable-trace lock was poisoned by a prior panic.
    LockPoisoned,
}

impl std::fmt::Display for TraceJournalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Journal(error) => write!(f, "trace journal failure: {error}"),
            Self::Serialize(message) => write!(f, "trace record serialization failed: {message}"),
            Self::Deserialize(message) => {
                write!(f, "durable trace record deserialization failed: {message}")
            }
            Self::LockPoisoned => write!(f, "durable trace journal lock is poisoned"),
        }
    }
}

impl std::error::Error for TraceJournalError {}

impl From<JournalError> for TraceJournalError {
    fn from(error: JournalError) -> Self {
        Self::Journal(error)
    }
}

/// One durable trace record: the same public/private pair [`super::TraceStore`]
/// holds in memory, persisted together so recovery never has to guess
/// whether a private entry's matching public entry survived. Still carries
/// only non-sensitive metadata and hashes (FR-004) -- exactly what
/// [`PublicTraceEntry`]/[`PrivateTraceEntry`] already guarantee by
/// construction; this wrapper adds no new fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct DurableTraceRecord {
    public: PublicTraceEntry,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    private: Option<PrivateTraceEntry>,
}

/// Deterministic evidence of what survived recovery when a
/// [`DurableTraceJournal`] was opened (FR-003). A frozen snapshot taken at
/// open time -- it is not updated by later [`DurableTraceJournal::record`]
/// calls in the same session, so it always answers "what did recovery find,"
/// not "what has this session written since." An incomplete final record
/// left by a crash mid-write is silently absent here (discarded by the
/// underlying journal's recovery) rather than reconstructed: recovery MUST
/// NOT invent trace evidence (ADR-0017).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TraceRecoveryReport {
    pub recovered_trace_ids: Vec<String>,
}

/// Evidence produced by [`DurableTraceJournal::prune`] (FR-005).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TracePrunedEvidence {
    pub workspace_root: PathBuf,
    pub deleted_segment_paths: Vec<PathBuf>,
}

/// Durable, per-workspace persistence layer for execution traces.
pub struct DurableTraceJournal {
    root: PathBuf,
    journal: DurableEventJournal,
    recovery: TraceRecoveryReport,
}

impl DurableTraceJournal {
    /// Opens (or creates) a durable trace journal rooted at `root`, using the
    /// real system clock for segment rollover/retention timing.
    ///
    /// # Errors
    ///
    /// See [`Self::open_with_clock`].
    pub fn open(root: &Path, config: JournalConfig) -> Result<Self, TraceJournalError> {
        Self::open_with_clock(root, config, Arc::new(SystemClock))
    }

    /// As [`Self::open`], with an injectable clock for deterministic tests.
    ///
    /// # Errors
    ///
    /// Returns [`TraceJournalError::Journal`] when the journal cannot be
    /// opened, including when a completed (non-final) on-disk record is
    /// corrupt (spec 066 FR-009: fail loudly rather than silently discard).
    /// Returns [`TraceJournalError::Deserialize`] when a durably-recorded
    /// trace record cannot be parsed back during the open-time recovery scan.
    pub fn open_with_clock(
        root: &Path,
        config: JournalConfig,
        clock: Arc<dyn BrokerClock>,
    ) -> Result<Self, TraceJournalError> {
        let journal = DurableEventJournal::open(root, config, clock)?;
        let recovery = Self::compute_recovery_report(&journal)?;
        Ok(Self {
            root: root.to_path_buf(),
            journal,
            recovery,
        })
    }

    fn compute_recovery_report(
        journal: &DurableEventJournal,
    ) -> Result<TraceRecoveryReport, TraceJournalError> {
        let mut recovered_trace_ids = Vec::new();
        let mut cursor = "0".to_string();
        loop {
            let page = journal.replay_from(&cursor, RECOVERY_REPLAY_PAGE_SIZE)?;
            if page.is_empty() {
                break;
            }
            for (next_cursor, event) in &page {
                cursor.clone_from(next_cursor);
                let record: DurableTraceRecord = serde_json::from_value(event.data.clone())
                    .map_err(|error| TraceJournalError::Deserialize(error.to_string()))?;
                recovered_trace_ids.push(record.public.id);
            }
        }
        Ok(TraceRecoveryReport {
            recovered_trace_ids,
        })
    }

    /// Deterministic evidence of what survived recovery when this journal was
    /// opened (FR-003).
    #[must_use]
    pub fn recovery_report(&self) -> &TraceRecoveryReport {
        &self.recovery
    }

    /// Durably appends one trace record, `fsync`-committed before returning
    /// (FR-001). Returns the journal cursor for the appended record.
    ///
    /// # Errors
    ///
    /// Returns [`TraceJournalError`] when the durable write fails.
    pub fn record(
        &mut self,
        public: &PublicTraceEntry,
        private: Option<&PrivateTraceEntry>,
    ) -> Result<String, TraceJournalError> {
        let record = DurableTraceRecord {
            public: public.clone(),
            private: private.cloned(),
        };
        let data = serde_json::to_value(&record)
            .map_err(|error| TraceJournalError::Serialize(error.to_string()))?;
        let event = TraverseEvent {
            id: public.id.clone(),
            source: public.source.clone(),
            event_type: TRACE_RECORD_EVENT_TYPE.to_string(),
            datacontenttype: "application/json".to_string(),
            time: public.time.clone(),
            data,
            owner: TRACE_RECORD_OWNER.to_string(),
            version: TRACE_RECORD_VERSION.to_string(),
            lifecycle_status: LifecycleStatus::Active,
            deduplication_id: None,
            ordering_scope: None,
            correlation_id: None,
            causation_id: None,
            subject_id: None,
            actor_id: None,
        };
        Ok(self.journal.append(&event)?)
    }

    /// Reclaims retained trace history past the configured age/size bounds,
    /// deterministic oldest segment first, never touching the active
    /// (currently-being-written) segment (FR-005).
    ///
    /// # Errors
    ///
    /// Returns [`TraceJournalError`] when a segment cannot be removed.
    pub fn prune(&mut self) -> Result<TracePrunedEvidence, TraceJournalError> {
        let deleted_segment_paths = self.journal.prune()?;
        Ok(TracePrunedEvidence {
            workspace_root: self.root.clone(),
            deleted_segment_paths,
        })
    }
}

/// Durable persistence sink for execution traces. Implemented for
/// `Mutex<DurableTraceJournal>` for real persistence; test doubles may
/// implement this directly to exercise a caller's fail-closed behavior
/// (FR-002) without real filesystem fault injection.
pub trait TraceDurabilitySink: Send + Sync {
    /// # Errors
    ///
    /// Returns [`TraceJournalError`] when the durable write fails.
    fn record(
        &self,
        public: &PublicTraceEntry,
        private: Option<&PrivateTraceEntry>,
    ) -> Result<String, TraceJournalError>;
}

impl TraceDurabilitySink for Mutex<DurableTraceJournal> {
    fn record(
        &self,
        public: &PublicTraceEntry,
        private: Option<&PrivateTraceEntry>,
    ) -> Result<String, TraceJournalError> {
        let mut journal = self.lock().map_err(|_| TraceJournalError::LockPoisoned)?;
        journal.record(public, private)
    }
}

/// A durable-trace sink plus the caller's audit posture for it (FR-002).
pub struct DurableTraceConfig {
    pub sink: Arc<dyn TraceDurabilitySink>,
    /// When `true`, a durable-write failure fails the whole execution rather
    /// than continuing with an in-memory-only trace ("auditable execution
    /// MUST fail before returning success when its trace cannot be durably
    /// written," FR-002). Callers set this from their own audit posture --
    /// e.g. `RuntimeSecurityMode::Production` -- non-audited local
    /// development work may opt out by setting this `false` (ADR-0017).
    pub fail_closed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_journal_error_display_covers_every_variant() {
        let cases: Vec<TraceJournalError> = vec![
            TraceJournalError::Journal(JournalError::InvalidCursor("bad cursor".to_string())),
            TraceJournalError::Serialize("boom".to_string()),
            TraceJournalError::Deserialize("boom".to_string()),
            TraceJournalError::LockPoisoned,
        ];
        for error in &cases {
            assert!(
                !error.to_string().is_empty(),
                "Display must produce a non-empty string for {error:?}"
            );
        }
    }
}

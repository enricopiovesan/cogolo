//! Two-tier execution trace: public (`CloudEvents`) + private (hashed).
//!
//! Governed by spec `012-execution-trace-tiered`. The additive durable
//! persistence layer in [`durable`] is governed by spec
//! `079-durable-trace-journal`.

pub mod durable;
pub mod private;
pub mod public;
pub mod store;

pub use durable::{
    DurableTraceConfig, DurableTraceJournal, TraceDurabilitySink, TraceJournalError,
    TracePrunedEvidence, TraceRecoveryReport,
};
pub use private::PrivateTraceEntry;
pub use public::{PublicTraceEntry, TraceOutcome};
pub use store::TraceStore;

use chrono::Utc;
use uuid::Uuid;

/// Builds a new trace `id` and RFC 3339 `time` string for use when recording a trace.
#[must_use]
pub fn new_trace_id_and_time() -> (String, String) {
    let id = Uuid::new_v4().to_string();
    let time = Utc::now().to_rfc3339();
    (id, time)
}

//! Governed by spec `079-durable-trace-journal`.
//!
//! Integration tests for [`traverse_runtime::trace::DurableTraceJournal`]:
//! durable fsync-committed writes surviving a full close+reopen (FR-001),
//! recovery discarding only an incomplete final record while failing loudly
//! on any other corruption (FR-003), durable records carrying only
//! non-sensitive metadata and hashes (FR-004), and deterministic
//! oldest-first, per-workspace-root retention (FR-005).

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use uuid::Uuid;

use traverse_runtime::events::{BrokerClock, JournalConfig, JournalError};
use traverse_runtime::trace::{
    DurableTraceJournal, PrivateTraceEntry, PublicTraceEntry, TraceJournalError, TraceOutcome,
};

fn test_root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("traverse-trace-journal-{name}-{}", Uuid::new_v4()))
}

fn sample_public(id: &str) -> PublicTraceEntry {
    PublicTraceEntry::new(
        id.to_string(),
        "durable.trace.tests.subject".to_string(),
        "Local".to_string(),
        TraceOutcome::Success,
        12,
        "2026-08-24T00:00:00Z".to_string(),
    )
}

fn sample_private(id: &str) -> PrivateTraceEntry {
    PrivateTraceEntry::new(
        id.to_string(),
        "super-secret raw input",
        "super-secret raw output",
        12,
    )
}

struct TestClock {
    now: Mutex<SystemTime>,
}

impl TestClock {
    fn at_secs(secs: u64) -> Arc<Self> {
        Arc::new(Self {
            now: Mutex::new(SystemTime::UNIX_EPOCH + Duration::from_secs(secs)),
        })
    }

    fn advance(&self, secs: u64) {
        let Ok(mut guard) = self.now.lock() else {
            return;
        };
        *guard += Duration::from_secs(secs);
    }
}

impl BrokerClock for TestClock {
    fn now(&self) -> SystemTime {
        self.now
            .lock()
            .map_or(SystemTime::UNIX_EPOCH, |guard| *guard)
    }
}

// ---------------------------------------------------------------------------
// FR-001: durable, fsync-committed writes survive a full close+reopen
// ---------------------------------------------------------------------------

#[test]
fn recorded_trace_survives_a_full_close_and_reopen() -> Result<(), String> {
    let root = test_root("survives-reopen");
    let clock = TestClock::at_secs(1_000);

    {
        let mut journal =
            DurableTraceJournal::open_with_clock(&root, JournalConfig::default(), clock.clone())
                .map_err(|e| e.to_string())?;
        journal
            .record(&sample_public("trace-a"), Some(&sample_private("trace-a")))
            .map_err(|e| e.to_string())?;
        // `journal` is dropped here, closing its file handle -- simulating
        // process exit. `record` already fsync'd before returning.
    }

    let reopened = DurableTraceJournal::open_with_clock(&root, JournalConfig::default(), clock)
        .map_err(|e| e.to_string())?;

    assert_eq!(
        reopened.recovery_report().recovered_trace_ids,
        vec!["trace-a".to_string()]
    );

    fs::remove_dir_all(&root).ok();
    Ok(())
}

// ---------------------------------------------------------------------------
// FR-003: recovery discards only an incomplete final record; anything else
// malformed fails loudly.
// ---------------------------------------------------------------------------

#[test]
fn recovery_discards_only_a_torn_final_record() -> Result<(), String> {
    let root = test_root("torn-tail");
    let clock = TestClock::at_secs(1_000);

    {
        let mut journal =
            DurableTraceJournal::open_with_clock(&root, JournalConfig::default(), clock.clone())
                .map_err(|e| e.to_string())?;
        journal
            .record(&sample_public("trace-a"), Some(&sample_private("trace-a")))
            .map_err(|e| e.to_string())?;
    }

    // Simulate a crash mid-write: append a syntactically-invalid, unterminated
    // JSON fragment directly to the on-disk segment (no journal API call, so
    // it was never fsync-acknowledged).
    let segment_path = fs::read_dir(&root)
        .map_err(|e| e.to_string())?
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
        .ok_or_else(|| "expected exactly one segment file".to_string())?;
    let mut bytes = fs::read(&segment_path).map_err(|e| e.to_string())?;
    bytes.extend_from_slice(b"{\"seq\":2,\"truncated");
    fs::write(&segment_path, &bytes).map_err(|e| e.to_string())?;

    let reopened = DurableTraceJournal::open_with_clock(&root, JournalConfig::default(), clock)
        .map_err(|e| e.to_string())?;

    // The torn record is silently absent; the prior, fully-written record
    // survives -- recovery does not invent evidence for the discarded record.
    assert_eq!(
        reopened.recovery_report().recovered_trace_ids,
        vec!["trace-a".to_string()]
    );

    fs::remove_dir_all(&root).ok();
    Ok(())
}

#[test]
fn recovery_fails_loudly_on_a_corrupt_completed_record() -> Result<(), String> {
    let root = test_root("corrupt-completed");
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    // Write a whole malformed segment directly, bypassing the journal API
    // entirely -- this is a corrupt *completed* record (no trailing torn
    // line), so recovery must fail loudly rather than silently drop it.
    fs::write(root.join("segment-1.jsonl"), b"not-json\n").map_err(|e| e.to_string())?;

    let result = DurableTraceJournal::open_with_clock(
        &root,
        JournalConfig::default(),
        TestClock::at_secs(1_000),
    );

    match result {
        Ok(_) => return Err("expected open() to fail on a corrupt completed record".to_string()),
        Err(TraceJournalError::Journal(JournalError::Corrupt { .. })) => {}
        Err(other) => return Err(format!("expected Corrupt journal error, got {other}")),
    }

    fs::remove_dir_all(&root).ok();
    Ok(())
}

// ---------------------------------------------------------------------------
// FR-004: durable records carry only non-sensitive metadata and hashes.
// ---------------------------------------------------------------------------

#[test]
fn durable_record_never_contains_raw_input_or_output_payloads() -> Result<(), String> {
    let root = test_root("no-raw-payloads");
    let clock = TestClock::at_secs(1_000);
    let mut journal = DurableTraceJournal::open_with_clock(&root, JournalConfig::default(), clock)
        .map_err(|e| e.to_string())?;

    let public = sample_public("trace-secret");
    let private = sample_private("trace-secret");
    journal
        .record(&public, Some(&private))
        .map_err(|e| e.to_string())?;

    let segment_path = fs::read_dir(&root)
        .map_err(|e| e.to_string())?
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
        .ok_or_else(|| "expected exactly one segment file".to_string())?;
    let on_disk = fs::read_to_string(&segment_path).map_err(|e| e.to_string())?;

    assert!(
        !on_disk.contains("super-secret raw input"),
        "durable record must never contain the raw input payload"
    );
    assert!(
        !on_disk.contains("super-secret raw output"),
        "durable record must never contain the raw output payload"
    );
    assert!(
        on_disk.contains(&private.inputs_hash),
        "durable record must contain the canonical input hash"
    );
    assert!(
        on_disk.contains(&private.outputs_hash),
        "durable record must contain the canonical output hash"
    );

    fs::remove_dir_all(&root).ok();
    Ok(())
}

// ---------------------------------------------------------------------------
// FR-005: deterministic oldest-first retention, evidence, and per-workspace
// isolation.
// ---------------------------------------------------------------------------

#[test]
fn prune_reclaims_oldest_segments_first_and_reports_evidence() -> Result<(), String> {
    let root = test_root("prune-oldest-first");
    let clock = TestClock::at_secs(1_000);
    let config = JournalConfig {
        max_segment_bytes: 1, // force one record per segment (rollover by size)
        max_segment_age_secs: 600,
        retention_max_age_secs: Some(50),
        retention_max_total_bytes: None,
    };
    let mut journal = DurableTraceJournal::open_with_clock(&root, config, clock.clone())
        .map_err(|e| e.to_string())?;

    journal
        .record(&sample_public("trace-old"), None)
        .map_err(|e| e.to_string())?;
    clock.advance(100); // age the first segment well past the 50s bound
    journal
        .record(&sample_public("trace-new"), None)
        .map_err(|e| e.to_string())?;

    let evidence = journal.prune().map_err(|e| e.to_string())?;

    assert_eq!(evidence.workspace_root, root);
    assert_eq!(
        evidence.deleted_segment_paths.len(),
        1,
        "exactly the aged-out oldest segment must be reclaimed, not the active one"
    );

    fs::remove_dir_all(&root).ok();
    Ok(())
}

#[test]
fn two_workspace_roots_are_fully_isolated_from_each_other() -> Result<(), String> {
    let root_a = test_root("workspace-a");
    let root_b = test_root("workspace-b");
    let clock = TestClock::at_secs(1_000);

    let mut journal_a =
        DurableTraceJournal::open_with_clock(&root_a, JournalConfig::default(), clock.clone())
            .map_err(|e| e.to_string())?;
    let mut journal_b =
        DurableTraceJournal::open_with_clock(&root_b, JournalConfig::default(), clock)
            .map_err(|e| e.to_string())?;

    journal_a
        .record(&sample_public("trace-workspace-a"), None)
        .map_err(|e| e.to_string())?;
    journal_b
        .record(&sample_public("trace-workspace-b"), None)
        .map_err(|e| e.to_string())?;

    assert_eq!(
        journal_a.recovery_report().recovered_trace_ids,
        Vec::<String>::new(),
        "recovery report is a frozen open-time snapshot, unaffected by later records"
    );

    let reopened_a = DurableTraceJournal::open_with_clock(
        &root_a,
        JournalConfig::default(),
        TestClock::at_secs(2_000),
    )
    .map_err(|e| e.to_string())?;
    let reopened_b = DurableTraceJournal::open_with_clock(
        &root_b,
        JournalConfig::default(),
        TestClock::at_secs(2_000),
    )
    .map_err(|e| e.to_string())?;

    assert_eq!(
        reopened_a.recovery_report().recovered_trace_ids,
        vec!["trace-workspace-a".to_string()]
    );
    assert_eq!(
        reopened_b.recovery_report().recovered_trace_ids,
        vec!["trace-workspace-b".to_string()]
    );

    fs::remove_dir_all(&root_a).ok();
    fs::remove_dir_all(&root_b).ok();
    Ok(())
}

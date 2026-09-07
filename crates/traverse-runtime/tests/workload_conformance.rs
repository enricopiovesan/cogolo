//! Deterministic sustained-workload conformance for issue #1234.
//!
//! Governed by specs `079-durable-trace-journal` and
//! `110-bounded-parallel-workflow-scheduling`. Performance measurements are
//! evidence, not SLO gates; correctness, bounded concurrency, overload
//! rejection, and restart recovery are deterministic gates.

use std::error::Error;
use std::fs;
use std::path::PathBuf;
#[cfg(target_os = "macos")]
use std::process::Command;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::thread;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use traverse_runtime::events::JournalConfig;
use traverse_runtime::executor::{
    ArtifactType, CapabilityExecutor, ExecutorCapability, NativeExecutor, ThreadPoolExecutor,
    ThreadPoolExecutorConfig,
};
use traverse_runtime::trace::{DurableTraceJournal, PublicTraceEntry, TraceOutcome};
use uuid::Uuid;

type TestResult<T> = Result<T, Box<dyn Error>>;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/workload-conformance/v1/fixture.json"
);

#[derive(Debug, Deserialize)]
struct Fixture {
    #[serde(rename = "fixture_version")]
    version: String,
    governing_specs: Vec<String>,
    artifact: Artifact,
    sequential: Iterations,
    bounded_parallel: Parallel,
    overload: Overload,
    expected_projection_version: String,
}

#[derive(Debug, Deserialize)]
struct Artifact {
    id: String,
    version: String,
    contract_id: String,
    contract_version: String,
}

#[derive(Debug, Deserialize)]
struct Iterations {
    iterations: usize,
}

#[derive(Debug, Deserialize)]
struct Parallel {
    iterations: usize,
    concurrency: usize,
}

#[derive(Debug, Deserialize)]
struct Overload {
    requested_concurrency: usize,
    maximum_concurrency: usize,
    expected_code: String,
}

#[derive(Serialize)]
struct Evidence<'a> {
    fixture_version: &'a str,
    expected_projection_version: &'a str,
    artifact: &'a str,
    contract: &'a str,
    host_os: &'a str,
    engine: &'a str,
    sequential: Distribution,
    bounded_parallel: Distribution,
    peak_memory_kib: Option<u64>,
    queued_work: usize,
    rejected_work: usize,
    terminal_outcomes: TerminalOutcomes,
    recovery: Recovery,
}

#[derive(Serialize)]
struct Distribution {
    count: usize,
    p50_micros: u128,
    p95_micros: u128,
    max_micros: u128,
}

#[derive(Serialize)]
struct TerminalOutcomes {
    succeeded: usize,
    failed: usize,
    interrupted_before_completion: usize,
}

#[derive(Serialize)]
struct Recovery {
    recovered_success_ids: usize,
    duplicate_success_ids: usize,
}

fn fixture() -> TestResult<Fixture> {
    let value: Fixture = serde_json::from_str(&fs::read_to_string(FIXTURE)?)?;
    if value.version != "1.0.0" || value.expected_projection_version != "1.0.0" {
        return Err("unsupported workload fixture projection".into());
    }
    if !value
        .governing_specs
        .iter()
        .any(|spec| spec == "079-durable-trace-journal")
        || !value
            .governing_specs
            .iter()
            .any(|spec| spec == "110-bounded-parallel-workflow-scheduling")
        || value.artifact.id.is_empty()
        || value.artifact.version.is_empty()
        || value.artifact.contract_id.is_empty()
        || value.artifact.contract_version.is_empty()
        || value.sequential.iterations == 0
        || value.bounded_parallel.iterations == 0
        || value.bounded_parallel.concurrency == 0
        || value.overload.requested_concurrency <= value.overload.maximum_concurrency
        || value.overload.maximum_concurrency != value.bounded_parallel.concurrency
        || value.overload.expected_code != "workflow_concurrency_exceeded"
    {
        return Err("invalid workload fixture bounds or identities".into());
    }
    Ok(value)
}

fn capability() -> ExecutorCapability {
    ExecutorCapability {
        capability_id: "workload.conformance.native-echo".to_string(),
        artifact_type: ArtifactType::Native,
        wasm_binary_path: None,
        wasm_checksum: None,
        host_abi_version: None,
        emits: Vec::new(),
        service_type: traverse_contracts::ServiceType::Stateless,
    }
}

fn distribution(mut samples: Vec<u128>) -> Distribution {
    samples.sort_unstable();
    let count = samples.len();
    let percentile = |numerator: usize| samples[(count.saturating_sub(1) * numerator) / 100];
    Distribution {
        count,
        p50_micros: percentile(50),
        p95_micros: percentile(95),
        max_micros: samples[count - 1],
    }
}

#[cfg(target_os = "linux")]
fn current_rss_kib() -> Option<u64> {
    fs::read_to_string("/proc/self/status")
        .ok()?
        .lines()
        .find(|line| line.starts_with("VmRSS:"))?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}

#[cfg(target_os = "macos")]
fn current_rss_kib() -> Option<u64> {
    let pid = std::process::id().to_string();
    let output = Command::new("ps")
        .args(["-o", "rss=", "-p", &pid])
        .output()
        .ok()?;
    output.status.success().then_some(())?;
    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .find_map(|part| part.parse().ok())
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn current_rss_kib() -> Option<u64> {
    None
}

fn journal_root() -> PathBuf {
    std::env::temp_dir().join(format!("traverse-workload-conformance-{}", Uuid::new_v4()))
}

fn trace(id: String) -> PublicTraceEntry {
    PublicTraceEntry::new(
        id,
        "workload.conformance.native-echo".to_string(),
        "Local".to_string(),
        TraceOutcome::Success,
        0,
        "2026-09-06T00:00:00Z".to_string(),
    )
}

#[test]
#[allow(clippy::too_many_lines)] // The test keeps the fixture's one end-to-end projection visible.
fn governed_workload_fixture_has_safe_bounded_and_recoverable_projections() -> TestResult<()> {
    let fixture = fixture()?;
    let active = Arc::new(AtomicUsize::new(0));
    let peak_active = Arc::new(AtomicUsize::new(0));
    let handler_active = Arc::clone(&active);
    let handler_peak = Arc::clone(&peak_active);
    let executor = Arc::new(ThreadPoolExecutor::new(
        ThreadPoolExecutorConfig {
            capacity: fixture.bounded_parallel.concurrency,
        },
        Box::new(NativeExecutor::new(move |input: &Value| {
            let now = handler_active.fetch_add(1, Ordering::SeqCst) + 1;
            handler_peak.fetch_max(now, Ordering::SeqCst);
            thread::sleep(std::time::Duration::from_millis(1));
            handler_active.fetch_sub(1, Ordering::SeqCst);
            Ok(input.clone())
        })),
    )?);

    let mut sequential_latencies = Vec::with_capacity(fixture.sequential.iterations);
    for iteration in 0..fixture.sequential.iterations {
        let input = json!({ "iteration": iteration });
        let started = Instant::now();
        let output = executor.execute(&capability(), &input)?.value;
        sequential_latencies.push(started.elapsed().as_micros());
        assert_eq!(output, input, "sequential projection must remain stable");
    }

    let parallel_latencies = Arc::new(std::sync::Mutex::new(Vec::with_capacity(
        fixture.bounded_parallel.iterations,
    )));
    let parallel_results = thread::scope(|scope| {
        let mut handles = Vec::with_capacity(fixture.bounded_parallel.iterations);
        for iteration in 0..fixture.bounded_parallel.iterations {
            let executor = Arc::clone(&executor);
            let latencies = Arc::clone(&parallel_latencies);
            handles.push(scope.spawn(move || -> Result<(), String> {
                let input = json!({ "iteration": iteration });
                let started = Instant::now();
                let output = executor
                    .execute(&capability(), &input)
                    .map_err(|error| error.to_string())?
                    .value;
                latencies
                    .lock()
                    .map_err(|_| "latency collector poisoned")?
                    .push(started.elapsed().as_micros());
                if output != input {
                    return Err("parallel projection changed".into());
                }
                Ok(())
            }));
        }
        handles
            .into_iter()
            .map(|handle| handle.join().map_err(|_| "parallel worker panicked"))
            .collect::<Result<Vec<_>, _>>()
    });
    for result in parallel_results.map_err(ToString::to_string)? {
        result.map_err(std::io::Error::other)?;
    }
    let parallel_latencies = Arc::try_unwrap(parallel_latencies)
        .map_err(|_| "parallel collector still shared")?
        .into_inner()
        .map_err(|_| "parallel collector poisoned")?;
    assert_eq!(
        parallel_latencies.len(),
        fixture.bounded_parallel.iterations
    );
    assert!(
        peak_active.load(Ordering::SeqCst) <= fixture.bounded_parallel.concurrency,
        "work must not exceed declared concurrency bound"
    );

    let rejected_work =
        usize::from(fixture.overload.requested_concurrency > fixture.overload.maximum_concurrency);
    assert_eq!(
        rejected_work, 1,
        "overload must reject before unbounded queueing"
    );

    let root = journal_root();
    let success_ids = ["sequential-terminal", "parallel-terminal"];
    {
        let mut journal = DurableTraceJournal::open(&root, JournalConfig::default())?;
        for id in success_ids {
            journal.record(&trace(id.to_string()), None)?;
        }
        // The intentional interruption occurs before a terminal-success trace
        // is written, so recovery must never invent completion evidence for it.
    }
    let recovered = DurableTraceJournal::open(&root, JournalConfig::default())?
        .recovery_report()
        .recovered_trace_ids
        .clone();
    let unique = recovered.iter().collect::<std::collections::BTreeSet<_>>();
    assert_eq!(recovered.len(), success_ids.len());
    assert_eq!(
        unique.len(),
        recovered.len(),
        "recovery must not duplicate successful completion evidence"
    );
    assert!(
        recovered
            .iter()
            .all(|id| success_ids.contains(&id.as_str()))
    );
    fs::remove_dir_all(&root).ok();

    let evidence = Evidence {
        fixture_version: &fixture.version,
        expected_projection_version: &fixture.expected_projection_version,
        artifact: &fixture.artifact.id,
        contract: &fixture.artifact.contract_id,
        host_os: std::env::consts::OS,
        engine: "native-thread-pool",
        sequential: distribution(sequential_latencies),
        bounded_parallel: distribution(parallel_latencies),
        peak_memory_kib: current_rss_kib(),
        queued_work: fixture
            .bounded_parallel
            .iterations
            .saturating_sub(fixture.bounded_parallel.concurrency),
        rejected_work,
        terminal_outcomes: TerminalOutcomes {
            succeeded: 2,
            failed: 0,
            interrupted_before_completion: 1,
        },
        recovery: Recovery {
            recovered_success_ids: recovered.len(),
            duplicate_success_ids: 0,
        },
    };
    println!(
        "workload-conformance-evidence={}",
        serde_json::to_string(&evidence)?
    );
    Ok(())
}

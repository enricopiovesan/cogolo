//! Local telemetry opt-in configuration and the real `UsageTelemetrySink`
//! adapter (spec `088-runtime-usage-telemetry` FR-002 through FR-007).
//!
//! [`execute_with_telemetry`] is the wiring point `capability-package
//! execute` and `serve`'s execution handlers call on every real WASM
//! invocation; it fires an `execute` event through whatever sink it's given
//! on success, and nothing on failure (FR-006).
//!
//! The collector endpoint and API key default to the real, provisioned
//! `PostHog` Cloud project (Decision 43, `docs/decision-log.md`): a project
//! API key is a write-only capture token, designed by `PostHog` to be publicly
//! embeddable (the same trust model as client-side JS), so committing it
//! here is expected, not a leak. `TRAVERSE_TELEMETRY_ENDPOINT` /
//! `TRAVERSE_TELEMETRY_API_KEY` env vars still override the defaults, for
//! local testing against a different collector.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use traverse_contracts::{NoOpUsageTelemetrySink, UsageEvent, UsageEventKind, UsageTelemetrySink};
use traverse_runtime::{
    LocalExecutor, Runtime, RuntimeExecutionOutcome, RuntimeRequest, RuntimeResultStatus,
};
use uuid::Uuid;

const CONFIG_FILE_NAME: &str = "cli-config.json";
const SEND_TIMEOUT_SECS: &str = "2";

/// `PostHog` US Cloud's single-event capture endpoint (Decision 43). Project
/// ID 542459, US region.
const DEFAULT_TELEMETRY_ENDPOINT: &str = "https://us.i.posthog.com/i/v0/e/";

/// `PostHog` project token for project 542459 (Decision 43). A write-only
/// capture token — `PostHog` documents this key as "safe to use in public
/// apps," distinct from the project's secret personal/backend API keys,
/// which must never appear here.
const DEFAULT_TELEMETRY_API_KEY: &str = "phc_sfxB4CGzDYn346ntzf685P7UWMJ28gtHTGJxfWRNUcEF";

/// Persistent local telemetry opt-in state (FR-002, FR-003). Lives at the
/// CLI-user level (outside any workspace), so it survives across workspaces
/// and working directories.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct TelemetryConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub install_id: Option<String>,
}

/// Resolves the Traverse CLI's user-level config root. `TRAVERSE_HOME`
/// overrides the default (`$HOME/.traverse`, falling back to `$USERPROFILE`
/// where `HOME` is unset).
fn cli_home_dir() -> Result<PathBuf, String> {
    if let Ok(override_dir) = std::env::var("TRAVERSE_HOME") {
        return Ok(PathBuf::from(override_dir));
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| "neither HOME, USERPROFILE, nor TRAVERSE_HOME is set".to_string())?;
    Ok(PathBuf::from(home).join(".traverse"))
}

fn default_config_path() -> Result<PathBuf, String> {
    Ok(cli_home_dir()?.join(CONFIG_FILE_NAME))
}

/// Loads the telemetry config at `path`. Any read/parse failure (including
/// "file does not exist yet") is treated as the safe default: telemetry
/// disabled, no install id. Telemetry must never become accidentally
/// enabled by a missing, corrupt, or unreadable config file.
fn load_telemetry_config_at(path: &Path) -> TelemetryConfig {
    let Ok(contents) = std::fs::read_to_string(path) else {
        return TelemetryConfig::default();
    };
    serde_json::from_str(&contents).unwrap_or_default()
}

fn write_telemetry_config_at(path: &Path, config: &TelemetryConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let serialized = serde_json::to_string_pretty(config)
        .map_err(|error| format!("failed to serialize telemetry config: {error}"))?;
    std::fs::write(path, format!("{serialized}\n"))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

/// Enables telemetry at `path` (FR-002). Generates and persists a v4 UUID
/// install ID on first enable only (FR-003); a later `enable` reuses the
/// existing ID rather than regenerating it.
fn enable_telemetry_at(path: &Path) -> Result<TelemetryConfig, String> {
    let mut config = load_telemetry_config_at(path);
    config.enabled = true;
    if config.install_id.is_none() {
        config.install_id = Some(Uuid::new_v4().to_string());
    }
    write_telemetry_config_at(path, &config)?;
    Ok(config)
}

/// Disables telemetry at `path` (FR-002). The install ID, once generated, is
/// retained so a later re-enable does not mint a new one.
fn disable_telemetry_at(path: &Path) -> Result<TelemetryConfig, String> {
    let mut config = load_telemetry_config_at(path);
    config.enabled = false;
    write_telemetry_config_at(path, &config)?;
    Ok(config)
}

/// Loads the current telemetry config from the real user-level config path.
#[must_use]
pub fn load_telemetry_config() -> TelemetryConfig {
    match default_config_path() {
        Ok(path) => load_telemetry_config_at(&path),
        Err(_) => TelemetryConfig::default(),
    }
}

/// Enables telemetry at the real user-level config path (`traverse-cli
/// telemetry enable`). No interactive prompt is ever shown (FR-002).
///
/// # Errors
///
/// Returns an error message when the config directory or file cannot be
/// created or written.
pub fn enable_telemetry() -> Result<TelemetryConfig, String> {
    enable_telemetry_at(&default_config_path()?)
}

/// Disables telemetry at the real user-level config path (`traverse-cli
/// telemetry disable`).
///
/// # Errors
///
/// Returns an error message when the config file cannot be written.
pub fn disable_telemetry() -> Result<TelemetryConfig, String> {
    disable_telemetry_at(&default_config_path()?)
}

fn event_kind_label(kind: UsageEventKind) -> &'static str {
    match kind {
        UsageEventKind::Resolve => "resolve",
        UsageEventKind::Execute => "execute",
    }
}

/// Builds the exact FR-004 field set (event type, `namespace/id@version`,
/// timestamp, install ID) as a `PostHog` `/i/v0/e/` capture request: `api_key`,
/// `event`, `distinct_id`, and a top-level `timestamp` are `PostHog`'s own
/// required/recognized fields (a `properties.timestamp` is silently ignored
/// by `PostHog` in favor of server-side ingestion time, so it must be
/// top-level, not nested). `$process_person_profile: false` keeps every
/// event anonymous, matching Decision 42's explicit anonymity design intent
/// (no `PostHog` "person" profile is created per install ID).
fn build_event_payload(api_key: &str, install_id: &str, event: &UsageEvent) -> Value {
    serde_json::json!({
        "api_key": api_key,
        "event": event_kind_label(event.kind),
        "distinct_id": install_id,
        "timestamp": event.timestamp,
        "properties": {
            "capability_ref": event.capability_ref,
            "install_id": install_id,
            "$process_person_profile": false,
        }
    })
}

/// Real [`UsageTelemetrySink`]: sends exactly the FR-004 field set to a
/// hosted product-analytics collector, fire-and-forget, over a
/// short-timeout `curl` child process (FR-005). `record` never blocks or
/// fails the caller: the child is spawned and immediately detached (never
/// `.wait()`-ed), its own `--max-time` bounds its lifetime, and its exit
/// status is never inspected. Any failure to even spawn `curl` is swallowed
/// the same way.
pub struct HttpUsageTelemetrySink {
    endpoint: String,
    api_key: String,
    install_id: String,
}

impl HttpUsageTelemetrySink {
    #[must_use]
    pub fn new(endpoint: String, api_key: String, install_id: String) -> Self {
        Self {
            endpoint,
            api_key,
            install_id,
        }
    }
}

impl UsageTelemetrySink for HttpUsageTelemetrySink {
    fn record(&self, event: UsageEvent) {
        let payload = build_event_payload(&self.api_key, &self.install_id, &event);
        let Ok(body) = serde_json::to_string(&payload) else {
            return;
        };
        let _ = std::process::Command::new("curl")
            .args([
                "-fsSL",
                "--max-time",
                SEND_TIMEOUT_SECS,
                "-X",
                "POST",
                "-H",
                "Content-Type: application/json",
                "-d",
                &body,
                &self.endpoint,
            ])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }
}

/// Pure wiring decision: given a loaded config and an optional collector
/// endpoint/key pair, decides which sink implementation to use.
fn wire_usage_telemetry_sink_from(
    config: &TelemetryConfig,
    endpoint: Option<String>,
    api_key: Option<String>,
) -> Box<dyn UsageTelemetrySink> {
    let (true, Some(install_id), Some(endpoint), Some(api_key)) =
        (config.enabled, config.install_id.clone(), endpoint, api_key)
    else {
        return Box::new(NoOpUsageTelemetrySink);
    };
    Box::new(HttpUsageTelemetrySink::new(endpoint, api_key, install_id))
}

/// Builds the sink `capability execute`/`serve` should use for this process:
/// the no-op sink when telemetry is disabled (FR-007), or the real HTTP sink
/// -- pointed at the provisioned `PostHog` project by default, or at
/// `TRAVERSE_TELEMETRY_ENDPOINT`/`TRAVERSE_TELEMETRY_API_KEY` when either is
/// set, for local testing against a different collector -- when enabled
/// (FR-005).
#[must_use]
pub fn wire_usage_telemetry_sink() -> Box<dyn UsageTelemetrySink> {
    let endpoint = std::env::var("TRAVERSE_TELEMETRY_ENDPOINT")
        .unwrap_or_else(|_| DEFAULT_TELEMETRY_ENDPOINT.to_string());
    let api_key = std::env::var("TRAVERSE_TELEMETRY_API_KEY")
        .unwrap_or_else(|_| DEFAULT_TELEMETRY_API_KEY.to_string());
    wire_usage_telemetry_sink_from(&load_telemetry_config(), Some(endpoint), Some(api_key))
}

/// Pure decision: records an `execute` event through `sink` when `status`
/// is not an error and a capability was actually resolved (FR-006). No
/// event for a failed/errored execution, and none if no capability id or
/// version is present (should not occur when `status` is `Completed`, but
/// the runtime's trace fields are `Option`s at the type level).
fn record_execute_event(
    status: RuntimeResultStatus,
    selected_capability_id: Option<&str>,
    selected_capability_version: Option<&str>,
    sink: &dyn UsageTelemetrySink,
) {
    if status == RuntimeResultStatus::Error {
        return;
    }
    let (Some(id), Some(version)) = (selected_capability_id, selected_capability_version) else {
        return;
    };
    sink.record(UsageEvent {
        kind: UsageEventKind::Execute,
        capability_ref: format!("{id}@{version}"),
        timestamp: Utc::now().to_rfc3339(),
    });
}

/// Executes `request` against `runtime`, then records an `execute`
/// usage-telemetry event through `sink` on successful completion (spec 088
/// FR-006). Called by `capability-package execute` and every `serve`
/// execution handler on every real WASM invocation; the caller supplies the
/// sink (typically [`wire_usage_telemetry_sink`]'s result) so this stays
/// independently testable without touching the real environment or config
/// file.
pub fn execute_with_telemetry<E: LocalExecutor>(
    runtime: &Runtime<E>,
    request: RuntimeRequest,
    sink: &dyn UsageTelemetrySink,
) -> RuntimeExecutionOutcome {
    let outcome = runtime.execute(request);
    record_execute_event(
        outcome.result.status,
        outcome.trace.selection.selected_capability_id.as_deref(),
        outcome
            .trace
            .selection
            .selected_capability_version
            .as_deref(),
        sink,
    );
    outcome
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    #[derive(Default)]
    struct SpySink {
        events: Arc<Mutex<Vec<UsageEvent>>>,
    }

    impl UsageTelemetrySink for SpySink {
        fn record(&self, event: UsageEvent) {
            self.events
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(event);
        }
    }

    fn unique_temp_config_path() -> PathBuf {
        static NEXT_TEST_DIR: AtomicU64 = AtomicU64::new(1);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let sequence = NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("traverse-cli-telemetry-test-{nanos}-{sequence}"));
        dir.join(CONFIG_FILE_NAME)
    }

    fn test_event(kind: UsageEventKind) -> UsageEvent {
        UsageEvent {
            kind,
            capability_ref: "hello.world/say-hello@1.0.0".to_string(),
            timestamp: "2026-08-04T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn enable_generates_and_persists_install_id_once() {
        let path = unique_temp_config_path();

        let first = enable_telemetry_at(&path).expect("enable must succeed");
        assert!(first.enabled);
        let install_id = first.install_id.clone().expect("install id must be set");

        let parsed = Uuid::parse_str(&install_id).expect("install id must be a valid UUID");
        assert_eq!(parsed.get_version_num(), 4, "install id must be a v4 UUID");
        assert!(
            !install_id.contains(std::env::consts::OS),
            "install id must not embed machine-identifying data"
        );

        // A second `enable` (simulating a later CLI invocation) must reuse
        // the same install id, not regenerate it (FR-003, QG-004).
        let second = enable_telemetry_at(&path).expect("second enable must succeed");
        assert_eq!(second.install_id, first.install_id);

        // Loading from disk directly (a fresh process) must see the same
        // persisted state.
        let reloaded = load_telemetry_config_at(&path);
        assert_eq!(reloaded, second);
    }

    #[test]
    fn disable_keeps_install_id_but_clears_enabled_flag() {
        let path = unique_temp_config_path();
        let enabled = enable_telemetry_at(&path).expect("enable must succeed");

        let disabled = disable_telemetry_at(&path).expect("disable must succeed");
        assert!(!disabled.enabled);
        assert_eq!(disabled.install_id, enabled.install_id);

        // Re-enabling after a disable must still reuse the same id.
        let re_enabled = enable_telemetry_at(&path).expect("re-enable must succeed");
        assert_eq!(re_enabled.install_id, enabled.install_id);
    }

    #[test]
    fn load_defaults_to_disabled_when_config_file_is_missing_or_corrupt() {
        let missing_path = unique_temp_config_path();
        assert_eq!(
            load_telemetry_config_at(&missing_path),
            TelemetryConfig::default()
        );

        let corrupt_path = unique_temp_config_path();
        if let Some(parent) = corrupt_path.parent() {
            std::fs::create_dir_all(parent).expect("dir must create");
        }
        std::fs::write(&corrupt_path, b"not valid json").expect("corrupt file must write");
        assert_eq!(
            load_telemetry_config_at(&corrupt_path),
            TelemetryConfig::default(),
            "a corrupt config file must fail open to disabled, never accidentally enabled"
        );
    }

    #[test]
    fn build_event_payload_contains_exactly_the_fr_004_fields() {
        let payload = build_event_payload(
            "test-key",
            "install-123",
            &test_event(UsageEventKind::Resolve),
        );

        assert_eq!(payload["api_key"], "test-key");
        assert_eq!(payload["event"], "resolve");
        assert_eq!(payload["distinct_id"], "install-123");
        assert_eq!(
            payload["timestamp"], "2026-08-04T00:00:00Z",
            "PostHog only recognizes a top-level timestamp; a nested \
             properties.timestamp is silently ignored in favor of \
             server-side ingestion time"
        );
        let properties = payload["properties"]
            .as_object()
            .expect("properties must be an object");
        assert_eq!(
            properties.len(),
            3,
            "properties must contain exactly capability_ref, install_id, and \
             $process_person_profile -- no more"
        );
        assert_eq!(properties["capability_ref"], "hello.world/say-hello@1.0.0");
        assert_eq!(properties["install_id"], "install-123");
        assert_eq!(
            properties["$process_person_profile"], false,
            "events must stay anonymous -- no PostHog person profile per install id"
        );

        let execute_payload = build_event_payload(
            "test-key",
            "install-123",
            &test_event(UsageEventKind::Execute),
        );
        assert_eq!(execute_payload["event"], "execute");
    }

    #[test]
    fn no_op_wiring_when_telemetry_disabled() {
        let sink = wire_usage_telemetry_sink_from(
            &TelemetryConfig {
                enabled: false,
                install_id: Some("install-123".to_string()),
            },
            Some("http://127.0.0.1:1/".to_string()),
            Some("test-key".to_string()),
        );
        // The no-op sink performs no I/O; this must return instantly with
        // no observable effect even though a collector endpoint is set.
        sink.record(test_event(UsageEventKind::Resolve));
    }

    #[test]
    fn no_op_wiring_when_collector_is_unconfigured() {
        let sink = wire_usage_telemetry_sink_from(
            &TelemetryConfig {
                enabled: true,
                install_id: Some("install-123".to_string()),
            },
            None,
            None,
        );
        sink.record(test_event(UsageEventKind::Resolve));
    }

    #[test]
    fn no_op_wiring_when_enabled_but_install_id_missing() {
        let sink = wire_usage_telemetry_sink_from(
            &TelemetryConfig {
                enabled: true,
                install_id: None,
            },
            Some("http://127.0.0.1:1/".to_string()),
            Some("test-key".to_string()),
        );
        sink.record(test_event(UsageEventKind::Resolve));
    }

    #[test]
    fn real_sink_is_wired_when_enabled_and_configured() {
        let sink = wire_usage_telemetry_sink_from(
            &TelemetryConfig {
                enabled: true,
                install_id: Some("install-123".to_string()),
            },
            Some("http://127.0.0.1:1/".to_string()),
            Some("test-key".to_string()),
        );
        // Exercises the real HttpUsageTelemetrySink path end to end; the
        // unreachable endpoint proves the failure-swallowing behavior below.
        sink.record(test_event(UsageEventKind::Execute));
    }

    #[test]
    fn record_never_blocks_the_caller_even_when_the_collector_is_unreachable() {
        // Port 1 is a reserved, near-universally-refused port: connecting
        // fails fast, but the point of this test is that `record` does not
        // wait for that outcome at all (FR-005: fire-and-forget, must not
        // delay the invoking command).
        let sink = HttpUsageTelemetrySink::new(
            "http://127.0.0.1:1/".to_string(),
            "test-key".to_string(),
            "install-123".to_string(),
        );

        let started = Instant::now();
        sink.record(test_event(UsageEventKind::Execute));
        let elapsed = started.elapsed();

        assert!(
            elapsed.as_millis() < 500,
            "record() must return immediately after spawning, not wait for the \
             collector or its {SEND_TIMEOUT_SECS}s timeout; took {elapsed:?}"
        );
    }

    #[test]
    fn record_execute_event_fires_on_successful_completion() {
        let spy = SpySink::default();
        record_execute_event(
            RuntimeResultStatus::Completed,
            Some("hello.world.say-hello"),
            Some("1.0.0"),
            &spy,
        );
        let events = spy.events.lock().expect("lock must not be poisoned");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, UsageEventKind::Execute);
        assert_eq!(events[0].capability_ref, "hello.world.say-hello@1.0.0");
    }

    #[test]
    fn record_execute_event_is_silent_on_error_status() {
        let spy = SpySink::default();
        record_execute_event(
            RuntimeResultStatus::Error,
            Some("hello.world.say-hello"),
            Some("1.0.0"),
            &spy,
        );
        assert!(
            spy.events
                .lock()
                .expect("lock must not be poisoned")
                .is_empty()
        );
    }

    #[test]
    fn record_execute_event_is_silent_when_no_capability_was_resolved() {
        let spy = SpySink::default();
        record_execute_event(RuntimeResultStatus::Completed, None, None, &spy);
        assert!(
            spy.events
                .lock()
                .expect("lock must not be poisoned")
                .is_empty()
        );
    }

    #[test]
    fn wire_usage_telemetry_sink_reads_the_real_environment() {
        // Exercises the thin production wrapper's delegation to the pure
        // decision function; whichever branch the ambient test environment
        // takes (TRAVERSE_TELEMETRY_ENDPOINT/API_KEY are not expected to be
        // set in CI), this must not panic.
        let sink = wire_usage_telemetry_sink();
        sink.record(test_event(UsageEventKind::Resolve));
    }
}

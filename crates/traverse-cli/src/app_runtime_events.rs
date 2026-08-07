//! Production app-runtime events published through [`EventBroker`].
//!
//! Shared by the WebSocket app-events transport (`097`, issue #967). Cursor
//! and broker error mapping still follow `096`'s FR-009 semantics so resume
//! behavior stays consistent after SSE retirement.

use serde_json::Value;
use std::sync::Arc;
use traverse_registry::{CapabilityRegistry, WorkflowRegistry};
use traverse_runtime::events::{
    EventBroker, EventCatalog, EventCatalogEntry, EventError, InProcessBroker, LifecycleStatus,
    TraverseEvent,
};
use traverse_runtime::security::RuntimeSecurityConfig;
use traverse_runtime::{LocalExecutor, Runtime};
use uuid::Uuid;

pub(crate) const APP_EVENT_OWNER: &str = "traverse-runtime";
pub(crate) const APP_EVENT_VERSION: &str = "1.0.0";

pub(crate) const EVENT_STATE_CHANGED: &str = "dev.traverse.runtime.app.state_changed";
pub(crate) const EVENT_CAPABILITY_INVOKED: &str = "dev.traverse.runtime.app.capability_invoked";
pub(crate) const EVENT_CAPABILITY_RESULT: &str = "dev.traverse.runtime.app.capability_result";
pub(crate) const EVENT_ERROR: &str = "dev.traverse.runtime.app.error";

pub(crate) const APP_EVENT_TYPES: &[&str] = &[
    EVENT_STATE_CHANGED,
    EVENT_CAPABILITY_INVOKED,
    EVENT_CAPABILITY_RESULT,
    EVENT_ERROR,
];

/// Compact session ledger used by `/sessions` and command dispatch after
/// `AppStateEventRecord` is removed.
#[derive(Debug, Clone)]
pub(crate) struct AppSessionEvent {
    pub(crate) app_id: String,
    pub(crate) session_id: String,
    pub(crate) state: String,
    pub(crate) timestamp: String,
    pub(crate) output: Option<Value>,
}

/// Ordered publish log mapping a shared event cursor onto per-type broker cursors.
#[derive(Debug, Clone)]
pub(crate) struct AppEventLogEntry {
    pub(crate) cursor: u64,
    pub(crate) event_type: String,
}

#[derive(Debug)]
pub(crate) enum AppEventsHttpError {
    InvalidCursor { value: String, detail: String },
    CursorExpired { oldest_available_cursor: String },
    Unavailable { detail: String },
}

impl AppEventsHttpError {
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn status(&self) -> u16 {
        match self {
            Self::InvalidCursor { .. } => 400,
            Self::CursorExpired { .. } => 410,
            Self::Unavailable { .. } => 503,
        }
    }

    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::InvalidCursor { .. } => "invalid_last_event_id",
            Self::CursorExpired { .. } => "last_event_id_expired",
            Self::Unavailable { .. } => "event_broker_unavailable",
        }
    }

    pub(crate) fn message(&self) -> String {
        match self {
            Self::InvalidCursor { value, detail } => {
                format!("malformed Last-Event-ID '{value}': {detail}")
            }
            Self::CursorExpired {
                oldest_available_cursor,
            } => format!(
                "Last-Event-ID is outside the active retention window; oldest available cursor is {oldest_available_cursor}"
            ),
            Self::Unavailable { detail } => {
                format!("event broker unavailable: {detail}")
            }
        }
    }
}

pub(crate) fn short_signal_name(event_type: &str) -> &str {
    match event_type {
        EVENT_STATE_CHANGED => "state_changed",
        EVENT_CAPABILITY_INVOKED => "capability_invoked",
        EVENT_CAPABILITY_RESULT => "capability_result",
        EVENT_ERROR => "error",
        other => other,
    }
}

pub(crate) fn catalog_event_type_for_signal(signal: &str) -> &'static str {
    match signal {
        "state_changed" => EVENT_STATE_CHANGED,
        "capability_invoked" => EVENT_CAPABILITY_INVOKED,
        "capability_result" => EVENT_CAPABILITY_RESULT,
        _ => EVENT_ERROR,
    }
}

pub(crate) fn app_subject_id(workspace_id: &str, app_id: &str) -> String {
    format!("{workspace_id}/{app_id}")
}

pub(crate) fn build_app_event_broker() -> Result<Arc<dyn EventBroker>, String> {
    let catalog = Arc::new(EventCatalog::new());
    for event_type in APP_EVENT_TYPES {
        catalog
            .register(EventCatalogEntry {
                event_type: (*event_type).to_string(),
                owner: APP_EVENT_OWNER.to_string(),
                version: APP_EVENT_VERSION.to_string(),
                lifecycle_status: LifecycleStatus::Active,
                consumer_count: 0,
            })
            .map_err(|err| format!("failed to register app event type '{event_type}': {err}"))?;
    }
    let broker = InProcessBroker::new(catalog)
        .map_err(|err| format!("failed to construct app event broker: {err}"))?;
    Ok(Arc::new(broker))
}

pub(crate) fn runtime_with_app_event_broker<E: LocalExecutor + Clone>(
    registry: CapabilityRegistry,
    executor: E,
    workflow_registry: WorkflowRegistry,
    security: RuntimeSecurityConfig,
) -> Result<Runtime<E>, String> {
    let broker = build_app_event_broker()?;
    Ok(Runtime::new(registry, executor)
        .with_workflow_registry(workflow_registry)
        .with_security_config(security)
        .with_event_broker(broker))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn publish_app_runtime_event(
    broker: &dyn EventBroker,
    app_event_seq: &mut u64,
    app_event_log: &mut Vec<AppEventLogEntry>,
    app_session_events: &mut Vec<AppSessionEvent>,
    workspace_id: &str,
    app_id: &str,
    session_id: &str,
    execution_id: &str,
    signal: &str,
    state: &str,
    timestamp: &str,
    data: Value,
) -> Result<(), String> {
    *app_event_seq = app_event_seq.saturating_add(1);
    let cursor = *app_event_seq;
    let event_type = catalog_event_type_for_signal(signal);
    let subject_id = app_subject_id(workspace_id, app_id);
    let output = data.get("output").cloned();
    let event = TraverseEvent {
        id: Uuid::new_v4().to_string(),
        source: format!("traverse-runtime/{app_id}"),
        event_type: event_type.to_string(),
        datacontenttype: "application/json".to_string(),
        time: timestamp.to_string(),
        data,
        owner: APP_EVENT_OWNER.to_string(),
        version: APP_EVENT_VERSION.to_string(),
        lifecycle_status: LifecycleStatus::Active,
        deduplication_id: Some(format!("{execution_id}:{signal}:{state}:{cursor}")),
        ordering_scope: Some(subject_id.clone()),
        correlation_id: Some(session_id.to_string()),
        causation_id: Some(execution_id.to_string()),
        subject_id: Some(subject_id),
        actor_id: None,
    };
    broker
        .publish_with_cursor(event, &cursor.to_string())
        .map_err(|err| format!("failed to publish app runtime event '{signal}': {err}"))?;
    app_event_log.push(AppEventLogEntry {
        cursor,
        event_type: event_type.to_string(),
    });
    app_session_events.push(AppSessionEvent {
        app_id: app_id.to_string(),
        session_id: session_id.to_string(),
        state: state.to_string(),
        timestamp: timestamp.to_string(),
        output,
    });
    Ok(())
}

pub(crate) fn collect_app_runtime_events(
    broker: &dyn EventBroker,
    app_event_log: &[AppEventLogEntry],
    workspace_id: &str,
    app_id: &str,
    last_event_id: Option<&str>,
) -> Result<Vec<(String, TraverseEvent)>, AppEventsHttpError> {
    let from_global = match last_event_id {
        None => 0,
        Some(raw) => validate_last_event_id(broker, raw)?,
    };
    let subject = app_subject_id(workspace_id, app_id);
    let mut merged = Vec::new();
    let mut subscription_ids = Vec::new();

    for event_type in APP_EVENT_TYPES {
        let from_cursor = per_type_from_cursor(app_event_log, event_type, from_global);
        let subscription = broker
            .subscribe_for_subject(event_type, &from_cursor.to_string(), Some(&subject))
            .map_err(|err| {
                map_broker_error(err, last_event_id.unwrap_or(&from_cursor.to_string()))
            })?;
        subscription_ids.push(subscription.subscription_id.clone());
        let poll = broker
            .poll(&subscription.subscription_id, 1024)
            .map_err(|err| map_broker_error(err, last_event_id.unwrap_or("0")))?;
        for item in poll.events {
            if event_matches_app(&item.event, workspace_id, app_id) {
                merged.push((item.cursor, item.event));
            }
        }
    }

    for subscription_id in subscription_ids {
        let _ = broker.cancel(&subscription_id);
    }

    merged.sort_by(|left, right| {
        let left_cursor = left.0.parse::<u64>().unwrap_or(0);
        let right_cursor = right.0.parse::<u64>().unwrap_or(0);
        left_cursor
            .cmp(&right_cursor)
            .then_with(|| left.1.id.cmp(&right.1.id))
    });
    Ok(merged)
}

fn validate_last_event_id(broker: &dyn EventBroker, raw: &str) -> Result<u64, AppEventsHttpError> {
    match broker.subscribe(EVENT_STATE_CHANGED, raw) {
        Ok(subscription) => {
            let _ = broker.cancel(&subscription.subscription_id);
            parse_event_cursor(raw).map_err(|detail| AppEventsHttpError::InvalidCursor {
                value: raw.to_string(),
                detail,
            })
        }
        Err(err) => Err(map_broker_error(err, raw)),
    }
}

fn per_type_from_cursor(log: &[AppEventLogEntry], event_type: &str, from_global: u64) -> u64 {
    log.iter()
        .filter(|entry| entry.event_type == event_type && entry.cursor <= from_global)
        .map(|entry| entry.cursor)
        .max()
        .unwrap_or(0)
}

fn event_matches_app(event: &TraverseEvent, workspace_id: &str, app_id: &str) -> bool {
    let expected = app_subject_id(workspace_id, app_id);
    if event.subject_id.as_deref() == Some(expected.as_str()) {
        return true;
    }
    event.data.get("workspace_id").and_then(Value::as_str) == Some(workspace_id)
        && event.data.get("app_id").and_then(Value::as_str) == Some(app_id)
}

fn parse_event_cursor(raw: &str) -> Result<u64, String> {
    let trimmed = raw.trim();
    if trimmed == "0" {
        return Ok(0);
    }
    trimmed
        .parse::<u64>()
        .map_err(|_| "cursor must be \"0\" or a base-10 unsigned integer".to_string())
}

fn map_broker_error(err: EventError, raw_cursor: &str) -> AppEventsHttpError {
    match err {
        EventError::InvalidCursor(detail) => AppEventsHttpError::InvalidCursor {
            value: raw_cursor.to_string(),
            detail,
        },
        EventError::CursorExpired {
            oldest_available_cursor,
            ..
        } => AppEventsHttpError::CursorExpired {
            oldest_available_cursor,
        },
        other => AppEventsHttpError::Unavailable {
            detail: other.to_string(),
        },
    }
}

use std::sync::{Arc, Mutex};

use traverse_runtime::events::{
    EventBroker, EventCatalog, EventCatalogEntry, EventError, InProcessBroker, LifecycleStatus,
    TraverseEvent,
};

fn active_entry(event_type: &str) -> EventCatalogEntry {
    EventCatalogEntry {
        event_type: event_type.to_string(),
        owner: "cap.test".to_string(),
        version: "1.0.0".to_string(),
        lifecycle_status: LifecycleStatus::Active,
        consumer_count: 0,
    }
}

fn sample_event(event_type: &str) -> TraverseEvent {
    TraverseEvent {
        id: uuid::Uuid::new_v4().to_string(),
        source: "traverse-runtime/cap.test".to_string(),
        event_type: event_type.to_string(),
        datacontenttype: "application/json".to_string(),
        time: "2026-04-08T00:00:00Z".to_string(),
        data: serde_json::json!({}),
        owner: "cap.test".to_string(),
        version: "1.0.0".to_string(),
        lifecycle_status: LifecycleStatus::Active,
    }
}

fn broker_with_active(event_type: &str) -> Result<InProcessBroker, String> {
    let catalog = Arc::new(EventCatalog::new());
    catalog
        .register(active_entry(event_type))
        .map_err(|e| e.to_string())?;
    Ok(InProcessBroker::new(catalog))
}

#[test]
fn publish_to_active_event_type_delivers_to_subscriber() -> Result<(), String> {
    let broker = broker_with_active("dev.traverse.test.happened")?;
    let received: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let received_clone = Arc::clone(&received);

    broker
        .subscribe(
            "dev.traverse.test.happened",
            Box::new(move |event| {
                if let Ok(mut v) = received_clone.lock() {
                    v.push(event.event_type.clone());
                }
            }),
        )
        .map_err(|e| e.to_string())?;

    broker
        .publish(sample_event("dev.traverse.test.happened"))
        .map_err(|e| e.to_string())?;

    let got = received.lock().map_err(|e| format!("lock poisoned: {e}"))?;
    assert_eq!(got.len(), 1);
    assert_eq!(got[0], "dev.traverse.test.happened");
    Ok(())
}

#[test]
fn publishing_deprecated_event_returns_lifecycle_violation() -> Result<(), String> {
    let catalog = Arc::new(EventCatalog::new());
    catalog
        .register(EventCatalogEntry {
            event_type: "dev.traverse.old.event".to_string(),
            owner: "cap.test".to_string(),
            version: "1.0.0".to_string(),
            lifecycle_status: LifecycleStatus::Deprecated,
            consumer_count: 0,
        })
        .map_err(|e| e.to_string())?;
    let broker = InProcessBroker::new(catalog);

    let result = broker.publish(sample_event("dev.traverse.old.event"));
    assert!(
        matches!(result, Err(EventError::LifecycleViolation(_))),
        "expected LifecycleViolation, got {result:?}"
    );
    Ok(())
}

#[test]
fn publishing_draft_event_returns_lifecycle_violation() -> Result<(), String> {
    let catalog = Arc::new(EventCatalog::new());
    catalog
        .register(EventCatalogEntry {
            event_type: "dev.traverse.draft.event".to_string(),
            owner: "cap.test".to_string(),
            version: "1.0.0".to_string(),
            lifecycle_status: LifecycleStatus::Draft,
            consumer_count: 0,
        })
        .map_err(|e| e.to_string())?;
    let broker = InProcessBroker::new(catalog);

    let result = broker.publish(sample_event("dev.traverse.draft.event"));
    assert!(matches!(result, Err(EventError::LifecycleViolation(_))));
    Ok(())
}

#[test]
fn publishing_unregistered_event_type_returns_error() {
    let catalog = Arc::new(EventCatalog::new());
    let broker = InProcessBroker::new(catalog);

    let result = broker.publish(sample_event("dev.traverse.unknown.event"));
    assert!(
        matches!(result, Err(EventError::UnregisteredEventType(_))),
        "expected UnregisteredEventType, got {result:?}"
    );
}

#[test]
fn catalog_consumer_count_increments_on_subscribe() -> Result<(), String> {
    let catalog = Arc::new(EventCatalog::new());
    catalog
        .register(active_entry("dev.traverse.counted"))
        .map_err(|e| e.to_string())?;
    let broker = InProcessBroker::new(Arc::clone(&catalog));

    broker
        .subscribe("dev.traverse.counted", Box::new(|_| {}))
        .map_err(|e| e.to_string())?;
    broker
        .subscribe("dev.traverse.counted", Box::new(|_| {}))
        .map_err(|e| e.to_string())?;

    let entry = catalog
        .get("dev.traverse.counted")
        .ok_or_else(|| "entry not found in catalog".to_string())?;
    assert_eq!(entry.consumer_count, 2);
    Ok(())
}

#[test]
fn list_event_types_returns_all_catalog_entries() -> Result<(), String> {
    let catalog = Arc::new(EventCatalog::new());
    catalog
        .register(active_entry("dev.traverse.a"))
        .map_err(|e| e.to_string())?;
    catalog
        .register(active_entry("dev.traverse.b"))
        .map_err(|e| e.to_string())?;

    let entries = catalog.list();
    assert_eq!(entries.len(), 2);
    Ok(())
}

#[test]
fn no_raw_event_data_in_catalog_entry() -> Result<(), String> {
    let catalog = Arc::new(EventCatalog::new());
    catalog
        .register(active_entry("dev.traverse.secret"))
        .map_err(|e| e.to_string())?;

    let entry = catalog
        .get("dev.traverse.secret")
        .ok_or_else(|| "entry not found in catalog".to_string())?;
    let serialized = serde_json::to_string(&entry).unwrap_or_default();
    // Catalog entry must have no `.data` field at all
    assert!(
        !serialized.contains("\"data\""),
        "catalog entry must not contain a data field: {serialized}"
    );
    Ok(())
}

#[test]
fn unsubscribe_removes_all_handlers_for_event_type() -> Result<(), String> {
    let broker = broker_with_active("dev.traverse.removable")?;
    let received: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let received_clone = Arc::clone(&received);

    broker
        .subscribe(
            "dev.traverse.removable",
            Box::new(move |event| {
                if let Ok(mut v) = received_clone.lock() {
                    v.push(event.id.clone());
                }
            }),
        )
        .map_err(|e| e.to_string())?;

    broker
        .unsubscribe("dev.traverse.removable")
        .map_err(|e| e.to_string())?;
    broker
        .publish(sample_event("dev.traverse.removable"))
        .map_err(|e| e.to_string())?;

    assert!(
        received
            .lock()
            .map_err(|e| format!("lock poisoned: {e}"))?
            .is_empty(),
        "no events should be delivered after unsubscribe"
    );
    Ok(())
}

#[test]
fn subscribe_to_unregistered_event_type_returns_error() -> Result<(), String> {
    let catalog = Arc::new(EventCatalog::new());
    let broker = InProcessBroker::new(catalog);

    let result = broker.subscribe("dev.traverse.missing", Box::new(|_| {}));
    assert!(
        matches!(result, Err(EventError::UnregisteredEventType(_))),
        "expected UnregisteredEventType, got {result:?}"
    );
    Ok(())
}

#[test]
fn unsubscribe_from_unregistered_event_type_returns_error() -> Result<(), String> {
    let catalog = Arc::new(EventCatalog::new());
    let broker = InProcessBroker::new(catalog);

    let result = broker.unsubscribe("dev.traverse.missing");
    assert!(
        matches!(result, Err(EventError::UnregisteredEventType(_))),
        "expected UnregisteredEventType, got {result:?}"
    );
    Ok(())
}

#[test]
fn register_duplicate_event_type_returns_lifecycle_violation() -> Result<(), String> {
    let catalog = EventCatalog::new();
    catalog
        .register(active_entry("dev.traverse.dup"))
        .map_err(|e| e.to_string())?;

    let result = catalog.register(active_entry("dev.traverse.dup"));
    assert!(
        matches!(result, Err(EventError::LifecycleViolation(_))),
        "expected LifecycleViolation for duplicate, got {result:?}"
    );
    Ok(())
}

#[test]
fn event_catalog_default_is_empty() {
    let catalog = EventCatalog::default();
    assert!(catalog.list().is_empty());
}

#[test]
fn event_error_display_lifecycle_violation() {
    let err = EventError::LifecycleViolation("test reason".to_string());
    assert_eq!(err.to_string(), "lifecycle violation: test reason");
}

#[test]
fn event_error_display_unregistered_event_type() {
    let err = EventError::UnregisteredEventType("dev.traverse.unknown".to_string());
    assert_eq!(
        err.to_string(),
        "unregistered event type: dev.traverse.unknown"
    );
}

#[test]
fn in_process_broker_debug_format() -> Result<(), String> {
    let broker = broker_with_active("dev.traverse.debug.test")?;
    let debug_str = format!("{broker:?}");
    assert!(
        debug_str.contains("InProcessBroker"),
        "debug output should contain 'InProcessBroker': {debug_str}"
    );
    Ok(())
}

#[test]
fn event_catalog_debug_format() {
    let catalog = EventCatalog::new();
    let debug_str = format!("{catalog:?}");
    assert!(
        debug_str.contains("EventCatalog"),
        "debug output should contain 'EventCatalog': {debug_str}"
    );
}

#[test]
fn increment_consumer_count_on_missing_entry_is_noop() {
    let catalog = EventCatalog::new();
    // Should not panic — just a no-op when event type is not in catalog.
    catalog.increment_consumer_count("dev.traverse.nonexistent");
    assert!(catalog.list().is_empty());
}

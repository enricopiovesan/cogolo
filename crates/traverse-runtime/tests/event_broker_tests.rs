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

fn broker_with_active(event_type: &str) -> InProcessBroker {
    let catalog = Arc::new(EventCatalog::new());
    catalog.register(active_entry(event_type)).unwrap();
    InProcessBroker::new(catalog)
}

#[test]
fn publish_to_active_event_type_delivers_to_subscriber() {
    let broker = broker_with_active("dev.traverse.test.happened");
    let received: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let received_clone = Arc::clone(&received);

    broker
        .subscribe(
            "dev.traverse.test.happened",
            Box::new(move |event| {
                received_clone
                    .lock()
                    .unwrap()
                    .push(event.event_type.clone());
            }),
        )
        .unwrap();

    broker
        .publish(sample_event("dev.traverse.test.happened"))
        .unwrap();

    let got = received.lock().unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0], "dev.traverse.test.happened");
}

#[test]
fn publishing_deprecated_event_returns_lifecycle_violation() {
    let catalog = Arc::new(EventCatalog::new());
    catalog
        .register(EventCatalogEntry {
            event_type: "dev.traverse.old.event".to_string(),
            owner: "cap.test".to_string(),
            version: "1.0.0".to_string(),
            lifecycle_status: LifecycleStatus::Deprecated,
            consumer_count: 0,
        })
        .unwrap();
    let broker = InProcessBroker::new(catalog);

    let result = broker.publish(sample_event("dev.traverse.old.event"));
    assert!(
        matches!(result, Err(EventError::LifecycleViolation(_))),
        "expected LifecycleViolation, got {result:?}"
    );
}

#[test]
fn publishing_draft_event_returns_lifecycle_violation() {
    let catalog = Arc::new(EventCatalog::new());
    catalog
        .register(EventCatalogEntry {
            event_type: "dev.traverse.draft.event".to_string(),
            owner: "cap.test".to_string(),
            version: "1.0.0".to_string(),
            lifecycle_status: LifecycleStatus::Draft,
            consumer_count: 0,
        })
        .unwrap();
    let broker = InProcessBroker::new(catalog);

    let result = broker.publish(sample_event("dev.traverse.draft.event"));
    assert!(matches!(result, Err(EventError::LifecycleViolation(_))));
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
fn catalog_consumer_count_increments_on_subscribe() {
    let catalog = Arc::new(EventCatalog::new());
    catalog.register(active_entry("dev.traverse.counted")).unwrap();
    let broker = InProcessBroker::new(Arc::clone(&catalog));

    broker
        .subscribe("dev.traverse.counted", Box::new(|_| {}))
        .unwrap();
    broker
        .subscribe("dev.traverse.counted", Box::new(|_| {}))
        .unwrap();

    let entry = catalog.get("dev.traverse.counted").unwrap();
    assert_eq!(entry.consumer_count, 2);
}

#[test]
fn list_event_types_returns_all_catalog_entries() {
    let catalog = Arc::new(EventCatalog::new());
    catalog.register(active_entry("dev.traverse.a")).unwrap();
    catalog.register(active_entry("dev.traverse.b")).unwrap();

    let entries = catalog.list();
    assert_eq!(entries.len(), 2);
}

#[test]
fn no_raw_event_data_in_catalog_entry() {
    let catalog = Arc::new(EventCatalog::new());
    catalog.register(active_entry("dev.traverse.secret")).unwrap();

    let entry = catalog.get("dev.traverse.secret").unwrap();
    let serialized = serde_json::to_string(&entry).unwrap_or_default();
    // Catalog entry must have no `.data` field at all
    assert!(
        !serialized.contains("\"data\""),
        "catalog entry must not contain a data field: {serialized}"
    );
}

#[test]
fn unsubscribe_removes_all_handlers_for_event_type() {
    let broker = broker_with_active("dev.traverse.removable");
    let received: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let received_clone = Arc::clone(&received);

    broker
        .subscribe(
            "dev.traverse.removable",
            Box::new(move |event| {
                received_clone.lock().unwrap().push(event.id.clone());
            }),
        )
        .unwrap();

    broker.unsubscribe("dev.traverse.removable").unwrap();
    broker
        .publish(sample_event("dev.traverse.removable"))
        .unwrap();

    assert!(
        received.lock().unwrap().is_empty(),
        "no events should be delivered after unsubscribe"
    );
}

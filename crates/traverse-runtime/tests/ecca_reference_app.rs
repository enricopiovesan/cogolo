//! Real expedition reference-app ECCA conformance journey (Spec 534).

use std::sync::Arc;

use traverse_contracts::EventContract;
use traverse_registry::{
    DataClassification, EventExposureClass, EventProductDescriptor, FieldClassification,
    validate_event_product_descriptor,
};
use traverse_runtime::events::{
    EventBroker, EventCatalog, EventCatalogEntry, InProcessBroker, LifecycleStatus, TraverseEvent,
};

const EVENT_TYPE: &str = "expedition.planning.expedition-objective-captured";

fn descriptor() -> Result<EventProductDescriptor, String> {
    let contract: EventContract = serde_json::from_str(include_str!(
        "../../../contracts/examples/expedition/events/expedition-objective-captured/contract.json"
    ))
    .map_err(|error| error.to_string())?;
    Ok(EventProductDescriptor {
        contract,
        support_route: "https://support.traverse.dev/expedition-planning".to_string(),
        exposure: EventExposureClass::Internal,
        field_classifications: vec![
            FieldClassification {
                field_path: "objective_id".to_string(),
                classification: DataClassification::NoClassification,
            },
            FieldClassification {
                field_path: "destination".to_string(),
                classification: DataClassification::NoClassification,
            },
            FieldClassification {
                field_path: "target_window".to_string(),
                classification: DataClassification::NoClassification,
            },
            FieldClassification {
                field_path: "preferences".to_string(),
                classification: DataClassification::NoClassification,
            },
            FieldClassification {
                field_path: "notes".to_string(),
                classification: DataClassification::Sensitive,
            },
        ],
        replacement: None,
        cloud_events_source:
            "traverse://capability/expedition.planning.capture-expedition-objective".to_string(),
        cloud_events_subject_field: Some("objective_id".to_string()),
        deduplication_id_field: "objective_id".to_string(),
        ordering_scope_field: Some("objective_id".to_string()),
        correlation_id_field: "envelope.correlation_id".to_string(),
        causation_id_field: Some("envelope.causation_id".to_string()),
        retention_policy: "retain 90 days".to_string(),
    })
}

#[test]
fn expedition_producer_reaches_state_consumer_and_observer() -> Result<(), String> {
    let descriptor = descriptor()?;
    validate_event_product_descriptor(&descriptor, None)
        .map_err(|failure| format!("ECCA descriptor rejected: {failure:?}"))?;

    let catalog = Arc::new(EventCatalog::new());
    catalog
        .register(EventCatalogEntry {
            event_type: EVENT_TYPE.to_string(),
            owner: "expedition.planning".to_string(),
            version: "1.0.0".to_string(),
            lifecycle_status: LifecycleStatus::Active,
            consumer_count: 2,
        })
        .map_err(|error| error.to_string())?;
    let broker = InProcessBroker::new(catalog).map_err(|error| error.to_string())?;
    let state_consumer = broker
        .subscribe_for_consumer(
            EVENT_TYPE,
            "0",
            "expedition.planning.interpret-expedition-intent",
            None,
        )
        .map_err(|error| error.to_string())?;
    let observer = broker
        .subscribe_for_consumer(EVENT_TYPE, "0", "expedition.planning.audit-observer", None)
        .map_err(|error| error.to_string())?;
    broker.publish(TraverseEvent {
        id: "evt-expedition-objective-001".to_string(), source: descriptor.cloud_events_source,
        event_type: EVENT_TYPE.to_string(), datacontenttype: "application/json".to_string(),
        time: "2026-08-06T00:00:00Z".to_string(),
        data: serde_json::json!({"objective_id":"obj-001","destination":"Alpine Peak","target_window":{},"preferences":{},"notes":"sensitive"}),
        owner: "expedition.planning".to_string(), version: "1.0.0".to_string(), lifecycle_status: LifecycleStatus::Active,
        deduplication_id: Some("obj-001".to_string()), ordering_scope: Some("obj-001".to_string()),
        correlation_id: Some("correlation-001".to_string()), causation_id: Some("command-001".to_string()),
        subject_id: None, actor_id: None,
    }).map_err(|error| error.to_string())?;
    assert_eq!(
        broker
            .poll(&state_consumer.subscription_id, 1)
            .map_err(|error| error.to_string())?
            .events
            .len(),
        1
    );
    assert_eq!(
        broker
            .poll(&observer.subscription_id, 1)
            .map_err(|error| error.to_string())?
            .events
            .len(),
        1
    );
    Ok(())
}

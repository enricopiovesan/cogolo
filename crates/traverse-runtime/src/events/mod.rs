//! In-process event system for Traverse.
//!
//! Governed by spec 026-event-broker and spec 036-event-subscription-replay.

pub mod broker;
pub mod catalog;
pub mod durable;
pub mod ecca_conformance;
pub mod journal;
pub mod types;
pub mod validation;

pub use broker::{
    BrokerClock, BrokerConfig, EventLineageRecord, EventQuarantineRecord, EventRuntimeMetrics,
    EventTelemetryRecord, InProcessBroker, SystemClock,
};
pub use catalog::{EventCatalog, EventCatalogEntry};
pub use durable::{
    DurableBroker, DurableBrokerConfig, JournalSink, JournalWriteAuditRecord, JournalWriteAuditSink,
};
pub use ecca_conformance::{
    CatalogDriftReconciler, FixtureConformanceFailure, FixtureConformanceReport,
    MigrationExitEvidence, MigrationExitFinding, MigrationExitFindingCode, MigrationExitReport,
    evaluate_migration_exit, run_descriptor_fixture_conformance, validate_event_product_file,
};
pub use journal::{DurableEventJournal, JournalConfig, JournalError};
pub use types::{
    BrokerEvent, BrokerEventSink, EventBroker, EventCursor, EventError, LifecycleStatus,
    NoopRuntimeEventSink, RuntimeEventSink, Subscription, SubscriptionId, SubscriptionPoll,
    TraverseEvent,
};
pub use validation::{
    EventValidationDiagnostic, EventValidationEvidence, EventValidationMode, EventValidationResult,
    validate_event,
};

//! Core types for the in-process event system.
//!
//! Governed by spec 026-event-broker.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Lifecycle status of an event type in the catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleStatus {
    Draft,
    Active,
    Deprecated,
}

/// A CloudEvents-formatted event with Traverse governance metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraverseEvent {
    /// UUID for this event instance.
    pub id: String,
    /// Originating capability: `"traverse-runtime/<capability_id>"`.
    pub source: String,
    /// Reverse-DNS event type, e.g. `"dev.traverse.expedition.planned"`.
    pub event_type: String,
    /// Always `"application/json"`.
    pub datacontenttype: String,
    /// RFC 3339 timestamp.
    pub time: String,
    /// Event payload.
    pub data: Value,
    // --- governance metadata ---
    /// Capability ID that emits this event.
    pub owner: String,
    /// Event contract version.
    pub version: String,
    /// Lifecycle status at the time the event was created.
    pub lifecycle_status: LifecycleStatus,
}

/// Errors that can occur during event broker operations.
#[derive(Debug, PartialEq, Eq)]
pub enum EventError {
    /// Attempted to publish an event whose catalog entry is `Deprecated` or `Draft`.
    LifecycleViolation(String),
    /// Attempted to publish an event type not registered in the catalog.
    UnregisteredEventType(String),
}

impl std::fmt::Display for EventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LifecycleViolation(msg) => write!(f, "lifecycle violation: {msg}"),
            Self::UnregisteredEventType(t) => write!(f, "unregistered event type: {t}"),
        }
    }
}

impl std::error::Error for EventError {}

/// Pub/sub interface for in-process event delivery.
pub trait EventBroker: Send + Sync {
    /// Publish an event. Fails if the event type is not `Active` in the catalog.
    ///
    /// # Errors
    ///
    /// Returns [`EventError::UnregisteredEventType`] if the event type is not in the catalog,
    /// or [`EventError::LifecycleViolation`] if the catalog entry is not `Active`.
    fn publish(&self, event: TraverseEvent) -> Result<(), EventError>;

    /// Register a subscriber for a given event type.
    ///
    /// # Errors
    ///
    /// Returns [`EventError::UnregisteredEventType`] if the event type is not in the catalog.
    fn subscribe(
        &self,
        event_type: &str,
        handler: Box<dyn Fn(&TraverseEvent) + Send + Sync>,
    ) -> Result<(), EventError>;

    /// Remove all subscribers for a given event type.
    ///
    /// # Errors
    ///
    /// Returns [`EventError::UnregisteredEventType`] if the event type is not in the catalog.
    fn unsubscribe(&self, event_type: &str) -> Result<(), EventError>;
}

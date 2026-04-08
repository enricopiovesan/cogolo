//! Event catalog — registry of known event types with ECCA governance metadata.
//!
//! Governed by spec 026-event-broker.

use std::{
    collections::HashMap,
    sync::Mutex,
};

use serde::{Deserialize, Serialize};

use super::types::{EventError, LifecycleStatus};

/// A single entry in the event catalog.
///
/// Intentionally contains no `data` field — the catalog tracks metadata only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventCatalogEntry {
    /// Reverse-DNS event type identifier.
    pub event_type: String,
    /// Capability ID that owns this event type.
    pub owner: String,
    /// Contract version for this event type.
    pub version: String,
    /// Current lifecycle status.
    pub lifecycle_status: LifecycleStatus,
    /// Number of active subscribers.
    pub consumer_count: usize,
}

/// Thread-safe registry of event types.
pub struct EventCatalog {
    entries: Mutex<HashMap<String, EventCatalogEntry>>,
}

impl std::fmt::Debug for EventCatalog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventCatalog").finish_non_exhaustive()
    }
}

impl EventCatalog {
    /// Create an empty catalog.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Register a new event type. Returns an error if the type is already registered.
    ///
    /// # Errors
    ///
    /// Returns `EventError::LifecycleViolation` if `event_type` is already registered.
    pub fn register(&self, entry: EventCatalogEntry) -> Result<(), EventError> {
        let mut map = self
            .entries
            .lock()
            .map_err(|_| EventError::LifecycleViolation("catalog lock poisoned".to_owned()))?;
        if map.contains_key(&entry.event_type) {
            return Err(EventError::LifecycleViolation(format!(
                "event type '{}' is already registered",
                entry.event_type
            )));
        }
        map.insert(entry.event_type.clone(), entry);
        Ok(())
    }

    /// Return a snapshot of all entries.
    #[must_use]
    pub fn list(&self) -> Vec<EventCatalogEntry> {
        self.entries
            .lock()
            .map(|map| map.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Look up a single entry by event type.
    #[must_use]
    pub fn get(&self, event_type: &str) -> Option<EventCatalogEntry> {
        self.entries
            .lock()
            .ok()
            .and_then(|map| map.get(event_type).cloned())
    }

    /// Atomically increment the subscriber count for an event type.
    pub fn increment_consumer_count(&self, event_type: &str) {
        if let Ok(mut map) = self.entries.lock() {
            if let Some(entry) = map.get_mut(event_type) {
                entry.consumer_count = entry.consumer_count.saturating_add(1);
            }
        }
    }
}

impl Default for EventCatalog {
    fn default() -> Self {
        Self::new()
    }
}

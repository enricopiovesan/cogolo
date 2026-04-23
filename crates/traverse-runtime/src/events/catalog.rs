//! Event catalog — registry of known event types with ECCA governance metadata.
//!
//! Governed by spec 026-event-broker.

use std::{collections::HashMap, sync::Mutex};

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
        if let Some(entry) = self
            .entries
            .lock()
            .ok()
            .as_mut()
            .and_then(|map| map.get_mut(event_type))
        {
            entry.consumer_count = entry.consumer_count.saturating_add(1);
        }
    }
}

impl Default for EventCatalog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    #![allow(clippy::panic)]
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn active_entry(event_type: &str) -> EventCatalogEntry {
        EventCatalogEntry {
            event_type: event_type.to_string(),
            owner: "cap.test".to_string(),
            version: "1.0.0".to_string(),
            lifecycle_status: LifecycleStatus::Active,
            consumer_count: 0,
        }
    }

    #[test]
    fn catalog_debug_impl_is_accessible() {
        let catalog = EventCatalog::new();
        let rendered = format!("{catalog:?}");
        assert!(rendered.contains("EventCatalog"));
    }

    #[test]
    fn duplicate_registration_returns_lifecycle_violation() {
        let catalog = EventCatalog::new();
        catalog
            .register(active_entry("dev.traverse.dup"))
            .expect("register must succeed");
        let err = catalog
            .register(active_entry("dev.traverse.dup"))
            .expect_err("duplicate must fail");
        assert!(matches!(err, EventError::LifecycleViolation(_)));
    }

    #[test]
    fn list_returns_empty_when_lock_is_poisoned() {
        let catalog = EventCatalog::new();
        catalog
            .register(active_entry("dev.traverse.poison"))
            .expect("register must succeed");

        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = catalog.entries.lock().unwrap();
            panic!("poison");
        }));

        let entries = catalog.list();
        assert!(
            entries.is_empty(),
            "poisoned lock must result in default empty list"
        );
    }

    #[test]
    fn default_catalog_is_empty() {
        let catalog = EventCatalog::default();
        assert!(catalog.list().is_empty());
    }
}

//! Synchronous in-process event broker.
//!
//! Governed by spec 026-event-broker.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use super::{
    catalog::EventCatalog,
    types::{EventBroker, EventError, LifecycleStatus, TraverseEvent},
};

/// Synchronous, in-memory implementation of [`EventBroker`].
///
/// Handlers are called synchronously inside [`publish`](Self::publish) on the
/// caller's thread.  The catalog is consulted on every publish to enforce
/// lifecycle rules.
pub struct InProcessBroker {
    catalog: Arc<EventCatalog>,
    subscribers: Mutex<HashMap<String, Vec<Box<dyn Fn(&TraverseEvent) + Send + Sync>>>>,
}

impl std::fmt::Debug for InProcessBroker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InProcessBroker").finish_non_exhaustive()
    }
}

impl InProcessBroker {
    /// Create a new broker backed by the given catalog.
    #[must_use]
    pub fn new(catalog: Arc<EventCatalog>) -> Self {
        Self {
            catalog,
            subscribers: Mutex::new(HashMap::new()),
        }
    }
}

impl EventBroker for InProcessBroker {
    /// Publish `event` to all registered subscribers.
    ///
    /// # Errors
    ///
    /// - [`EventError::UnregisteredEventType`] if the event type is not in the catalog.
    /// - [`EventError::LifecycleViolation`] if the catalog entry is `Draft` or `Deprecated`.
    fn publish(&self, event: TraverseEvent) -> Result<(), EventError> {
        let entry = self
            .catalog
            .get(&event.event_type)
            .ok_or_else(|| EventError::UnregisteredEventType(event.event_type.clone()))?;

        match entry.lifecycle_status {
            LifecycleStatus::Active => {}
            LifecycleStatus::Deprecated => {
                return Err(EventError::LifecycleViolation(format!(
                    "event type '{}' is Deprecated and cannot be published",
                    event.event_type
                )));
            }
            LifecycleStatus::Draft => {
                return Err(EventError::LifecycleViolation(format!(
                    "event type '{}' is Draft and cannot be published",
                    event.event_type
                )));
            }
        }

        let subs = self
            .subscribers
            .lock()
            .map_err(|_| EventError::LifecycleViolation("subscriber lock poisoned".to_owned()))?;

        if let Some(handlers) = subs.get(&event.event_type) {
            for handler in handlers {
                handler(&event);
            }
        }

        Ok(())
    }

    /// Register `handler` to receive events of `event_type`.
    ///
    /// The event type must already be registered in the catalog.
    ///
    /// # Errors
    ///
    /// Returns [`EventError::UnregisteredEventType`] if the event type is not catalogued.
    fn subscribe(
        &self,
        event_type: &str,
        handler: Box<dyn Fn(&TraverseEvent) + Send + Sync>,
    ) -> Result<(), EventError> {
        // Verify the type exists in the catalog before accepting the subscription.
        if self.catalog.get(event_type).is_none() {
            return Err(EventError::UnregisteredEventType(event_type.to_owned()));
        }

        self.catalog.increment_consumer_count(event_type);

        let mut subs = self
            .subscribers
            .lock()
            .map_err(|_| EventError::LifecycleViolation("subscriber lock poisoned".to_owned()))?;

        subs.entry(event_type.to_owned()).or_default().push(handler);
        Ok(())
    }

    /// Remove all subscribers for `event_type`.
    ///
    /// # Errors
    ///
    /// Returns [`EventError::UnregisteredEventType`] if the event type is not catalogued.
    fn unsubscribe(&self, event_type: &str) -> Result<(), EventError> {
        if self.catalog.get(event_type).is_none() {
            return Err(EventError::UnregisteredEventType(event_type.to_owned()));
        }

        let mut subs = self
            .subscribers
            .lock()
            .map_err(|_| EventError::LifecycleViolation("subscriber lock poisoned".to_owned()))?;

        subs.remove(event_type);
        Ok(())
    }
}

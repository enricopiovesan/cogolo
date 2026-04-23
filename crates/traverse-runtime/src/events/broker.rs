//! Synchronous in-process event broker.
//!
//! Governed by spec 026-event-broker and spec 036-event-subscription-replay.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Arc, Mutex},
    time::Duration,
};

use super::{
    catalog::EventCatalog,
    types::{
        BrokerEvent, EventBroker, EventCursor, EventError, LifecycleStatus, Subscription,
        SubscriptionId, SubscriptionPoll, TraverseEvent,
    },
};

/// Clock abstraction used by the broker for retention pruning.
pub trait BrokerClock: Send + Sync {
    fn now(&self) -> std::time::SystemTime;
}

#[derive(Debug)]
pub struct SystemClock;

impl BrokerClock for SystemClock {
    fn now(&self) -> std::time::SystemTime {
        std::time::SystemTime::now()
    }
}

/// Broker runtime configuration.
#[derive(Debug, Clone)]
pub struct BrokerConfig {
    pub retention_window: Duration,
    pub max_queue_len: usize,
}

impl Default for BrokerConfig {
    fn default() -> Self {
        Self {
            retention_window: Duration::from_secs(5 * 60),
            max_queue_len: 1024,
        }
    }
}

#[derive(Debug, Clone)]
struct BufferedEvent {
    cursor: u64,
    published_at: std::time::SystemTime,
    event: TraverseEvent,
}

#[derive(Debug)]
struct SubscriptionState {
    subscription_id: SubscriptionId,
    event_type: String,
    cursor: u64,
    queue: VecDeque<BufferedEvent>,
}

#[derive(Debug, Default)]
struct BrokerState {
    next_subscription: u64,
    next_cursor: HashMap<String, u64>,
    buffers: HashMap<String, VecDeque<BufferedEvent>>,
    seen_event_ids: HashMap<String, HashSet<String>>,
    subscriptions: HashMap<SubscriptionId, SubscriptionState>,
}

/// Synchronous, in-memory implementation of [`EventBroker`].
///
/// The broker stores a bounded retention buffer per event type and maintains a
/// bounded delivery queue per subscription. Subscribers poll for events using a
/// broker-issued subscription id and a cursor.
pub struct InProcessBroker {
    catalog: Arc<EventCatalog>,
    config: BrokerConfig,
    clock: Arc<dyn BrokerClock>,
    state: Mutex<BrokerState>,
}

impl std::fmt::Debug for InProcessBroker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InProcessBroker").finish_non_exhaustive()
    }
}

impl InProcessBroker {
    /// Create a new broker backed by the given catalog.
    ///
    /// # Errors
    ///
    /// Returns [`EventError::InvalidRetentionWindow`] when the provided configuration is invalid.
    pub fn new(catalog: Arc<EventCatalog>) -> Result<Self, EventError> {
        Self::with_clock(catalog, BrokerConfig::default(), Arc::new(SystemClock))
    }

    /// Create a broker with explicit configuration and clock.
    ///
    /// # Errors
    ///
    /// Returns [`EventError::InvalidRetentionWindow`] when the provided configuration is invalid.
    pub fn with_clock(
        catalog: Arc<EventCatalog>,
        config: BrokerConfig,
        clock: Arc<dyn BrokerClock>,
    ) -> Result<Self, EventError> {
        if config.retention_window == Duration::from_secs(0) {
            return Err(EventError::InvalidRetentionWindow(
                "retention_window must be > 0".to_string(),
            ));
        }
        if config.max_queue_len == 0 {
            return Err(EventError::InvalidRetentionWindow(
                "max_queue_len must be > 0".to_string(),
            ));
        }

        Ok(Self {
            catalog,
            config,
            clock,
            state: Mutex::new(BrokerState::default()),
        })
    }
}

fn parse_cursor(raw: &str) -> Result<u64, EventError> {
    let trimmed = raw.trim();
    if trimmed == "0" {
        return Ok(0);
    }
    trimmed.parse::<u64>().map_err(|_| {
        EventError::InvalidCursor("cursor must be \"0\" or a base-10 unsigned integer".to_string())
    })
}

fn cursor_to_string(cursor: u64) -> EventCursor {
    cursor.to_string()
}

fn enqueue_with_drop_oldest(
    queue: &mut VecDeque<BufferedEvent>,
    max_len: usize,
    item: BufferedEvent,
) {
    while queue.len() >= max_len {
        let _ = queue.pop_front();
    }
    queue.push_back(item);
}

fn prune_expired(
    state: &mut BrokerState,
    event_type: &str,
    retention_window: Duration,
    now: std::time::SystemTime,
) {
    let buffer = state.buffers.entry(event_type.to_string()).or_default();
    let mut oldest_retained_cursor = None;
    while let Some(front) = buffer.front() {
        let age = now
            .duration_since(front.published_at)
            .unwrap_or(Duration::from_secs(0));
        if age <= retention_window {
            oldest_retained_cursor = Some(front.cursor);
            break;
        }

        if let Some(expired) = buffer.pop_front() {
            if let Some(ids) = state.seen_event_ids.get_mut(event_type) {
                let _ = ids.remove(&expired.event.id);
            }
        } else {
            break;
        }
    }

    let Some(oldest_cursor) = oldest_retained_cursor else {
        // Buffer is empty after pruning; nothing to sync.
        return;
    };

    // Sync per-subscription queues so they don't deliver events that are no longer retained.
    for sub in state.subscriptions.values_mut() {
        if sub.event_type != event_type {
            continue;
        }
        while let Some(front) = sub.queue.front() {
            if front.cursor >= oldest_cursor {
                break;
            }
            let _ = sub.queue.pop_front();
        }
        if sub.cursor != 0 && sub.cursor < oldest_cursor.saturating_sub(1) {
            // Cursor is now outside the retention window; keep it as-is so poll can surface cursor_expired.
        }
    }
}

fn validate_from_cursor(
    state: &BrokerState,
    event_type: &str,
    from_cursor: u64,
) -> Result<(), EventError> {
    if from_cursor == 0 {
        return Ok(());
    }

    let last_cursor = state.next_cursor.get(event_type).copied().unwrap_or(0);
    if let Some(buffer) = state.buffers.get(event_type)
        && let Some(front) = buffer.front()
    {
        let oldest_ok = front.cursor.saturating_sub(1);
        if from_cursor < oldest_ok {
            return Err(EventError::CursorExpired {
                event_type: event_type.to_string(),
                oldest_available_cursor: cursor_to_string(oldest_ok),
            });
        }
        return Ok(());
    }

    // If the buffer is empty but we have published events before, treat cursors behind the last
    // observed cursor as expired to avoid silent gaps.
    if last_cursor > 0 && from_cursor < last_cursor {
        return Err(EventError::CursorExpired {
            event_type: event_type.to_string(),
            oldest_available_cursor: cursor_to_string(last_cursor),
        });
    }

    Ok(())
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

        let now = self.clock.now();

        let mut state = self
            .state
            .lock()
            .map_err(|_| EventError::LifecycleViolation("broker lock poisoned".to_owned()))?;

        prune_expired(
            &mut state,
            &event.event_type,
            self.config.retention_window,
            now,
        );

        let seen = state
            .seen_event_ids
            .entry(event.event_type.clone())
            .or_default();
        if seen.contains(&event.id) {
            // Duplicate emissions are silently discarded.
            return Ok(());
        }
        seen.insert(event.id.clone());

        let next = state
            .next_cursor
            .entry(event.event_type.clone())
            .or_insert(0);
        *next = next.saturating_add(1);
        let cursor = *next;

        let buffered = BufferedEvent {
            cursor,
            published_at: now,
            event: event.clone(),
        };

        state
            .buffers
            .entry(event.event_type.clone())
            .or_default()
            .push_back(buffered.clone());

        for sub in state.subscriptions.values_mut() {
            if sub.event_type != event.event_type {
                continue;
            }
            enqueue_with_drop_oldest(&mut sub.queue, self.config.max_queue_len, buffered.clone());
        }

        Ok(())
    }

    /// Create a subscription for `event_type` starting from `from_cursor`.
    ///
    /// The event type must already be registered in the catalog.
    ///
    /// # Errors
    ///
    /// Returns [`EventError::UnregisteredEventType`] if the event type is not catalogued.
    fn subscribe(&self, event_type: &str, from_cursor: &str) -> Result<Subscription, EventError> {
        if self.catalog.get(event_type).is_none() {
            return Err(EventError::UnregisteredEventType(event_type.to_owned()));
        }

        let from_cursor = parse_cursor(from_cursor)?;

        let now = self.clock.now();
        let mut state = self
            .state
            .lock()
            .map_err(|_| EventError::LifecycleViolation("broker lock poisoned".to_owned()))?;

        prune_expired(&mut state, event_type, self.config.retention_window, now);

        validate_from_cursor(&state, event_type, from_cursor)?;

        self.catalog.increment_consumer_count(event_type);

        state.next_subscription = state.next_subscription.saturating_add(1);
        let subscription_id = format!("sub-{}", state.next_subscription);

        let mut queue = VecDeque::new();
        if let Some(buffer) = state.buffers.get(event_type) {
            for item in buffer {
                if from_cursor == 0 || item.cursor > from_cursor {
                    enqueue_with_drop_oldest(&mut queue, self.config.max_queue_len, item.clone());
                }
            }
        }

        state.subscriptions.insert(
            subscription_id.clone(),
            SubscriptionState {
                subscription_id: subscription_id.clone(),
                event_type: event_type.to_string(),
                cursor: from_cursor,
                queue,
            },
        );

        Ok(Subscription {
            subscription_id,
            event_type: event_type.to_string(),
            cursor: cursor_to_string(from_cursor),
        })
    }

    /// Poll a subscription for up to `max_events`.
    ///
    /// # Errors
    ///
    fn poll(
        &self,
        subscription_id: &str,
        max_events: usize,
    ) -> Result<SubscriptionPoll, EventError> {
        let now = self.clock.now();
        let mut state = self
            .state
            .lock()
            .map_err(|_| EventError::LifecycleViolation("broker lock poisoned".to_owned()))?;

        let (event_type, cursor) = match state.subscriptions.get(subscription_id) {
            Some(sub) => (sub.event_type.clone(), sub.cursor),
            None => {
                return Err(EventError::SubscriptionNotFound(
                    subscription_id.to_string(),
                ));
            }
        };

        prune_expired(&mut state, &event_type, self.config.retention_window, now);

        validate_from_cursor(&state, &event_type, cursor)?;

        if max_events == 0 {
            return Ok(SubscriptionPoll {
                subscription_id: subscription_id.to_string(),
                event_type: event_type.clone(),
                cursor: cursor_to_string(cursor),
                events: Vec::new(),
            });
        }

        let Some(subscription) = state.subscriptions.get_mut(subscription_id) else {
            return Err(EventError::SubscriptionNotFound(
                subscription_id.to_string(),
            ));
        };

        let mut out = Vec::new();
        let mut delivered_cursor = subscription.cursor;
        for _ in 0..max_events {
            let Some(item) = subscription.queue.pop_front() else {
                break;
            };
            delivered_cursor = item.cursor;
            out.push(BrokerEvent {
                cursor: cursor_to_string(item.cursor),
                event: item.event,
            });
        }
        subscription.cursor = delivered_cursor;

        Ok(SubscriptionPoll {
            subscription_id: subscription.subscription_id.clone(),
            event_type: subscription.event_type.clone(),
            cursor: cursor_to_string(subscription.cursor),
            events: out,
        })
    }

    /// Cancel a subscription.
    ///
    /// # Errors
    ///
    /// Returns [`EventError::SubscriptionNotFound`] if the subscription id is unknown.
    fn cancel(&self, subscription_id: &str) -> Result<(), EventError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| EventError::LifecycleViolation("broker lock poisoned".to_owned()))?;

        if state.subscriptions.remove(subscription_id).is_none() {
            return Err(EventError::SubscriptionNotFound(
                subscription_id.to_string(),
            ));
        }
        Ok(())
    }
}

//! In-memory store for public and private trace entries.

use super::{private::PrivateTraceEntry, public::PublicTraceEntry};
use std::collections::HashMap;

/// In-memory trace store keyed by trace UUID string.
///
/// Each entry holds a [`PublicTraceEntry`] and an optional [`PrivateTraceEntry`].
#[derive(Debug, Default)]
pub struct TraceStore {
    entries: HashMap<String, (PublicTraceEntry, Option<PrivateTraceEntry>)>,
}

impl TraceStore {
    /// Creates an empty [`TraceStore`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Inserts a public entry and an optional private entry into the store.
    pub fn insert(&mut self, public: PublicTraceEntry, private: Option<PrivateTraceEntry>) {
        self.entries.insert(public.id.clone(), (public, private));
    }

    /// Returns all public entries, optionally filtered to a specific `capability_id`.
    #[must_use]
    pub fn list_public(&self, capability_id: Option<&str>) -> Vec<&PublicTraceEntry> {
        self.entries
            .values()
            .filter(|(pub_entry, _)| capability_id.map_or(true, |id| pub_entry.capability_id == id))
            .map(|(pub_entry, _)| pub_entry)
            .collect()
    }

    /// Looks up a trace by its UUID string.
    ///
    /// Returns `None` when the `trace_id` is not present in the store.
    #[must_use]
    pub fn get(&self, trace_id: &str) -> Option<(&PublicTraceEntry, Option<&PrivateTraceEntry>)> {
        self.entries
            .get(trace_id)
            .map(|(pub_entry, priv_entry)| (pub_entry, priv_entry.as_ref()))
    }
}

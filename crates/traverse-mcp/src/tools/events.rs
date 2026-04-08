//! MCP tool surfaces for the event catalog.
//!
//! Governed by spec 026-event-broker.

use traverse_runtime::events::{EventCatalog, EventCatalogEntry};

/// Return all entries currently registered in the event catalog.
#[must_use]
pub fn list_event_types(catalog: &EventCatalog) -> Vec<EventCatalogEntry> {
    catalog.list()
}

/// Look up a single event type by its reverse-DNS identifier.
#[must_use]
pub fn get_event_type(catalog: &EventCatalog, event_type: &str) -> Option<EventCatalogEntry> {
    catalog.get(event_type)
}

//! MCP execution context holding shared registry and store references.
//!
//! Governed by spec 015-capability-discovery-mcp

use std::sync::Arc;
use traverse_registry::CapabilityRegistry;
use traverse_runtime::{events::EventCatalog, trace::TraceStore};

/// Holds the shared registries and stores injected at MCP startup.
///
/// All six MCP tool functions accept a reference to this struct.
#[derive(Debug)]
pub struct McpContext {
    /// Capability registry — source of truth for capability contracts.
    pub capability_registry: Arc<CapabilityRegistry>,
    /// Event catalog — registry of event types with ECCA governance metadata.
    pub event_catalog: Arc<EventCatalog>,
    /// Trace store — in-memory store for public and private trace entries.
    pub trace_store: Arc<TraceStore>,
}

impl McpContext {
    /// Create a new [`McpContext`] with the supplied shared registries.
    #[must_use]
    pub fn new(
        capability_registry: Arc<CapabilityRegistry>,
        event_catalog: Arc<EventCatalog>,
        trace_store: Arc<TraceStore>,
    ) -> Self {
        Self {
            capability_registry,
            event_catalog,
            trace_store,
        }
    }
}

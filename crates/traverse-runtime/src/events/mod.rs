//! In-process event system for Traverse.
//!
//! Governed by spec 026-event-broker.

pub mod broker;
pub mod catalog;
pub mod types;

pub use broker::InProcessBroker;
pub use catalog::{EventCatalog, EventCatalogEntry};
pub use types::{EventBroker, EventError, LifecycleStatus, TraverseEvent};

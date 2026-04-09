//! MCP-facing trace tools: `list_traces` and `get_trace`.
//!
//! Governed by spec `012-execution-trace-tiered`.

use traverse_runtime::trace::{PrivateTraceEntry, PublicTraceEntry, TraceStore};

/// Request parameters for listing public traces.
#[derive(Debug, Clone)]
pub struct ListTracesRequest {
    /// Optional filter: only return traces for this capability.
    pub capability_id: Option<String>,
}

/// Request parameters for fetching a single trace by ID.
#[derive(Debug, Clone)]
pub struct GetTraceRequest {
    /// The UUID string of the trace to retrieve.
    pub trace_id: String,
    /// Whether to include the private tier in the response.
    pub include_private: bool,
}

/// Response returned by [`get_trace`].
#[derive(Debug, Clone)]
pub struct TraceResponse {
    /// The public tier of the trace.
    pub public: PublicTraceEntry,
    /// The private tier, present only when `include_private` was `true`
    /// and a private entry exists.
    pub private: Option<PrivateTraceEntry>,
}

/// Lists public trace entries, optionally filtered by capability ID.
///
/// Returns a `Vec` of cloned [`PublicTraceEntry`] values from the store.
#[must_use]
pub fn list_traces(store: &TraceStore, request: &ListTracesRequest) -> Vec<PublicTraceEntry> {
    store
        .list_public(request.capability_id.as_deref())
        .into_iter()
        .cloned()
        .collect()
}

/// Fetches a single trace by its UUID string.
///
/// Returns `None` when the `trace_id` is not found in the store.
/// When `include_private` is `false` the `private` field of the
/// returned [`TraceResponse`] will always be `None`.
#[must_use]
pub fn get_trace(store: &TraceStore, request: &GetTraceRequest) -> Option<TraceResponse> {
    store.get(&request.trace_id).map(|(public, private)| {
        let private = if request.include_private {
            private.cloned()
        } else {
            None
        };
        TraceResponse {
            public: public.clone(),
            private,
        }
    })
}

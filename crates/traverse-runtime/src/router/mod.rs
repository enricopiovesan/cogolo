//! Governed by spec 016-runtime-placement-router
//!
//! `PlacementRouter` is the single public entry point for all capability execution
//! in `traverse-runtime`.  It wires together:
//!
//! 1. Placement evaluation ([`PlacementConstraintEvaluator`])
//! 2. Executor selection ([`CapabilityExecutorRegistry`])
//! 3. Execution ([`CapabilityExecutor`])
//! 4. Trace recording ([`TraceStore`])
//! 5. Conditional event publishing ([`EventBroker`])

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Instant,
};

use chrono::Utc;
use serde_json::Value;
use traverse_contracts::{CapabilityContract, ServiceType, ViolationRecord};

use crate::{
    events::types::{EventBroker, TraverseEvent},
    executor::{ArtifactType, CapabilityExecutor, ExecutorCapability},
    placement::{
        PlacementConstraintEvaluator, PlacementDecision, PlacementError, PlacementRequest,
        RuntimeSnapshot,
    },
    trace::{
        DurableTraceConfig, PrivateTraceEntry, PublicTraceEntry, TraceOutcome, TraceStore,
        new_trace_id_and_time,
    },
};

use traverse_contracts::ExecutionTarget;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Maps [`ArtifactType`] to the appropriate [`CapabilityExecutor`] implementation.
pub type CapabilityExecutorRegistry = HashMap<ArtifactType, Box<dyn CapabilityExecutor>>;

/// Input to [`PlacementRouter::execute`].
pub struct RouterRequest {
    /// Unique capability identifier.
    pub capability_id: String,
    /// How the capability is packaged.
    pub artifact_type: ArtifactType,
    /// The validated contract for this capability (used for placement evaluation).
    pub contract: CapabilityContract,
    /// Optional caller hint for target placement.
    pub target_hint: Option<ExecutionTarget>,
    /// Current runtime load snapshot used by the placement evaluator.
    pub runtime_snapshot: RuntimeSnapshot,
    /// JSON input payload for the capability.
    pub input: Value,
    /// Resolved capability descriptor passed to the executor.
    pub executor_capability: ExecutorCapability,
    /// When set, used as the public/private [`TraceStore`] id instead of minting a new UUID.
    pub trace_id_override: Option<String>,
}

/// Errors returned by [`PlacementRouter::execute`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouterError {
    /// The placement constraint evaluator rejected the request.
    PlacementFailed(PlacementError),
    /// No executor is registered for the requested [`ArtifactType`].
    ExecutorNotFound(String),
    /// The selected executor returned an error.
    ExecutionFailed(String),
    /// Execution violated a governed contract (aggregate violations).
    ContractViolation(Vec<ViolationRecord>),
    /// The trace store lock was poisoned.
    TraceLockPoisoned,
    /// The trace could not be durably written and this router is configured
    /// to fail closed on that condition (spec 079 FR-002).
    DurableTraceWriteFailed(String),
}

impl std::fmt::Display for RouterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PlacementFailed(e) => write!(f, "placement failed: {e:?}"),
            Self::ExecutorNotFound(t) => write!(f, "no executor registered for artifact type: {t}"),
            Self::ExecutionFailed(msg) => write!(f, "execution failed: {msg}"),
            Self::ContractViolation(violations) => {
                write!(f, "contract violation: {} violation(s)", violations.len())
            }
            Self::TraceLockPoisoned => write!(f, "trace store lock is poisoned"),
            Self::DurableTraceWriteFailed(msg) => {
                write!(f, "durable trace write failed: {msg}")
            }
        }
    }
}

impl std::error::Error for RouterError {}

/// Result of a successful [`PlacementRouter::execute`] call.
#[derive(Debug)]
pub struct RouterResponse {
    /// The JSON output produced by the executor.
    pub output: Value,
    /// Events the executor emitted and validated during this call (spec
    /// 098-capability-event-host-abi), already published to `EventBroker`
    /// by Step 5 for `Subscribable` capabilities.
    pub emitted_events: Vec<TraverseEvent>,
    /// The public trace entry written to the store.
    pub trace_id: String,
    /// The placement decision that was made.
    pub placement_decision: PlacementDecision,
}

// ---------------------------------------------------------------------------
// PlacementRouter
// ---------------------------------------------------------------------------

/// Single orchestrating entry point for all capability execution in Traverse.
///
/// Wires together placement evaluation → executor selection → execution →
/// trace recording → event publishing.
pub struct PlacementRouter {
    evaluator: PlacementConstraintEvaluator,
    executor_registry: CapabilityExecutorRegistry,
    trace_store: Arc<Mutex<TraceStore>>,
    event_broker: Arc<dyn EventBroker>,
    durable_trace: Option<DurableTraceConfig>,
}

impl PlacementRouter {
    /// Construct a new [`PlacementRouter`] from injected dependencies. Traces
    /// are recorded to `trace_store` only (in-memory, process-local) unless
    /// [`Self::with_durable_trace`] is also called.
    #[must_use]
    pub fn new(
        evaluator: PlacementConstraintEvaluator,
        executor_registry: CapabilityExecutorRegistry,
        trace_store: Arc<Mutex<TraceStore>>,
        event_broker: Arc<dyn EventBroker>,
    ) -> Self {
        Self {
            evaluator,
            executor_registry,
            trace_store,
            event_broker,
            durable_trace: None,
        }
    }

    /// Additionally persist every recorded trace through a durable trace
    /// journal (spec `079-durable-trace-journal`). Without this, traces are
    /// recorded to `trace_store` only and are lost on restart.
    #[must_use]
    pub fn with_durable_trace(mut self, durable_trace: DurableTraceConfig) -> Self {
        self.durable_trace = Some(durable_trace);
        self
    }

    /// Execute a capability end-to-end.
    ///
    /// Steps:
    /// 1. Evaluate placement constraints — returns [`RouterError::PlacementFailed`] with no trace on failure.
    /// 2. Select executor by `artifact_type`.
    /// 3. Run the executor.
    /// 4. Write public + private trace entries to the store.
    /// 5. If `service_type == Subscribable`, publish emitted events.
    ///
    /// # Errors
    ///
    /// Returns [`RouterError`] when any step cannot complete.
    pub fn execute(&self, request: RouterRequest) -> Result<RouterResponse, RouterError> {
        let executor = self
            .executor_registry
            .get(&request.artifact_type)
            .ok_or_else(|| RouterError::ExecutorNotFound(format!("{:?}", request.artifact_type)))?;
        self.execute_with_executor(request, executor.as_ref())
    }

    /// Execute a capability with an explicitly provided executor.
    ///
    /// Used by the live `Runtime::execute` path to bridge a host
    /// `LocalExecutor` without requiring a `'static` registry entry.
    ///
    /// # Errors
    ///
    /// Returns [`RouterError`] when any step cannot complete.
    pub fn execute_with_executor(
        &self,
        request: RouterRequest,
        executor: &dyn CapabilityExecutor,
    ) -> Result<RouterResponse, RouterError> {
        // --- Step 1: Placement evaluation ---
        let placement_req = PlacementRequest {
            capability_id: request.capability_id.clone(),
            target_hint: request.target_hint,
            runtime_snapshot: request.runtime_snapshot,
        };

        let decision = self
            .evaluator
            .evaluate(&placement_req, &request.contract)
            .map_err(RouterError::PlacementFailed)?;

        let placement_target_str = format!("{:?}", decision.target);

        // --- Step 3: Execute capability ---
        // Events emitted via `traverse_host::emit_event` (spec
        // 098-capability-event-host-abi) are already validated
        // synchronously, at call time, against `request.contract.emits` and
        // `service_type` by the host function itself (FR-002/FR-003) — no
        // post-hoc enforcement gate is needed here.
        let start = Instant::now();
        let exec_result = executor.execute(&request.executor_capability, &request.input);
        let duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);

        let (output, emitted_events, outcome) = match exec_result {
            Ok(exec_output) => (
                exec_output.value,
                exec_output.emitted_events,
                TraceOutcome::Success,
            ),
            Err(e) => return Err(RouterError::ExecutionFailed(format!("{e}"))),
        };

        // --- Step 4: Write trace ---
        let (trace_id, time) = match request.trace_id_override {
            Some(override_id) => (override_id, Utc::now().to_rfc3339()),
            None => new_trace_id_and_time(),
        };

        let public_entry = PublicTraceEntry::new(
            trace_id.clone(),
            request.capability_id.clone(),
            placement_target_str,
            outcome,
            duration_ms,
            time,
        );

        let input_str = serde_json::to_string(&request.input).unwrap_or_default();
        let output_str = serde_json::to_string(&output).unwrap_or_default();
        let private_entry =
            PrivateTraceEntry::new(trace_id.clone(), &input_str, &output_str, duration_ms);

        // Durable write gates the in-memory record when the caller is
        // configured to fail closed (spec 079 FR-002): a trace that could
        // not be durably written is not silently kept in-memory-only for an
        // auditable execution.
        if let Some(durable) = &self.durable_trace
            && let Err(error) = durable.sink.record(&public_entry, Some(&private_entry))
            && durable.fail_closed
        {
            return Err(RouterError::DurableTraceWriteFailed(error.to_string()));
        }

        {
            let mut store = self
                .trace_store
                .lock()
                .map_err(|_| RouterError::TraceLockPoisoned)?;
            store.insert(public_entry, Some(private_entry));
        }

        // --- Step 5: Publish events for Subscribable capabilities ---
        let published_events = if request.contract.service_type == ServiceType::Subscribable {
            for event in &emitted_events {
                // Best-effort: publish errors are logged but do not fail the response.
                let _ = self.event_broker.publish(event.clone());
            }
            emitted_events
        } else {
            Vec::new()
        };

        Ok(RouterResponse {
            output,
            emitted_events: published_events,
            trace_id,
            placement_decision: decision,
        })
    }
}

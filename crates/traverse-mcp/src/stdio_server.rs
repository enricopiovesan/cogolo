//! Deterministic stdio server foundation for Traverse MCP.

use crate::{TraverseMcp, youaskm3_mcp_consumption_validation_path};
use serde::Deserialize;
use serde_json::{Value, json};
use std::fmt;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use traverse_contracts::Lifecycle;
use traverse_registry::{
    BinaryFormat as RegistryBinaryFormat, BinaryReference, CapabilityRegistration,
    CapabilityRegistry, ComposabilityMetadata, CompositionKind, CompositionPattern, EventRegistry,
    ImplementationKind, RegistryBundle, RegistryProvenance, SourceKind, SourceReference,
    WorkflowReference, WorkflowRegistration, WorkflowRegistry, load_registry_bundle,
};
use traverse_runtime::{LocalExecutor, Runtime, RuntimeRequest, parse_runtime_request};

const SERVER_NAME: &str = "traverse-mcp";
const HOST_MODE: &str = "stdio";
const GOVERNING_SPEC: &str = "022-mcp-wasm-server";
const PUBLIC_SURFACE_ID: &str = "traverse.mcp.stdio-server";
const SUPPORTING_COMMANDS: &[&str] = &[
    "describe_server",
    "list_entrypoints",
    "describe_entrypoint",
    "validate_entrypoint",
    "execute_entrypoint",
    "shutdown",
];
const FUTURE_OPERATIONS: &[&str] = &[];

#[derive(Debug, Deserialize)]
struct StdioCommandEnvelope {
    command: String,
    #[serde(default)]
    entrypoint_kind: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    request_path: Option<String>,
}

#[derive(Debug)]
pub struct McpDiscoveryCatalog {
    bundle: RegistryBundle,
}

#[derive(Debug)]
struct CanonicalExecutionContext {
    capability_registry: CapabilityRegistry,
    workflow_registry: WorkflowRegistry,
}

impl McpDiscoveryCatalog {
    /// Load the canonical discovery catalog used by the stdio server.
    ///
    /// # Errors
    ///
    /// Returns `catalog_load_failed` when the expedition registry bundle cannot be loaded.
    pub fn load_canonical() -> Result<Self, StdioServerFailure> {
        let manifest_path = canonical_expedition_bundle_path();
        let bundle = load_registry_bundle(&manifest_path).map_err(|failure| {
            StdioServerFailure::new(
                "catalog_load_failed",
                format!(
                    "Failed to load expedition registry bundle {}: {}",
                    manifest_path.display(),
                    failure.errors[0].message
                ),
            )
        })?;

        Ok(Self { bundle })
    }

    #[must_use]
    pub fn capability_count(&self) -> usize {
        self.bundle.capabilities.len()
    }

    #[must_use]
    pub fn workflow_count(&self) -> usize {
        self.bundle.workflows.len()
    }

    #[must_use]
    pub fn event_count(&self) -> usize {
        self.bundle.events.len()
    }
}

impl CanonicalExecutionContext {
    pub fn load_canonical() -> Result<Self, StdioServerFailure> {
        let manifest_path = canonical_expedition_bundle_path();
        let bundle = load_registry_bundle(&manifest_path).map_err(|failure| {
            StdioServerFailure::new(
                "catalog_load_failed",
                format!(
                    "Failed to load expedition registry bundle {}: {}",
                    manifest_path.display(),
                    failure.errors[0].message,
                ),
            )
        })?;

        let mut capability_registry = CapabilityRegistry::new();
        let mut workflow_registry = WorkflowRegistry::new();

        for capability in &bundle.capabilities {
            let request = build_capability_registration(&bundle, capability)?;
            capability_registry.register(request).map_err(|failure| {
                StdioServerFailure::new(
                    "registry_registration_failed",
                    format!(
                        "Failed to register capability {}@{} for stdio execution: {}",
                        capability.contract.id,
                        capability.contract.version,
                        failure.errors[0].message,
                    ),
                )
            })?;
        }

        for workflow in &bundle.workflows {
            workflow_registry
                .register(
                    &capability_registry,
                    WorkflowRegistration {
                        scope: bundle.scope,
                        definition: workflow.definition.clone(),
                        workflow_path: workflow.path.display().to_string(),
                        registered_at: bundle_registered_at(&bundle),
                        validator_version: env!("CARGO_PKG_VERSION").to_string(),
                    },
                )
                .map_err(|failure| {
                    StdioServerFailure::new(
                        "registry_registration_failed",
                        format!(
                            "Failed to register workflow {}@{} for stdio execution: {}",
                            workflow.definition.id,
                            workflow.definition.version,
                            failure.errors[0].message,
                        ),
                    )
                })?;
        }

        Ok(Self {
            capability_registry,
            workflow_registry,
        })
    }
}

#[derive(Debug)]
pub struct TraverseMcpStdioServer<'a, E> {
    _mcp: &'a TraverseMcp<'a, E>,
    catalog: &'a McpDiscoveryCatalog,
}

impl<'a, E> TraverseMcpStdioServer<'a, E>
where
    E: LocalExecutor,
{
    #[must_use]
    pub fn new(mcp: &'a TraverseMcp<'a, E>, catalog: &'a McpDiscoveryCatalog) -> Self {
        Self { _mcp: mcp, catalog }
    }

    #[must_use]
    pub fn startup_envelope(&self) -> Value {
        json!({
            "kind": "mcp_stdio_server_startup",
            "server_name": SERVER_NAME,
            "host_mode": HOST_MODE,
            "governing_spec": GOVERNING_SPEC,
            "status": "ready",
            "supported_commands": SUPPORTING_COMMANDS,
            "future_operations": FUTURE_OPERATIONS,
            "public_surface_id": PUBLIC_SURFACE_ID,
        })
    }

    #[must_use]
    pub fn describe_envelope(&self) -> Value {
        let validation_path = youaskm3_mcp_consumption_validation_path();

        json!({
            "kind": "mcp_stdio_server_description",
            "server_name": SERVER_NAME,
            "host_mode": HOST_MODE,
            "governing_spec": GOVERNING_SPEC,
            "runtime_authority": "Traverse runtime authority",
            "public_surface_id": PUBLIC_SURFACE_ID,
            "supported_commands": SUPPORTING_COMMANDS,
            "future_operations": FUTURE_OPERATIONS,
            "governed_surface_counts": {
                "capabilities": self.catalog.capability_count(),
                "events": self.catalog.event_count(),
                "workflows": self.catalog.workflow_count(),
            },
            "downstream_validation_path": {
                "consumer_name": validation_path.consumer_name,
                "validated_flow_id": validation_path.validated_flow_id,
                "public_surface_id": validation_path.public_surface_id,
                "governing_specs": validation_path.governing_specs,
            },
        })
    }

    #[must_use]
    pub fn list_entrypoints_envelope(&self) -> Value {
        let capability_entries = self
            .catalog
            .bundle
            .capabilities
            .iter()
            .map(capability_entrypoint_summary)
            .collect::<Vec<_>>();
        let workflow_entries = self
            .catalog
            .bundle
            .workflows
            .iter()
            .map(workflow_entrypoint_summary)
            .collect::<Vec<_>>();

        json!({
            "kind": "mcp_stdio_server_entrypoint_list",
            "server_name": SERVER_NAME,
            "host_mode": HOST_MODE,
            "governing_spec": GOVERNING_SPEC,
            "entrypoints": {
                "capabilities": capability_entries,
                "workflows": workflow_entries,
            },
        })
    }

    /// Describe a single discovered entrypoint by id and version.
    ///
    /// # Errors
    ///
    /// Returns `not_found` when the requested capability or workflow is absent.
    pub fn describe_entrypoint_envelope(
        &self,
        entrypoint_kind: &str,
        id: &str,
        version: &str,
    ) -> Result<Value, StdioServerFailure> {
        match entrypoint_kind {
            "capability" => self
                .catalog
                .bundle
                .capabilities
                .iter()
                .find(|artifact| artifact.contract.id == id && artifact.contract.version == version)
                .map(|artifact| {
                    json!({
                        "kind": "mcp_stdio_server_entrypoint_description",
                        "server_name": SERVER_NAME,
                        "host_mode": HOST_MODE,
                        "governing_spec": GOVERNING_SPEC,
                        "entrypoint": capability_entrypoint_detail(artifact),
                    })
                })
                .ok_or_else(|| not_found("capability entrypoint", id, version)),
            "workflow" => self
                .catalog
                .bundle
                .workflows
                .iter()
                .find(|artifact| {
                    artifact.definition.id == id && artifact.definition.version == version
                })
                .map(|artifact| {
                    json!({
                        "kind": "mcp_stdio_server_entrypoint_description",
                        "server_name": SERVER_NAME,
                        "host_mode": HOST_MODE,
                        "governing_spec": GOVERNING_SPEC,
                        "entrypoint": workflow_entrypoint_detail(artifact),
                    })
                })
                .ok_or_else(|| not_found("workflow entrypoint", id, version)),
            other => Err(StdioServerFailure::new(
                "invalid_request",
                format!("Unsupported entrypoint_kind: {other}"),
            )),
        }
    }

    #[must_use]
    fn validate_entrypoint_envelope(
        &self,
        command: &StdioCommandEnvelope,
    ) -> Result<Value, StdioServerFailure> {
        let entrypoint_kind = command.entrypoint_kind.as_deref().ok_or_else(|| {
            StdioServerFailure::new(
                "invalid_request",
                "validate_entrypoint requires entrypoint_kind.",
            )
        })?;
        let id = command.id.as_deref().ok_or_else(|| {
            StdioServerFailure::new("invalid_request", "validate_entrypoint requires id.")
        })?;
        let version = command.version.as_deref().ok_or_else(|| {
            StdioServerFailure::new("invalid_request", "validate_entrypoint requires version.")
        })?;
        let request_path = command.request_path.as_deref().ok_or_else(|| {
            StdioServerFailure::new(
                "invalid_request",
                "validate_entrypoint requires request_path.",
            )
        })?;
        let runtime_request = load_runtime_request(request_path)?;
        self.validate_runtime_request(entrypoint_kind, id, version, &runtime_request)?;

        Ok(json!({
            "kind": "mcp_stdio_server_entrypoint_validation",
            "server_name": SERVER_NAME,
            "host_mode": HOST_MODE,
            "governing_spec": GOVERNING_SPEC,
            "status": "valid",
            "request_path": request_path,
            "entrypoint": self.describe_entrypoint_envelope(entrypoint_kind, id, version)?,
            "request": runtime_request_summary(&runtime_request),
        }))
    }

    #[must_use]
    fn execute_entrypoint_envelope(
        &self,
        command: &StdioCommandEnvelope,
    ) -> Result<Value, StdioServerFailure> {
        let entrypoint_kind = command.entrypoint_kind.as_deref().ok_or_else(|| {
            StdioServerFailure::new(
                "invalid_request",
                "execute_entrypoint requires entrypoint_kind.",
            )
        })?;
        let id = command.id.as_deref().ok_or_else(|| {
            StdioServerFailure::new("invalid_request", "execute_entrypoint requires id.")
        })?;
        let version = command.version.as_deref().ok_or_else(|| {
            StdioServerFailure::new("invalid_request", "execute_entrypoint requires version.")
        })?;
        let request_path = command.request_path.as_deref().ok_or_else(|| {
            StdioServerFailure::new(
                "invalid_request",
                "execute_entrypoint requires request_path.",
            )
        })?;
        let runtime_request = load_runtime_request(request_path)?;
        self.validate_runtime_request(entrypoint_kind, id, version, &runtime_request)?;
        let response = self
            ._mcp
            .execute(runtime_request)
            .map_err(|error| StdioServerFailure::new("execution_failed", format!("{error:?}")))?;
        let result = response.result;
        let trace = response.trace;
        let observation_messages = response
            .observation_messages
            .into_iter()
            .map(|message| format!("{message:?}"))
            .collect::<Vec<_>>();

        Ok(json!({
            "kind": "mcp_stdio_server_entrypoint_execution",
            "server_name": SERVER_NAME,
            "host_mode": HOST_MODE,
            "governing_spec": GOVERNING_SPEC,
            "status": "completed",
            "request_path": request_path,
            "entrypoint": self.describe_entrypoint_envelope(entrypoint_kind, id, version)?,
            "request_id": result.request_id,
            "execution_id": result.execution_id,
            "result": result,
            "trace": trace,
            "observation_messages": observation_messages,
        }))
    }

    fn validate_runtime_request(
        &self,
        entrypoint_kind: &str,
        id: &str,
        version: &str,
        runtime_request: &RuntimeRequest,
    ) -> Result<(), StdioServerFailure> {
        match entrypoint_kind {
            "capability" => {
                let Some(capability_id) = runtime_request.intent.capability_id.as_deref() else {
                    return Err(StdioServerFailure::new(
                        "invalid_request",
                        "runtime request must include intent.capability_id for capability entrypoints.",
                    ));
                };
                let Some(capability_version) = runtime_request.intent.capability_version.as_deref()
                else {
                    return Err(StdioServerFailure::new(
                        "invalid_request",
                        "runtime request must include intent.capability_version for capability entrypoints.",
                    ));
                };

                if capability_id != id || capability_version != version {
                    return Err(StdioServerFailure::new(
                        "invalid_request",
                        format!(
                            "runtime request target {}@{} does not match capability entrypoint {}@{}",
                            capability_id, capability_version, id, version
                        ),
                    ));
                }
            }
            "workflow" => {
                let Some(_capability_id) = runtime_request.intent.capability_id.as_deref() else {
                    return Err(StdioServerFailure::new(
                        "invalid_request",
                        "runtime request must include intent.capability_id for workflow entrypoints.",
                    ));
                };
                let Some(_capability_version) =
                    runtime_request.intent.capability_version.as_deref()
                else {
                    return Err(StdioServerFailure::new(
                        "invalid_request",
                        "runtime request must include intent.capability_version for workflow entrypoints.",
                    ));
                };
                let Some(workflow) = self.catalog.bundle.workflows.iter().find(|artifact| {
                    artifact.definition.id == id && artifact.definition.version == version
                }) else {
                    return Err(not_found("workflow entrypoint", id, version));
                };
                let _ = workflow;
            }
            other => {
                return Err(StdioServerFailure::new(
                    "invalid_request",
                    format!("Unsupported entrypoint_kind: {other}"),
                ));
            }
        }

        Ok(())
    }

    #[must_use]
    pub fn shutdown_envelope(&self, reason: &str) -> Value {
        json!({
            "kind": "mcp_stdio_server_shutdown",
            "server_name": SERVER_NAME,
            "host_mode": HOST_MODE,
            "governing_spec": GOVERNING_SPEC,
            "status": "complete",
            "reason": reason,
        })
    }

    /// Run the stdio server loop.
    ///
    /// # Errors
    ///
    /// Returns `startup_failed` when deterministic startup failure simulation is enabled.
    /// Returns `io_error` when writing startup or message envelopes to stdio fails, or when
    /// reading input from stdin fails.
    #[allow(clippy::too_many_lines)]
    pub fn run_stdio<R, W, EWrite>(
        &self,
        input: R,
        stdout: &mut W,
        stderr: &mut EWrite,
        simulate_startup_failure: bool,
    ) -> Result<(), StdioServerFailure>
    where
        R: BufRead,
        W: Write,
        EWrite: Write,
    {
        if simulate_startup_failure {
            let failure = StdioServerFailure::new(
                "startup_failed",
                "Simulated startup failure for deterministic validation.",
            );
            write_json_line(stderr, &failure.envelope()).map_err(|error| {
                StdioServerFailure::new(
                    "io_error",
                    format!("Failed to write startup failure envelope: {error}"),
                )
            })?;
            return Err(failure);
        }

        write_json_line(stdout, &self.startup_envelope()).map_err(|error| {
            StdioServerFailure::new(
                "io_error",
                format!("Failed to write startup envelope: {error}"),
            )
        })?;

        for line in input.lines() {
            let line = line.map_err(|error| {
                StdioServerFailure::new(
                    "io_error",
                    format!("Failed to read stdio command line: {error}"),
                )
            })?;

            if line.trim().is_empty() {
                continue;
            }

            let command = match parse_command(&line) {
                Ok(command) => command,
                Err(failure) => {
                    let _ = write_json_line(stderr, &failure.envelope());
                    return Err(failure);
                }
            };
            match command.command.as_str() {
                "describe_server" | "describe" => {
                    write_json_line(stdout, &self.describe_envelope()).map_err(|error| {
                        StdioServerFailure::new(
                            "io_error",
                            format!("Failed to write server description envelope: {error}"),
                        )
                    })?;
                }
                "list_entrypoints" | "list" => {
                    write_json_line(stdout, &self.list_entrypoints_envelope()).map_err(
                        |error| {
                            StdioServerFailure::new(
                                "io_error",
                                format!("Failed to write entrypoint list envelope: {error}"),
                            )
                        },
                    )?;
                }
                "describe_entrypoint" => {
                    let Some(entrypoint_kind) = command.entrypoint_kind.as_deref() else {
                        let failure = StdioServerFailure::new(
                            "invalid_request",
                            "describe_entrypoint requires entrypoint_kind.",
                        );
                        let _ = write_json_line(stderr, &failure.envelope());
                        return Err(failure);
                    };
                    let Some(id) = command.id.as_deref() else {
                        let failure = StdioServerFailure::new(
                            "invalid_request",
                            "describe_entrypoint requires id.",
                        );
                        let _ = write_json_line(stderr, &failure.envelope());
                        return Err(failure);
                    };
                    let Some(version) = command.version.as_deref() else {
                        let failure = StdioServerFailure::new(
                            "invalid_request",
                            "describe_entrypoint requires version.",
                        );
                        let _ = write_json_line(stderr, &failure.envelope());
                        return Err(failure);
                    };

                    let envelope =
                        self.describe_entrypoint_envelope(entrypoint_kind, id, version)?;
                    write_json_line(stdout, &envelope).map_err(|error| {
                        StdioServerFailure::new(
                            "io_error",
                            format!("Failed to write entrypoint description envelope: {error}"),
                        )
                    })?;
                }
                "validate_entrypoint" => {
                    let envelope = match self.validate_entrypoint_envelope(&command) {
                        Ok(envelope) => envelope,
                        Err(failure) => {
                            let _ = write_json_line(stderr, &failure.envelope());
                            return Err(failure);
                        }
                    };
                    write_json_line(stdout, &envelope).map_err(|error| {
                        StdioServerFailure::new(
                            "io_error",
                            format!("Failed to write entrypoint validation envelope: {error}"),
                        )
                    })?;
                }
                "execute_entrypoint" => {
                    let envelope = match self.execute_entrypoint_envelope(&command) {
                        Ok(envelope) => envelope,
                        Err(failure) => {
                            let _ = write_json_line(stderr, &failure.envelope());
                            return Err(failure);
                        }
                    };
                    write_json_line(stdout, &envelope).map_err(|error| {
                        StdioServerFailure::new(
                            "io_error",
                            format!("Failed to write entrypoint execution envelope: {error}"),
                        )
                    })?;
                }
                "shutdown" => {
                    write_json_line(stdout, &self.shutdown_envelope("shutdown_command")).map_err(
                        |error| {
                            StdioServerFailure::new(
                                "io_error",
                                format!("Failed to write shutdown envelope: {error}"),
                            )
                        },
                    )?;
                    return Ok(());
                }
                other => {
                    let failure = StdioServerFailure::new(
                        "unsupported_command",
                        format!("Unsupported stdio command: {other}"),
                    );
                    let _ = write_json_line(stderr, &failure.envelope());
                    return Err(failure);
                }
            }
        }

        write_json_line(stdout, &self.shutdown_envelope("stdin_closed")).map_err(|error| {
            StdioServerFailure::new(
                "io_error",
                format!("Failed to write shutdown envelope: {error}"),
            )
        })?;
        Ok(())
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct ExpeditionStdioExecutor;

impl LocalExecutor for ExpeditionStdioExecutor {
    fn execute(
        &self,
        capability: &traverse_registry::ResolvedCapability,
        input: &Value,
    ) -> Result<Value, traverse_runtime::LocalExecutionFailure> {
        match capability.contract.id.as_str() {
            "expedition.planning.capture-expedition-objective" => {
                execute_capture_expedition_objective(input)
            }
            "expedition.planning.interpret-expedition-intent" => {
                execute_interpret_expedition_intent(input)
            }
            "expedition.planning.assess-conditions-summary" => {
                execute_assess_conditions_summary(input)
            }
            "expedition.planning.validate-team-readiness" => execute_validate_team_readiness(input),
            "expedition.planning.assemble-expedition-plan" => {
                execute_assemble_expedition_plan(input)
            }
            other => Err(executor_failure(&format!(
                "unsupported expedition capability for stdio execution: {other}"
            ))),
        }
    }
}

fn load_runtime_request(request_path: &str) -> Result<RuntimeRequest, StdioServerFailure> {
    let path = resolve_relative_path(request_path);
    let contents = read_text_file(&path, "runtime request")?;
    parse_runtime_request(&contents).map_err(|error| {
        StdioServerFailure::new(
            "invalid_request",
            format!(
                "failed to parse runtime request {}: {}",
                path.display(),
                error.message
            ),
        )
    })
}

fn read_text_file(path: &PathBuf, artifact_kind: &str) -> Result<String, StdioServerFailure> {
    fs::read_to_string(path).map_err(|error| {
        StdioServerFailure::new(
            "io_error",
            format!("failed to read {artifact_kind} {}: {error}", path.display()),
        )
    })
}

fn resolve_relative_path(relative_path: &str) -> PathBuf {
    let candidate = PathBuf::from(relative_path);
    if candidate.is_absolute() {
        candidate
    } else {
        repo_root().join(candidate)
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn runtime_request_summary(runtime_request: &RuntimeRequest) -> Value {
    json!({
        "kind": runtime_request.kind,
        "schema_version": runtime_request.schema_version,
        "request_id": runtime_request.request_id,
        "governing_spec": runtime_request.governing_spec,
        "intent": {
            "capability_id": runtime_request.intent.capability_id,
            "capability_version": runtime_request.intent.capability_version,
            "intent_key": runtime_request.intent.intent_key,
        },
        "lookup": {
            "scope": runtime_request.lookup.scope,
            "allow_ambiguity": runtime_request.lookup.allow_ambiguity,
        },
        "requested_target": format!("{:?}", runtime_request.context.requested_target).to_lowercase(),
        "correlation_id": runtime_request.context.correlation_id,
        "caller": runtime_request.context.caller,
    })
}

fn execute_capture_expedition_objective(
    input: &Value,
) -> Result<Value, traverse_runtime::LocalExecutionFailure> {
    let map = input_object(input)?;
    let destination = required_value(map, "destination")?;
    let target_window = required_value(map, "target_window")?;
    let preferences = required_value(map, "preferences")?;
    let notes = required_value(map, "notes")?;
    let objective_id = format!("objective-{}", slug(required_string(map, "destination")?));
    let objective = serde_json::json!({
        "objective_id": objective_id,
        "destination": destination.clone(),
        "target_window": target_window.clone(),
        "preferences": preferences.clone(),
        "notes": notes.clone()
    });

    Ok(serde_json::json!({
        "objective_id": objective_id,
        "destination": destination.clone(),
        "target_window": target_window.clone(),
        "preferences": preferences.clone(),
        "notes": notes.clone(),
        "objective": objective,
        "emitted_events": [event_ref("expedition.planning.expedition-objective-captured")]
    }))
}

fn execute_interpret_expedition_intent(
    input: &Value,
) -> Result<Value, traverse_runtime::LocalExecutionFailure> {
    let map = input_object(input)?;
    let objective = required_object(map, "objective")?;
    let objective_id = required_string(objective, "objective_id")?;
    let preferences = required_object(objective, "preferences")?;
    let style = required_string(preferences, "style")?;
    let priority = required_string(preferences, "priority")?;
    let planning_intent = required_string(map, "planning_intent")?;
    let interpreted_intent = serde_json::json!({
        "intent_id": format!("intent-{objective_id}"),
        "objective_id": objective_id,
        "route_preferences": [style, priority],
        "constraints": [format!("priority:{priority}")],
        "assumptions": [planning_intent],
        "confidence": 0.87
    });

    Ok(serde_json::json!({
        "intent_id": format!("intent-{objective_id}"),
        "objective_id": objective_id,
        "route_preferences": [style, priority],
        "constraints": [format!("priority:{priority}")],
        "assumptions": [planning_intent],
        "confidence": 0.87,
        "interpreted_intent": interpreted_intent,
        "emitted_events": [event_ref("expedition.planning.expedition-intent-interpreted")]
    }))
}

fn execute_assess_conditions_summary(
    input: &Value,
) -> Result<Value, traverse_runtime::LocalExecutionFailure> {
    let map = input_object(input)?;
    let objective = required_object(map, "objective")?;
    let objective_id = required_string(objective, "objective_id")?;
    let destination = required_string(objective, "destination")?;
    let interpreted = required_object(map, "interpreted_intent")?;
    let route_preferences = required_string_array(interpreted, "route_preferences")?;
    let conditions_summary = serde_json::json!({
        "conditions_summary_id": format!("conditions-{objective_id}"),
        "objective_id": objective_id,
        "overall_rating": "watchful",
        "key_findings": [format!("stable morning window for {destination}"), format!("preferred style: {}", route_preferences.first().cloned().unwrap_or_else(|| "conservative".to_string()))],
        "blocking_concerns": []
    });

    Ok(serde_json::json!({
        "conditions_summary_id": format!("conditions-{objective_id}"),
        "objective_id": objective_id,
        "overall_rating": "watchful",
        "key_findings": [format!("stable morning window for {destination}"), format!("preferred style: {}", route_preferences.first().cloned().unwrap_or_else(|| "conservative".to_string()))],
        "blocking_concerns": [],
        "conditions_summary": conditions_summary,
        "emitted_events": [event_ref("expedition.planning.conditions-summary-assessed")]
    }))
}

fn execute_validate_team_readiness(
    input: &Value,
) -> Result<Value, traverse_runtime::LocalExecutionFailure> {
    let map = input_object(input)?;
    let objective = required_object(map, "objective")?;
    let objective_id = required_string(objective, "objective_id")?;
    let team_profile = required_object(map, "team_profile")?;
    let equipment_ready = required_bool(team_profile, "equipment_ready")?;
    let status = if equipment_ready {
        "ready"
    } else {
        "needs_action"
    };
    let required_actions = if equipment_ready {
        Vec::<String>::new()
    } else {
        vec!["complete equipment verification".to_string()]
    };
    let readiness_result = serde_json::json!({
        "readiness_result_id": format!("readiness-{objective_id}"),
        "objective_id": objective_id,
        "status": status,
        "reasons": ["team profile satisfies baseline expedition requirements"],
        "required_actions": required_actions.clone()
    });

    Ok(serde_json::json!({
        "readiness_result_id": format!("readiness-{objective_id}"),
        "objective_id": objective_id,
        "status": status,
        "reasons": ["team profile satisfies baseline expedition requirements"],
        "required_actions": required_actions,
        "readiness_result": readiness_result,
        "emitted_events": [event_ref("expedition.planning.team-readiness-validated")]
    }))
}

fn execute_assemble_expedition_plan(
    input: &Value,
) -> Result<Value, traverse_runtime::LocalExecutionFailure> {
    let map = input_object(input)?;
    let objective = required_object(map, "objective")?;
    let objective_id = required_string(objective, "objective_id")?;
    let interpreted = required_object(map, "interpreted_intent")?;
    let route_preferences = required_string_array(interpreted, "route_preferences")?;
    let constraints = required_string_array(interpreted, "constraints")?;
    let readiness = required_object(map, "readiness_result")?;
    let readiness_status = required_string(readiness, "status")?;
    let readiness_reasons = required_string_array(readiness, "reasons")?;
    let required_actions = required_string_array(readiness, "required_actions")?;
    let route_style = route_preferences
        .first()
        .cloned()
        .unwrap_or_else(|| "conservative-alpine-push".to_string());

    let mut readiness_notes = readiness_reasons;
    readiness_notes.extend(required_actions);

    Ok(serde_json::json!({
        "plan_id": format!("plan-{objective_id}"),
        "objective_id": objective_id,
        "status": if readiness_status == "ready" { "ready" } else { "requires_attention" },
        "recommended_route_style": route_style,
        "key_steps": [
            "depart before sunrise",
            "reassess winds at mid-route checkpoint",
            "apply conservative turnaround time"
        ],
        "constraints": constraints,
        "readiness_notes": readiness_notes,
        "summary": "Proceed with a conservative same-day ascent plan under a limited morning weather window.",
        "emitted_events": [event_ref("expedition.planning.expedition-plan-assembled")]
    }))
}

fn input_object(
    value: &Value,
) -> Result<&serde_json::Map<String, Value>, traverse_runtime::LocalExecutionFailure> {
    value
        .as_object()
        .ok_or_else(|| executor_failure("executor input must be an object"))
}

fn required_object<'a>(
    map: &'a serde_json::Map<String, Value>,
    key: &str,
) -> Result<&'a serde_json::Map<String, Value>, traverse_runtime::LocalExecutionFailure> {
    map.get(key)
        .and_then(Value::as_object)
        .ok_or_else(|| executor_failure(&format!("missing object field: {key}")))
}

fn required_value<'a>(
    map: &'a serde_json::Map<String, Value>,
    key: &str,
) -> Result<&'a Value, traverse_runtime::LocalExecutionFailure> {
    map.get(key)
        .ok_or_else(|| executor_failure(&format!("missing field: {key}")))
}

fn required_string<'a>(
    map: &'a serde_json::Map<String, Value>,
    key: &str,
) -> Result<&'a str, traverse_runtime::LocalExecutionFailure> {
    map.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| executor_failure(&format!("missing string field: {key}")))
}

fn required_bool(
    map: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<bool, traverse_runtime::LocalExecutionFailure> {
    map.get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| executor_failure(&format!("missing boolean field: {key}")))
}

fn required_string_array(
    map: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Vec<String>, traverse_runtime::LocalExecutionFailure> {
    let items = map
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| executor_failure(&format!("missing string array field: {key}")))?;

    items
        .iter()
        .map(|item| {
            item.as_str()
                .map(ToString::to_string)
                .ok_or_else(|| executor_failure(&format!("invalid string array field: {key}")))
        })
        .collect()
}

fn executor_failure(message: &str) -> traverse_runtime::LocalExecutionFailure {
    traverse_runtime::LocalExecutionFailure {
        code: traverse_runtime::LocalExecutionFailureCode::ExecutionFailed,
        message: message.to_string(),
    }
}

fn event_ref(event_id: &str) -> Value {
    serde_json::json!({
        "event_id": event_id,
        "version": "1.0.0"
    })
}

fn build_capability_registration(
    bundle: &RegistryBundle,
    capability: &traverse_registry::CapabilityBundleArtifact,
) -> Result<CapabilityRegistration, StdioServerFailure> {
    let raw_contract = read_text_file(&capability.path, "capability contract")?;
    let envelope = serde_json::from_str::<Value>(&raw_contract).map_err(|error| {
        StdioServerFailure::new(
            "invalid_request",
            format!(
                "failed to parse capability registration metadata {}: {error}",
                capability.path.display()
            ),
        )
    })?;
    let implementation_kind = derive_implementation_kind(envelope.get("composability"));
    let workflow_ref = derive_workflow_ref(envelope.get("composability"))?;
    let composability =
        derive_composability_metadata(implementation_kind, workflow_ref.as_ref(), capability)?;
    let artifact = build_capability_artifact(bundle, capability, implementation_kind, workflow_ref);

    Ok(CapabilityRegistration {
        scope: bundle.scope,
        contract: capability.contract.clone(),
        contract_path: capability.path.display().to_string(),
        artifact,
        registered_at: bundle_registered_at(bundle),
        tags: Vec::new(),
        composability,
        governing_spec: "005-capability-registry".to_string(),
        validator_version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

fn derive_implementation_kind(composability_value: Option<&Value>) -> ImplementationKind {
    match composability_value
        .and_then(|composability| composability.get("implementation_kind"))
        .and_then(Value::as_str)
    {
        Some("workflow") => ImplementationKind::Workflow,
        _ => ImplementationKind::Executable,
    }
}

fn derive_workflow_ref(
    composability_value: Option<&Value>,
) -> Result<Option<WorkflowReference>, StdioServerFailure> {
    composability_value
        .and_then(|composability| composability.get("workflow_ref"))
        .map(parse_workflow_ref)
        .transpose()
}

fn derive_composability_metadata(
    implementation_kind: ImplementationKind,
    workflow_ref: Option<&WorkflowReference>,
    capability: &traverse_registry::CapabilityBundleArtifact,
) -> Result<ComposabilityMetadata, StdioServerFailure> {
    let requires = capability
        .contract
        .consumes
        .iter()
        .map(|event| event.event_id.clone())
        .collect();

    match implementation_kind {
        ImplementationKind::Workflow => {
            if workflow_ref.is_none() {
                return Err(StdioServerFailure::new(
                    "invalid_request",
                    format!(
                        "workflow-backed capability {} must declare workflow_ref",
                        capability.contract.id
                    ),
                ));
            }
            Ok(ComposabilityMetadata {
                kind: CompositionKind::Composite,
                patterns: vec![CompositionPattern::Sequential],
                provides: vec![capability.contract.id.clone()],
                requires,
            })
        }
        ImplementationKind::Executable => Ok(ComposabilityMetadata {
            kind: CompositionKind::Atomic,
            patterns: vec![CompositionPattern::Sequential],
            provides: vec![capability.contract.id.clone()],
            requires,
        }),
    }
}

fn build_capability_artifact(
    bundle: &RegistryBundle,
    capability: &traverse_registry::CapabilityBundleArtifact,
    implementation_kind: ImplementationKind,
    workflow_ref: Option<WorkflowReference>,
) -> traverse_registry::CapabilityArtifactRecord {
    traverse_registry::CapabilityArtifactRecord {
        artifact_ref: format!(
            "bundle:{}:{}:{}",
            bundle.bundle_id, capability.contract.id, capability.contract.version
        ),
        implementation_kind,
        source: SourceReference {
            kind: SourceKind::Local,
            location: capability.path.display().to_string(),
        },
        binary: match implementation_kind {
            ImplementationKind::Executable => Some(BinaryReference {
                format: RegistryBinaryFormat::Wasm,
                location: format!(
                    "bundled://{}/{}/module.wasm",
                    capability.contract.id, capability.contract.version
                ),
            }),
            ImplementationKind::Workflow => None,
        },
        workflow_ref,
        digests: traverse_registry::ArtifactDigests {
            source_digest: format!(
                "source:{}:{}",
                capability.contract.id, capability.contract.version
            ),
            binary_digest: match implementation_kind {
                ImplementationKind::Executable => Some(format!(
                    "binary:{}:{}",
                    capability.contract.id, capability.contract.version
                )),
                ImplementationKind::Workflow => None,
            },
        },
        provenance: RegistryProvenance {
            source: provenance_source_label(&capability.contract.provenance.source),
            author: capability.contract.provenance.author.clone(),
            created_at: capability.contract.provenance.created_at.clone(),
        },
    }
}

fn bundle_registered_at(bundle: &RegistryBundle) -> String {
    format!("bundle:{}@{}", bundle.bundle_id, bundle.version)
}

fn parse_workflow_ref(value: &Value) -> Result<WorkflowReference, StdioServerFailure> {
    let workflow_id = value
        .get("workflow_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            StdioServerFailure::new(
                "invalid_request",
                "workflow_ref.workflow_id must be a string",
            )
        })?;
    let workflow_version = value
        .get("workflow_version")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            StdioServerFailure::new(
                "invalid_request",
                "workflow_ref.workflow_version must be a string",
            )
        })?;
    Ok(WorkflowReference {
        workflow_id: workflow_id.to_string(),
        workflow_version: workflow_version.to_string(),
    })
}

fn provenance_source_label(source: &traverse_contracts::ProvenanceSource) -> String {
    match source {
        traverse_contracts::ProvenanceSource::Greenfield => "greenfield",
        traverse_contracts::ProvenanceSource::BrownfieldExtracted => "brownfield-extracted",
        traverse_contracts::ProvenanceSource::AiGenerated => "ai-generated",
        traverse_contracts::ProvenanceSource::AiAssisted => "ai-assisted",
    }
    .to_string()
}

fn slug(value: &str) -> String {
    let mut slug = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.trim_matches('-').to_string()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StdioServerFailure {
    pub code: &'static str,
    pub message: String,
}

impl StdioServerFailure {
    #[must_use]
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn envelope(&self) -> Value {
        json!({
            "kind": "mcp_stdio_server_error",
            "server_name": SERVER_NAME,
            "host_mode": HOST_MODE,
            "governing_spec": GOVERNING_SPEC,
            "status": "error",
            "code": self.code,
            "message": self.message,
        })
    }
}

impl fmt::Display for StdioServerFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for StdioServerFailure {}

/// Run the dedicated Traverse MCP stdio server entrypoint.
///
/// # Errors
///
/// Returns `usage` when the CLI arguments request help or omit the required `stdio` mode.
/// Returns `startup_failed` when deterministic startup failure simulation is enabled.
/// Returns `io_error` when delegating to the server loop fails to read or write stdio data.
pub fn run_mcp_stdio_server<I, W, E>(
    args: I,
    stdout: &mut W,
    stderr: &mut E,
) -> Result<(), StdioServerFailure>
where
    I: IntoIterator<Item = String>,
    W: Write,
    E: Write,
{
    let mut simulate_startup_failure = false;

    for arg in args {
        match arg.as_str() {
            "stdio" => {}
            "--simulate-startup-failure" => {
                simulate_startup_failure = true;
            }
            "-h" | "--help" => {
                return Err(StdioServerFailure::new(
                    "usage",
                    "Usage: cargo run -p traverse-mcp -- stdio [--simulate-startup-failure]",
                ));
            }
            other => {
                return Err(StdioServerFailure::new(
                    "invalid_argument",
                    format!("Unsupported Traverse MCP server argument: {other}"),
                ));
            }
        }
    }

    let catalog = McpDiscoveryCatalog::load_canonical()?;
    let execution_context = CanonicalExecutionContext::load_canonical()?;
    let capability_registry = CapabilityRegistry::new();
    let event_registry = EventRegistry::new();
    let workflow_registry = WorkflowRegistry::new();
    let runtime = Runtime::new(
        execution_context.capability_registry,
        ExpeditionStdioExecutor,
    )
    .with_workflow_registry(execution_context.workflow_registry);
    let mcp = TraverseMcp::new(
        &capability_registry,
        &event_registry,
        &workflow_registry,
        &runtime,
    );
    let server = TraverseMcpStdioServer::new(&mcp, Box::leak(Box::new(catalog)));
    let input = io::stdin();
    let input = input.lock();
    server.run_stdio(input, stdout, stderr, simulate_startup_failure)
}

fn parse_command(line: &str) -> Result<StdioCommandEnvelope, StdioServerFailure> {
    serde_json::from_str(line).map_err(|error| {
        StdioServerFailure::new(
            "invalid_request",
            format!("Commands must be JSON objects with a command field: {error}"),
        )
    })
}

fn write_json_line<W: Write>(writer: &mut W, value: &Value) -> io::Result<()> {
    serde_json::to_writer(&mut *writer, value).map_err(io::Error::other)?;
    writer.write_all(b"\n")
}

fn capability_entrypoint_summary(artifact: &traverse_registry::CapabilityBundleArtifact) -> Value {
    let contract = &artifact.contract;
    json!({
        "entrypoint_kind": "capability",
        "artifact_kind": "capability",
        "scope": "public",
        "id": contract.id.clone(),
        "version": contract.version.clone(),
        "lifecycle": lifecycle_name(&contract.lifecycle),
        "summary": contract.summary.clone(),
        "owner_team": contract.owner.team.clone(),
        "invocation_surface_id": PUBLIC_SURFACE_ID,
    })
}

fn workflow_entrypoint_summary(artifact: &traverse_registry::WorkflowBundleArtifact) -> Value {
    let definition = &artifact.definition;
    json!({
        "entrypoint_kind": "workflow",
        "artifact_kind": "workflow",
        "scope": "public",
        "id": definition.id.clone(),
        "version": definition.version.clone(),
        "lifecycle": lifecycle_name(&definition.lifecycle),
        "summary": definition.summary.clone(),
        "owner_team": definition.owner.team.clone(),
        "start_node": definition.start_node.clone(),
        "terminal_nodes": definition.terminal_nodes.clone(),
        "node_count": definition.nodes.len(),
        "edge_count": definition.edges.len(),
        "invocation_surface_id": PUBLIC_SURFACE_ID,
    })
}

fn capability_entrypoint_detail(artifact: &traverse_registry::CapabilityBundleArtifact) -> Value {
    let contract = &artifact.contract;
    json!({
        "entrypoint_kind": "capability",
        "artifact_kind": "capability",
        "scope": "public",
        "id": contract.id.clone(),
        "version": contract.version.clone(),
        "lifecycle": lifecycle_name(&contract.lifecycle),
        "summary": contract.summary.clone(),
        "description": contract.description.clone(),
        "owner_team": contract.owner.team.clone(),
        "owner_contact": contract.owner.contact.clone(),
        "entrypoint_command": contract.execution.entrypoint.command.clone(),
        "binary_format": format!("{:?}", contract.execution.binary_format).to_lowercase(),
        "invocation_surface_id": PUBLIC_SURFACE_ID,
        "artifact_path": artifact.path.display().to_string(),
        "tags": Vec::<String>::new(),
    })
}

fn workflow_entrypoint_detail(artifact: &traverse_registry::WorkflowBundleArtifact) -> Value {
    let definition = &artifact.definition;
    json!({
        "entrypoint_kind": "workflow",
        "artifact_kind": "workflow",
        "scope": "public",
        "id": definition.id.clone(),
        "version": definition.version.clone(),
        "lifecycle": lifecycle_name(&definition.lifecycle),
        "summary": definition.summary.clone(),
        "owner_team": definition.owner.team.clone(),
        "owner_contact": definition.owner.contact.clone(),
        "start_node": definition.start_node.clone(),
        "terminal_nodes": definition.terminal_nodes.clone(),
        "node_count": definition.nodes.len(),
        "edge_count": definition.edges.len(),
        "invocation_surface_id": PUBLIC_SURFACE_ID,
        "artifact_path": artifact.path.display().to_string(),
        "participating_capabilities": definition
            .nodes
            .iter()
            .map(|node| format!("{}@{}", node.capability_id, node.capability_version))
            .collect::<Vec<_>>(),
    })
}

fn lifecycle_name(lifecycle: &Lifecycle) -> &'static str {
    match lifecycle {
        Lifecycle::Draft => "draft",
        Lifecycle::Active => "active",
        Lifecycle::Deprecated => "deprecated",
        Lifecycle::Retired => "retired",
        Lifecycle::Archived => "archived",
    }
}

fn canonical_expedition_bundle_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("examples/expedition/registry-bundle/manifest.json")
}

fn not_found(kind: &str, id: &str, version: &str) -> StdioServerFailure {
    StdioServerFailure::new("not_found", format!("{kind} {id}@{version} was not found"))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic, clippy::unwrap_used, clippy::manual_let_else)]

    use super::*;
    use traverse_registry::{CapabilityRegistry, EventRegistry, WorkflowRegistry};
    use traverse_runtime::Runtime;

    fn server_fixture() -> TraverseMcpStdioServer<'static, ExpeditionStdioExecutor> {
        let capability_registry = Box::leak(Box::new(CapabilityRegistry::new()));
        let event_registry = Box::leak(Box::new(EventRegistry::new()));
        let workflow_registry = Box::leak(Box::new(WorkflowRegistry::new()));
        let CanonicalExecutionContext {
            capability_registry: execution_capability_registry,
            workflow_registry: execution_workflow_registry,
        } = CanonicalExecutionContext::load_canonical().unwrap();
        let runtime = Box::leak(Box::new(
            Runtime::new(execution_capability_registry, ExpeditionStdioExecutor)
                .with_workflow_registry(execution_workflow_registry),
        ));
        let mcp = Box::leak(Box::new(TraverseMcp::new(
            capability_registry,
            event_registry,
            workflow_registry,
            runtime,
        )));
        let catalog = Box::leak(Box::new(McpDiscoveryCatalog::load_canonical().unwrap()));
        TraverseMcpStdioServer::new(mcp, catalog)
    }

    #[test]
    fn emits_deterministic_startup_list_validate_execute_and_shutdown_envelopes() {
        let server = server_fixture();
        let input = std::io::Cursor::new(
            br#"{"command":"list_entrypoints"}
{"command":"describe_entrypoint","entrypoint_kind":"capability","id":"expedition.planning.capture-expedition-objective","version":"1.0.0"}
{"command":"describe_entrypoint","entrypoint_kind":"workflow","id":"expedition.planning.plan-expedition","version":"1.0.0"}
{"command":"validate_entrypoint","entrypoint_kind":"workflow","id":"expedition.planning.plan-expedition","version":"1.0.0","request_path":"examples/expedition/runtime-requests/plan-expedition.json"}
{"command":"execute_entrypoint","entrypoint_kind":"workflow","id":"expedition.planning.plan-expedition","version":"1.0.0","request_path":"examples/expedition/runtime-requests/plan-expedition.json"}
{"command":"shutdown"}
"#,
        );
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        assert!(
            server
                .run_stdio(input, &mut stdout, &mut stderr, false)
                .is_ok()
        );

        let output = String::from_utf8(stdout).unwrap();
        assert!(output.contains("\"kind\":\"mcp_stdio_server_startup\""));
        assert!(output.contains("\"host_mode\":\"stdio\""));
        assert!(output.contains("\"kind\":\"mcp_stdio_server_entrypoint_list\""));
        assert!(output.contains("\"entrypoint_kind\":\"capability\""));
        assert!(output.contains("\"entrypoint_kind\":\"workflow\""));
        assert!(output.contains("\"kind\":\"mcp_stdio_server_entrypoint_validation\""));
        assert!(output.contains("\"kind\":\"mcp_stdio_server_entrypoint_execution\""));
        assert!(output.contains("\"status\":\"completed\""));
        assert!(output.contains("\"kind\":\"mcp_stdio_server_shutdown\""));
        assert!(stderr.is_empty());
    }

    #[test]
    fn emits_machine_readable_startup_failure() {
        let server = server_fixture();
        let input = std::io::Cursor::new(Vec::<u8>::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = server
            .run_stdio(input, &mut stdout, &mut stderr, true)
            .unwrap_err();

        assert_eq!(error.code, "startup_failed");
        let stderr_text = String::from_utf8(stderr).unwrap();
        assert!(stderr_text.contains("\"kind\":\"mcp_stdio_server_error\""));
        assert!(stderr_text.contains("\"code\":\"startup_failed\""));
        assert!(stdout.is_empty());
    }

    #[test]
    fn rejects_non_json_commands_deterministically() {
        let server = server_fixture();
        let input = std::io::Cursor::new("not-json\n");
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = server
            .run_stdio(input, &mut stdout, &mut stderr, false)
            .unwrap_err();

        assert_eq!(error.code, "invalid_request");
        let stderr_text = String::from_utf8(stderr).unwrap();
        assert!(stderr_text.contains("\"code\":\"invalid_request\""));
        assert!(stdout.contains(&b'\n'));
    }
}

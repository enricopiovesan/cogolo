//! Deterministic stdio server foundation for Traverse MCP.

use crate::{TraverseMcp, youaskm3_mcp_consumption_validation_path};
use serde::Deserialize;
use serde_json::{Value, json};
use std::fmt;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use traverse_contracts::Lifecycle;
use traverse_registry::{
    CapabilityRegistry, EventRegistry, RegistryBundle, WorkflowRegistry, load_registry_bundle,
};
use traverse_runtime::LocalExecutor;
use traverse_runtime::Runtime;

const SERVER_NAME: &str = "traverse-mcp";
const HOST_MODE: &str = "stdio";
const GOVERNING_SPEC: &str = "022-mcp-wasm-server";
const PUBLIC_SURFACE_ID: &str = "traverse.mcp.stdio-server";
const SUPPORTING_COMMANDS: &[&str] = &[
    "describe_server",
    "list_entrypoints",
    "describe_entrypoint",
    "shutdown",
];
const FUTURE_OPERATIONS: &[&str] = &[
    "mcp.capabilities.discover",
    "mcp.capability.get",
    "mcp.runtime.execute",
    "mcp.runtime.observe_execution",
];

#[derive(Debug, Deserialize)]
struct StdioCommandEnvelope {
    command: String,
    #[serde(default)]
    entrypoint_kind: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    version: Option<String>,
}

#[derive(Debug)]
pub struct McpDiscoveryCatalog {
    bundle: RegistryBundle,
}

impl McpDiscoveryCatalog {
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
            .map(|artifact| capability_entrypoint_summary(artifact))
            .collect::<Vec<_>>();
        let workflow_entries = self
            .catalog
            .bundle
            .workflows
            .iter()
            .map(|artifact| workflow_entrypoint_summary(artifact))
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
struct StdioBootstrapExecutor;

impl LocalExecutor for StdioBootstrapExecutor {
    fn execute(
        &self,
        _capability: &traverse_registry::ResolvedCapability,
        _input: &Value,
    ) -> Result<Value, traverse_runtime::LocalExecutionFailure> {
        Err(traverse_runtime::LocalExecutionFailure {
            code: traverse_runtime::LocalExecutionFailureCode::ExecutionFailed,
            message: "stdio foundation does not execute runtime requests".to_string(),
        })
    }
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
    let capability_registry = CapabilityRegistry::new();
    let event_registry = EventRegistry::new();
    let workflow_registry = WorkflowRegistry::new();
    let runtime = Runtime::new(CapabilityRegistry::new(), StdioBootstrapExecutor)
        .with_workflow_registry(WorkflowRegistry::new());
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
    use serde_json::Value;
    use traverse_registry::{CapabilityRegistry, EventRegistry, WorkflowRegistry};
    use traverse_runtime::{LocalExecutionFailure, Runtime};

    #[derive(Debug)]
    struct NoopExecutor;

    impl LocalExecutor for NoopExecutor {
        fn execute(
            &self,
            _capability: &traverse_registry::ResolvedCapability,
            _input: &Value,
        ) -> Result<Value, LocalExecutionFailure> {
            Err(LocalExecutionFailure {
                code: traverse_runtime::LocalExecutionFailureCode::ExecutionFailed,
                message: "stdio foundation does not execute runtime requests".to_string(),
            })
        }
    }

    fn server_fixture() -> TraverseMcpStdioServer<'static, NoopExecutor> {
        let capability_registry = Box::leak(Box::new(CapabilityRegistry::new()));
        let event_registry = Box::leak(Box::new(EventRegistry::new()));
        let workflow_registry = Box::leak(Box::new(WorkflowRegistry::new()));
        let runtime = Box::leak(Box::new(
            Runtime::new(CapabilityRegistry::new(), NoopExecutor)
                .with_workflow_registry(WorkflowRegistry::new()),
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
    fn emits_deterministic_startup_list_describe_and_shutdown_envelopes() {
        let server = server_fixture();
        let input = std::io::Cursor::new(
            br#"{"command":"list_entrypoints"}
{"command":"describe_entrypoint","entrypoint_kind":"capability","id":"expedition.planning.capture-expedition-objective","version":"1.0.0"}
{"command":"describe_entrypoint","entrypoint_kind":"workflow","id":"expedition.planning.plan-expedition","version":"1.0.0"}
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

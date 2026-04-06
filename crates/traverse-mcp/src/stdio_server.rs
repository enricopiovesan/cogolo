//! Deterministic stdio server foundation for Traverse MCP.

use crate::{
    DiscoveryQuery, McpLookupScope, TraverseMcp, youaskm3_mcp_consumption_validation_path,
};
use serde_json::{Value, json};
use std::fmt;
use std::io::{self, BufRead, Write};
use traverse_registry::{CapabilityRegistry, EventRegistry, WorkflowRegistry};
use traverse_runtime::LocalExecutor;
use traverse_runtime::Runtime;

const SERVER_NAME: &str = "traverse-mcp";
const HOST_MODE: &str = "stdio";
const GOVERNING_SPEC: &str = "022-mcp-wasm-server";
const PUBLIC_SURFACE_ID: &str = "traverse.mcp.stdio-server";
const SUPPORTING_COMMANDS: &[&str] = &["describe", "shutdown"];
const FUTURE_OPERATIONS: &[&str] = &[
    "mcp.capabilities.discover",
    "mcp.capability.get",
    "mcp.runtime.execute",
    "mcp.runtime.observe_execution",
];

#[derive(Debug)]
pub struct TraverseMcpStdioServer<'a, E> {
    mcp: &'a TraverseMcp<'a, E>,
}

impl<'a, E> TraverseMcpStdioServer<'a, E>
where
    E: LocalExecutor,
{
    #[must_use]
    pub fn new(mcp: &'a TraverseMcp<'a, E>) -> Self {
        Self { mcp }
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
        let capability_count = self
            .mcp
            .discover_capabilities(McpLookupScope::PreferPrivate, &DiscoveryQuery::default())
            .len();
        let event_count = self
            .mcp
            .discover_events(McpLookupScope::PreferPrivate)
            .len();
        let workflow_count = self
            .mcp
            .discover_workflows(McpLookupScope::PreferPrivate)
            .len();

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
                "capabilities": capability_count,
                "events": event_count,
                "workflows": workflow_count,
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
            match command.as_str() {
                "describe" => {
                    write_json_line(stdout, &self.describe_envelope()).map_err(|error| {
                        StdioServerFailure::new(
                            "io_error",
                            format!("Failed to write describe envelope: {error}"),
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
    let server = TraverseMcpStdioServer::new(&mcp);
    let input = io::stdin();
    let input = input.lock();
    server.run_stdio(input, stdout, stderr, simulate_startup_failure)
}

fn parse_command(line: &str) -> Result<String, StdioServerFailure> {
    let value: Value = serde_json::from_str(line).map_err(|error| {
        StdioServerFailure::new(
            "invalid_request",
            format!("Commands must be JSON objects with a command field: {error}"),
        )
    })?;

    let command = value
        .get("command")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            StdioServerFailure::new(
                "invalid_request",
                "Commands must contain a string `command` field.",
            )
        })?;

    Ok(command.to_string())
}

fn write_json_line<W: Write>(writer: &mut W, value: &Value) -> io::Result<()> {
    serde_json::to_writer(&mut *writer, value).map_err(io::Error::other)?;
    writer.write_all(b"\n")
}

#[cfg(test)]
mod tests {
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
        TraverseMcpStdioServer::new(mcp)
    }

    #[test]
    fn emits_deterministic_startup_describe_and_shutdown_envelopes() {
        let server = server_fixture();
        let input = std::io::Cursor::new(
            br#"{"command":"describe"}
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
        assert!(output.contains("\"kind\":\"mcp_stdio_server_description\""));
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

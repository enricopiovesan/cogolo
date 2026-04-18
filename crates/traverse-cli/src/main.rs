mod agent_packages;
mod browser_adapter;
mod federation_operator;

use agent_packages::load_agent_package;
use browser_adapter::serve_local_browser_adapter;
use federation_operator::{
    render_federation_peers, render_federation_status, render_federation_sync,
};
use serde_json::Value;
use std::env;
use std::fs;
use std::path::Component;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use traverse_contracts::{
    EventContract, EventValidationContext, parse_event_contract, validate_event_contract,
};
use traverse_registry::{
    ArtifactDigests, BinaryFormat, BinaryReference, CapabilityArtifactRecord,
    CapabilityRegistration, CapabilityRegistry, ComposabilityMetadata, CompositionKind,
    CompositionPattern, EventRegistration, EventRegistry, ImplementationKind, RegistryBundle,
    RegistryProvenance, SourceKind, SourceReference, WorkflowDefinition, WorkflowReference,
    WorkflowRegistration, WorkflowRegistry, load_registry_bundle,
};
use traverse_runtime::{
    LocalExecutionFailure, LocalExecutionFailureCode, LocalExecutor, Runtime,
    RuntimeExecutionOutcome, RuntimeRequest, RuntimeResultStatus, RuntimeTrace,
    parse_runtime_request,
};

#[derive(Debug)]
enum Command {
    BundleInspect {
        manifest_path: PathBuf,
    },
    BundleRegister {
        manifest_path: PathBuf,
    },
    BrowserAdapterServe {
        bind_address: String,
    },
    AgentInspect {
        manifest_path: PathBuf,
    },
    AgentExecute {
        manifest_path: PathBuf,
        request_path: PathBuf,
    },
    FederationPeers {
        manifest_path: PathBuf,
    },
    FederationSync {
        manifest_path: PathBuf,
    },
    FederationStatus {
        manifest_path: PathBuf,
    },
    ExpeditionExecute {
        request_path: PathBuf,
        trace_output_path: Option<PathBuf>,
    },
    Event {
        contract_path: PathBuf,
    },
    TraceInspect {
        trace_path: PathBuf,
    },
    Workflow {
        workflow_path: PathBuf,
    },
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    match parse_command(&args) {
        Ok(Command::BrowserAdapterServe { bind_address }) => {
            if let Err(error) = serve_local_browser_adapter(&bind_address) {
                eprintln!("{error}");
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Ok(command) => match run_command(command) {
            Ok(output) => {
                println!("{output}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("{error}");
                ExitCode::FAILURE
            }
        },
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run_command(command: Command) -> Result<String, String> {
    match command {
        Command::BundleInspect { manifest_path } => inspect_bundle(&manifest_path),
        Command::BundleRegister { manifest_path } => register_bundle(&manifest_path),
        Command::BrowserAdapterServe { .. } => Err(usage()),
        Command::AgentInspect { manifest_path } => inspect_agent(&manifest_path),
        Command::AgentExecute {
            manifest_path,
            request_path,
        } => execute_agent(&manifest_path, &request_path),
        Command::FederationPeers { manifest_path } => render_federation_peers(&manifest_path),
        Command::FederationSync { manifest_path } => render_federation_sync(&manifest_path),
        Command::FederationStatus { manifest_path } => render_federation_status(&manifest_path),
        Command::ExpeditionExecute {
            request_path,
            trace_output_path,
        } => execute_expedition(&request_path, trace_output_path.as_deref()),
        Command::Event { contract_path } => inspect_event(&contract_path),
        Command::TraceInspect { trace_path } => inspect_trace(&trace_path),
        Command::Workflow { workflow_path } => inspect_workflow(&workflow_path),
    }
}

fn parse_command(args: &[String]) -> Result<Command, String> {
    // Handle global --help / help
    if args.get(1).map(String::as_str) == Some("--help")
        || args.get(1).map(String::as_str) == Some("help")
    {
        return Err(usage());
    }

    // Handle per-subcommand --help
    let family = args.get(1).map(String::as_str);
    let subcommand = args.get(2).map(String::as_str);
    let has_help_flag = args.iter().any(|a| a == "--help");

    if has_help_flag {
        return Err(subcommand_help(family, subcommand));
    }

    match (family, subcommand) {
        (Some("browser-adapter"), Some("serve")) => parse_browser_adapter_command(args),
        (Some("federation"), Some(_)) => parse_federation_command(args),
        (Some("agent"), Some("execute")) => parse_agent_execute_command(args),
        (Some("expedition"), Some("execute")) => parse_expedition_execute_command(args),
        _ => parse_fixed_arity_command(args),
    }
}

fn subcommand_help(family: Option<&str>, subcommand: Option<&str>) -> String {
    match (family, subcommand) {
        (Some("bundle"), Some("inspect")) => help_bundle_inspect(),
        (Some("bundle"), Some("register")) => help_bundle_register(),
        (Some("bundle"), _) => help_bundle(),
        (Some("agent"), Some("inspect")) => help_agent_inspect(),
        (Some("agent"), Some("execute")) => help_agent_execute(),
        (Some("agent"), _) => help_agent(),
        (Some("workflow"), Some("inspect")) => help_workflow_inspect(),
        (Some("workflow"), _) => help_workflow(),
        (Some("expedition"), Some("execute")) => help_expedition_execute(),
        (Some("expedition"), _) => help_expedition(),
        (Some("capability"), Some("inspect")) => help_capability_inspect(),
        (Some("capability"), _) => help_capability(),
        (Some("event"), Some("inspect")) => help_event_inspect(),
        (Some("event"), _) => help_event(),
        (Some("trace"), Some("inspect")) => help_trace_inspect(),
        (Some("trace"), _) => help_trace(),
        (Some("browser-adapter"), Some("serve")) => help_browser_adapter_serve(),
        (Some("browser-adapter"), _) => help_browser_adapter(),
        _ => usage(),
    }
}

fn help_bundle_inspect() -> String {
    "traverse-cli bundle inspect <manifest-path>

  Purpose:
    Validate and summarize a registry bundle manifest. Reads the manifest JSON,
    resolves all declared capability/event/workflow artifact paths, and prints a
    structured summary of the bundle without registering anything.

  Required arguments:
    <manifest-path>   Path to the registry bundle manifest.json file.

  Optional flags:
    --help            Print this help text.

  Example:
    traverse-cli bundle inspect examples/expedition/registry-bundle/manifest.json"
        .to_string()
}

fn help_bundle_register() -> String {
    "traverse-cli bundle register <manifest-path>

  Purpose:
    Load a registry bundle and register its capabilities, events, and workflows
    into in-memory registries. Validates all artifact contracts and reports the
    set of records that would be committed.

  Required arguments:
    <manifest-path>   Path to the registry bundle manifest.json file.

  Optional flags:
    --help            Print this help text.

  Example:
    traverse-cli bundle register examples/expedition/registry-bundle/manifest.json"
        .to_string()
}

fn help_bundle() -> String {
    "traverse-cli bundle <subcommand> [options]

  Subcommands:
    inspect <manifest-path>    Validate and summarize a bundle manifest.
    register <manifest-path>   Register bundle artifacts into in-memory registries.

  Run `traverse-cli bundle <subcommand> --help` for subcommand-specific help."
        .to_string()
}

fn help_agent_inspect() -> String {
    "traverse-cli agent inspect <manifest-path>

  Purpose:
    Load and summarize a governed WASM agent package manifest. Verifies the
    binary digest, resolves the capability contract, and prints package metadata
    including model dependencies and workflow references.

  Required arguments:
    <manifest-path>   Path to the agent package manifest.json file.

  Optional flags:
    --help            Print this help text.

  Example:
    traverse-cli agent inspect examples/agents/expedition-intent-agent/manifest.json"
        .to_string()
}

fn help_agent_execute() -> String {
    "traverse-cli agent execute <manifest-path> <request-path>

  Purpose:
    Load a governed WASM agent package and execute it against a runtime request.
    Validates the package binary digest, registers the capability, and runs the
    request through the Traverse runtime.

  Required arguments:
    <manifest-path>   Path to the agent package manifest.json file.
    <request-path>    Path to the runtime request JSON file.

  Optional flags:
    --help            Print this help text.

  Example:
    traverse-cli agent execute \\
      examples/agents/expedition-intent-agent/manifest.json \\
      examples/agents/runtime-requests/interpret-expedition-intent.json"
        .to_string()
}

fn help_agent() -> String {
    "traverse-cli agent <subcommand> [options]

  Subcommands:
    inspect <manifest-path>                      Summarize a governed agent package.
    execute <manifest-path> <request-path>       Execute an agent against a runtime request.

  Run `traverse-cli agent <subcommand> --help` for subcommand-specific help."
        .to_string()
}

fn help_workflow_inspect() -> String {
    "traverse-cli workflow inspect <workflow-path>

  Purpose:
    Parse and summarize a workflow definition artifact. Prints the workflow id,
    version, lifecycle, start/terminal nodes, node-to-capability mappings, and
    edge topology.

  Required arguments:
    <workflow-path>   Path to the workflow definition JSON file.

  Optional flags:
    --help            Print this help text.

  Example:
    traverse-cli workflow inspect workflows/examples/expedition/plan-expedition/workflow.json"
        .to_string()
}

fn help_workflow() -> String {
    "traverse-cli workflow <subcommand> [options]

  Subcommands:
    inspect <workflow-path>    Parse and summarize a workflow definition.

  Run `traverse-cli workflow inspect --help` for subcommand-specific help."
        .to_string()
}

fn help_expedition_execute() -> String {
    "traverse-cli expedition execute <request-path> [--trace-out <trace-path>]

  Purpose:
    Execute the canonical expedition workflow through the Traverse runtime.
    Loads the built-in expedition registry bundle, runs the request, and prints
    a structured execution summary. Optionally writes the full runtime trace to
    a JSON file for later inspection with `trace inspect`.

  Required arguments:
    <request-path>          Path to the runtime request JSON file.

  Optional flags:
    --trace-out <path>      Write the runtime trace artifact to this path.
    --help                  Print this help text.

  Example:
    traverse-cli expedition execute \\
      examples/expedition/runtime-requests/plan-expedition.json \\
      --trace-out target/traces/plan-expedition.json"
        .to_string()
}

fn help_expedition() -> String {
    "traverse-cli expedition <subcommand> [options]

  Subcommands:
    execute <request-path> [--trace-out <path>]  Run the expedition workflow.

  Run `traverse-cli expedition execute --help` for subcommand-specific help."
        .to_string()
}

fn help_capability_inspect() -> String {
    "traverse-cli capability inspect <contract-path>

  Purpose:
    Parse and validate a capability contract file. Prints contract metadata
    including id, version, lifecycle, input/output schema references, and
    provenance information.

  Required arguments:
    <contract-path>   Path to the capability contract JSON file.

  Optional flags:
    --help            Print this help text.

  Example:
    traverse-cli capability inspect \\
      contracts/examples/expedition/capabilities/capture-expedition-objective/contract.json"
        .to_string()
}

fn help_capability() -> String {
    "traverse-cli capability <subcommand> [options]

  Subcommands:
    inspect <contract-path>   Parse and validate a capability contract.

  Run `traverse-cli capability inspect --help` for subcommand-specific help."
        .to_string()
}

fn help_event_inspect() -> String {
    "traverse-cli event inspect <contract-path>

  Purpose:
    Parse and validate an event contract file. Prints the event id, version,
    lifecycle, classification (domain/event-type), publisher and subscriber
    capability bindings, and tags.

  Required arguments:
    <contract-path>   Path to the event contract JSON file.

  Optional flags:
    --help            Print this help text.

  Example:
    traverse-cli event inspect \\
      contracts/examples/expedition/events/expedition-objective-captured/contract.json"
        .to_string()
}

fn help_event() -> String {
    "traverse-cli event <subcommand> [options]

  Subcommands:
    inspect <contract-path>   Parse and validate an event contract.

  Run `traverse-cli event inspect --help` for subcommand-specific help."
        .to_string()
}

fn help_trace_inspect() -> String {
    "traverse-cli trace inspect <trace-path>

  Purpose:
    Parse and summarize a runtime trace artifact produced by `expedition execute
    --trace-out`. Prints trace metadata, state-machine validation results, the
    candidate collection summary, the selected capability, and the terminal state
    transition.

  Required arguments:
    <trace-path>   Path to the runtime trace JSON file.

  Optional flags:
    --help         Print this help text.

  Example:
    traverse-cli trace inspect target/traces/plan-expedition.json"
        .to_string()
}

fn help_trace() -> String {
    "traverse-cli trace <subcommand> [options]

  Subcommands:
    inspect <trace-path>   Parse and summarize a runtime trace artifact.

  Run `traverse-cli trace inspect --help` for subcommand-specific help."
        .to_string()
}

fn help_browser_adapter_serve() -> String {
    "traverse-cli browser-adapter serve [--bind <address>]

  Purpose:
    Start the local browser adapter proxy. The adapter bridges browser-side
    consumers to the local Traverse runtime over a same-origin HTTP endpoint.
    Stays running until stopped (Ctrl-C).

  Optional flags:
    --bind <address>   Address and port to listen on (default: 127.0.0.1:0).
    --help             Print this help text.

  Example:
    traverse-cli browser-adapter serve --bind 127.0.0.1:4174"
        .to_string()
}

fn help_browser_adapter() -> String {
    "traverse-cli browser-adapter <subcommand> [options]

  Subcommands:
    serve [--bind <address>]   Start the local browser adapter proxy.

  Run `traverse-cli browser-adapter serve --help` for subcommand-specific help."
        .to_string()
}

fn parse_browser_adapter_command(args: &[String]) -> Result<Command, String> {
    match args.len() {
        3 => Ok(Command::BrowserAdapterServe {
            bind_address: "127.0.0.1:0".to_string(),
        }),
        5 if args[3] == "--bind" => Ok(Command::BrowserAdapterServe {
            bind_address: args[4].clone(),
        }),
        _ => Err(usage()),
    }
}

fn parse_fixed_arity_command(args: &[String]) -> Result<Command, String> {
    if args.len() != 4 {
        return Err(usage());
    }

    match (args[1].as_str(), args[2].as_str()) {
        ("bundle", "inspect") => Ok(Command::BundleInspect {
            manifest_path: PathBuf::from(&args[3]),
        }),
        ("bundle", "register") => Ok(Command::BundleRegister {
            manifest_path: PathBuf::from(&args[3]),
        }),
        ("agent", "inspect") => Ok(Command::AgentInspect {
            manifest_path: PathBuf::from(&args[3]),
        }),
        ("federation", "peers") => Ok(Command::FederationPeers {
            manifest_path: PathBuf::from(&args[3]),
        }),
        ("federation", "sync") => Ok(Command::FederationSync {
            manifest_path: PathBuf::from(&args[3]),
        }),
        ("federation", "status") => Ok(Command::FederationStatus {
            manifest_path: PathBuf::from(&args[3]),
        }),
        ("event", "inspect") => Ok(Command::Event {
            contract_path: PathBuf::from(&args[3]),
        }),
        ("trace", "inspect") => Ok(Command::TraceInspect {
            trace_path: PathBuf::from(&args[3]),
        }),
        ("workflow", "inspect") => Ok(Command::Workflow {
            workflow_path: PathBuf::from(&args[3]),
        }),
        _ => Err(usage()),
    }
}

fn parse_agent_execute_command(args: &[String]) -> Result<Command, String> {
    match args {
        [_, _, _, manifest_path, request_path] => Ok(Command::AgentExecute {
            manifest_path: PathBuf::from(manifest_path),
            request_path: PathBuf::from(request_path),
        }),
        _ => Err(usage()),
    }
}

fn parse_federation_command(args: &[String]) -> Result<Command, String> {
    match args {
        [_, _, _, manifest_path] if args[2] == "peers" => Ok(Command::FederationPeers {
            manifest_path: PathBuf::from(manifest_path),
        }),
        [_, _, _, manifest_path] if args[2] == "sync" => Ok(Command::FederationSync {
            manifest_path: PathBuf::from(manifest_path),
        }),
        [_, _, _, manifest_path] if args[2] == "status" => Ok(Command::FederationStatus {
            manifest_path: PathBuf::from(manifest_path),
        }),
        _ => Err(usage()),
    }
}

fn parse_expedition_execute_command(args: &[String]) -> Result<Command, String> {
    match args {
        [_, _, _, request_path] => Ok(Command::ExpeditionExecute {
            request_path: PathBuf::from(request_path),
            trace_output_path: None,
        }),
        [_, _, _, request_path, flag, trace_output_path] if flag == "--trace-out" => {
            Ok(Command::ExpeditionExecute {
                request_path: PathBuf::from(request_path),
                trace_output_path: Some(PathBuf::from(trace_output_path)),
            })
        }
        _ => Err(usage()),
    }
}

fn inspect_bundle(manifest_path: &Path) -> Result<String, String> {
    let bundle =
        load_registry_bundle(manifest_path).map_err(|failure| failure.errors[0].message.clone())?;
    Ok(render_bundle_summary(&bundle))
}

fn register_bundle(manifest_path: &Path) -> Result<String, String> {
    let registered = load_registered_bundle(manifest_path)?;
    Ok(render_bundle_registration_summary(
        &registered.bundle,
        &registered.capability_records,
        &registered.event_records,
        &registered.workflow_records,
    ))
}

fn inspect_agent(manifest_path: &Path) -> Result<String, String> {
    let package = load_agent_package(manifest_path)?;
    Ok(package.render_summary())
}

fn execute_agent(manifest_path: &Path, request_path: &Path) -> Result<String, String> {
    let package = load_agent_package(manifest_path)?;
    let request = load_runtime_request(request_path)?;
    let mut registry = CapabilityRegistry::new();
    registry
        .register(package.capability_registration())
        .map_err(render_registry_failure)?;
    let runtime = Runtime::new(registry, AgentPackageExampleExecutor);
    let outcome = runtime.execute(request);

    if outcome.result.status == RuntimeResultStatus::Error {
        return Err(render_runtime_execution_failure(&outcome));
    }

    Ok(render_agent_execution_summary(
        &package.manifest.package_id,
        &package.manifest.capability_ref.id,
        &outcome,
    ))
}

fn execute_expedition(
    request_path: &Path,
    trace_output_path: Option<&Path>,
) -> Result<String, String> {
    let outcome = execute_expedition_outcome(request_path)?;

    if outcome.result.status == RuntimeResultStatus::Error {
        return Err(render_runtime_execution_failure(&outcome));
    }

    if let Some(path) = trace_output_path {
        write_trace_artifact(path, &outcome.trace)?;
    }

    Ok(render_runtime_execution_summary(
        &outcome,
        trace_output_path,
    ))
}

fn canonical_expedition_runtime_outcome() -> Result<RuntimeExecutionOutcome, String> {
    execute_expedition_outcome(&canonical_expedition_request_path())
}

fn inspect_event(contract_path: &Path) -> Result<String, String> {
    let contents = read_text_file(contract_path, "event contract")?;
    let parsed = parse_event_contract(&contents)
        .map_err(|failure| render_validation_failure("event contract", contract_path, failure))?;
    let validated = validate_event_contract(
        parsed,
        &EventValidationContext {
            governing_spec: "003-event-contracts",
            validator_version: env!("CARGO_PKG_VERSION"),
            existing_published: None,
        },
    )
    .map_err(|failure| render_validation_failure("event contract", contract_path, failure))?;

    Ok(render_event_summary(contract_path, &validated.normalized))
}

fn inspect_workflow(workflow_path: &Path) -> Result<String, String> {
    let contents = read_text_file(workflow_path, "workflow artifact")?;
    let definition = serde_json::from_str::<WorkflowDefinition>(&contents).map_err(|error| {
        format!(
            "failed to parse workflow artifact {}: {error}",
            workflow_path.display()
        )
    })?;

    Ok(render_workflow_summary(workflow_path, &definition))
}

fn inspect_trace(trace_path: &Path) -> Result<String, String> {
    let contents = read_text_file(trace_path, "runtime trace")?;
    let trace = serde_json::from_str::<RuntimeTrace>(&contents).map_err(|error| {
        format!(
            "failed to parse runtime trace {}: {error}",
            trace_path.display()
        )
    })?;

    Ok(render_trace_summary(trace_path, &trace))
}

fn read_text_file(path: &Path, artifact_kind: &str) -> Result<String, String> {
    fs::read_to_string(path)
        .map_err(|error| format!("failed to read {artifact_kind} {}: {error}", path.display()))
}

fn render_validation_failure(
    artifact_kind: &str,
    path: &Path,
    failure: traverse_contracts::ValidationFailure,
) -> String {
    let details = failure
        .errors
        .into_iter()
        .map(|error| format!("{} at {}", error.message, error.path))
        .collect::<Vec<_>>()
        .join("; ");

    format!(
        "failed to validate {artifact_kind} {}: {details}",
        path.display()
    )
}

fn render_bundle_summary(bundle: &RegistryBundle) -> String {
    let mut lines = vec![
        format!("bundle_id: {}", bundle.bundle_id),
        format!("version: {}", bundle.version),
        format!("scope: {:?}", bundle.scope).to_lowercase(),
        format!("capabilities: {}", bundle.capabilities.len()),
        format!("events: {}", bundle.events.len()),
        format!("workflows: {}", bundle.workflows.len()),
        "capability_ids:".to_string(),
    ];

    for capability in &bundle.capabilities {
        lines.push(format!(
            "  - {}@{}",
            capability.manifest.id, capability.manifest.version
        ));
    }

    lines.push("event_ids:".to_string());
    for event in &bundle.events {
        lines.push(format!(
            "  - {}@{}",
            event.manifest.id, event.manifest.version
        ));
    }

    lines.push("workflow_ids:".to_string());
    for workflow in &bundle.workflows {
        lines.push(format!(
            "  - {}@{}",
            workflow.manifest.id, workflow.manifest.version
        ));
    }

    lines.join("\n")
}

fn render_bundle_registration_summary(
    bundle: &RegistryBundle,
    capability_records: &[String],
    event_records: &[String],
    workflow_records: &[String],
) -> String {
    let mut lines = vec![
        format!("bundle_id: {}", bundle.bundle_id),
        format!("version: {}", bundle.version),
        format!("scope: {:?}", bundle.scope).to_lowercase(),
        format!("registered_capabilities: {}", capability_records.len()),
        format!("registered_events: {}", event_records.len()),
        format!("registered_workflows: {}", workflow_records.len()),
        "capability_records:".to_string(),
    ];

    for record in capability_records {
        lines.push(format!("  - {record}"));
    }

    lines.push("event_records:".to_string());
    for record in event_records {
        lines.push(format!("  - {record}"));
    }

    lines.push("workflow_records:".to_string());
    for record in workflow_records {
        lines.push(format!("  - {record}"));
    }

    lines.join("\n")
}

fn render_event_summary(path: &Path, contract: &EventContract) -> String {
    let mut lines = vec![
        format!("path: {}", path.display()),
        format!("id: {}", contract.id),
        format!("version: {}", contract.version),
        format!("lifecycle: {:?}", contract.lifecycle).to_lowercase(),
        format!("event_type: {:?}", contract.classification.event_type).to_lowercase(),
        format!("domain: {}", contract.classification.domain),
        format!(
            "bounded_context: {}",
            contract.classification.bounded_context
        ),
        format!("publishers: {}", contract.publishers.len()),
        format!("subscribers: {}", contract.subscribers.len()),
        format!("tags: {}", contract.tags.join(",")),
        "publisher_ids:".to_string(),
    ];

    for publisher in &contract.publishers {
        lines.push(format!(
            "  - {}@{}",
            publisher.capability_id, publisher.version
        ));
    }

    lines.push("subscriber_ids:".to_string());
    for subscriber in &contract.subscribers {
        lines.push(format!(
            "  - {}@{}",
            subscriber.capability_id, subscriber.version
        ));
    }

    lines.join("\n")
}

fn render_workflow_summary(path: &Path, definition: &WorkflowDefinition) -> String {
    let mut lines = vec![
        format!("path: {}", path.display()),
        format!("id: {}", definition.id),
        format!("version: {}", definition.version),
        format!("lifecycle: {:?}", definition.lifecycle).to_lowercase(),
        format!("start_node: {}", definition.start_node),
        format!("terminal_nodes: {}", definition.terminal_nodes.join(",")),
        format!("node_count: {}", definition.nodes.len()),
        format!("edge_count: {}", definition.edges.len()),
        format!("governing_spec: {}", definition.governing_spec),
        "node_capabilities:".to_string(),
    ];

    for node in &definition.nodes {
        lines.push(format!(
            "  - {} -> {}@{}",
            node.node_id, node.capability_id, node.capability_version
        ));
    }

    lines.push("edges:".to_string());
    for edge in &definition.edges {
        lines.push(format!(
            "  - {}: {} -> {}",
            edge.edge_id, edge.from, edge.to
        ));
    }

    lines.join("\n")
}

fn render_runtime_execution_summary(
    outcome: &RuntimeExecutionOutcome,
    trace_output_path: Option<&Path>,
) -> String {
    let output = outcome.result.output.as_ref().unwrap_or(&Value::Null);
    let mut lines = vec![
        format!("request_id: {}", outcome.result.request_id),
        format!("execution_id: {}", outcome.result.execution_id),
        "capability_id: expedition.planning.plan-expedition".to_string(),
        "capability_version: 1.0.0".to_string(),
        "status: completed".to_string(),
        format!("trace_ref: {}", outcome.result.trace_ref),
    ];

    if let Some(path) = trace_output_path {
        lines.push(format!("trace_path: {}", path.display()));
    }

    if let Some(plan_id) = output.get("plan_id").and_then(Value::as_str) {
        lines.push(format!("plan_id: {plan_id}"));
    }
    if let Some(objective_id) = output.get("objective_id").and_then(Value::as_str) {
        lines.push(format!("objective_id: {objective_id}"));
    }
    if let Some(route_style) = output
        .get("recommended_route_style")
        .and_then(Value::as_str)
    {
        lines.push(format!("recommended_route_style: {route_style}"));
    }
    if let Some(summary) = output.get("summary").and_then(Value::as_str) {
        lines.push(format!("summary: {summary}"));
    }

    lines.join("\n")
}

fn render_agent_execution_summary(
    package_id: &str,
    capability_id: &str,
    outcome: &RuntimeExecutionOutcome,
) -> String {
    let output = outcome.result.output.as_ref().unwrap_or(&Value::Null);
    let mut lines = vec![
        format!("request_id: {}", outcome.result.request_id),
        format!("execution_id: {}", outcome.result.execution_id),
        format!("package_id: {package_id}"),
        format!("capability_id: {capability_id}"),
        "capability_version: 1.0.0".to_string(),
        "status: completed".to_string(),
        format!("trace_ref: {}", outcome.result.trace_ref),
    ];

    match capability_id {
        "expedition.planning.interpret-expedition-intent" => {
            if let Some(intent_id) = output.get("intent_id").and_then(Value::as_str) {
                lines.push(format!("intent_id: {intent_id}"));
            }
            if let Some(objective_id) = output.get("objective_id").and_then(Value::as_str) {
                lines.push(format!("objective_id: {objective_id}"));
            }
            if let Some(confidence) = output.get("confidence").and_then(Value::as_f64) {
                lines.push(format!("confidence: {confidence:.2}"));
            }
            if let Some(route_preferences) =
                output.get("route_preferences").and_then(Value::as_array)
            {
                let joined = route_preferences
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(", ");
                lines.push(format!("route_preferences: {joined}"));
            }
        }
        "expedition.planning.validate-team-readiness" => {
            if let Some(readiness_result_id) =
                output.get("readiness_result_id").and_then(Value::as_str)
            {
                lines.push(format!("readiness_result_id: {readiness_result_id}"));
            }
            if let Some(objective_id) = output.get("objective_id").and_then(Value::as_str) {
                lines.push(format!("objective_id: {objective_id}"));
            }
            if let Some(status) = output.get("status").and_then(Value::as_str) {
                lines.push(format!("readiness_status: {status}"));
            }
            if let Some(required_actions) = output.get("required_actions").and_then(Value::as_array)
            {
                let joined = required_actions
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(", ");
                lines.push(format!("required_actions: {joined}"));
            }
        }
        "hello.world.say-hello" => {
            if let Some(name) = output.get("name").and_then(Value::as_str) {
                lines.push(format!("name: {name}"));
            }
            if let Some(greeting) = output.get("greeting").and_then(Value::as_str) {
                lines.push(format!("greeting: {greeting}"));
            }
        }
        _ => {}
    }

    lines.join("\n")
}

fn render_trace_summary(trace_path: &Path, trace: &RuntimeTrace) -> String {
    let final_transition = trace.state_transitions.last();
    let mut lines = vec![
        format!("path: {}", trace_path.display()),
        format!("trace_id: {}", trace.trace_id),
        format!("execution_id: {}", trace.execution_id),
        format!("request_id: {}", trace.request_id),
        format!("governing_spec: {}", trace.governing_spec),
        format!("result_status: {:?}", trace.result.status).to_lowercase(),
        format!(
            "state_machine_validation: {:?}",
            trace.state_machine_validation.status
        )
        .to_lowercase(),
        format!("state_transition_count: {}", trace.state_transitions.len()),
        format!(
            "candidate_count: {}",
            trace.candidate_collection.candidates.len()
        ),
        format!(
            "rejected_candidate_count: {}",
            trace.candidate_collection.rejected_candidates.len()
        ),
        format!("execution_status: {:?}", trace.execution.status).to_lowercase(),
    ];

    if let Some(selected) = &trace.selection.selected_capability_id {
        lines.push(format!("selected_capability_id: {selected}"));
    }
    if let Some(version) = &trace.selection.selected_capability_version {
        lines.push(format!("selected_capability_version: {version}"));
    }
    if let Some(artifact_ref) = &trace.execution.artifact_ref {
        lines.push(format!("artifact_ref: {artifact_ref}"));
    }
    if let Some(transition) = final_transition {
        lines.push(format!(
            "terminal_transition: {} -> {} ({})",
            format!("{:?}", transition.from_state).to_lowercase(),
            format!("{:?}", transition.to_state).to_lowercase(),
            debug_enum_to_snake_case(&format!("{:?}", transition.reason_code))
        ));
    }
    if let Some(error) = &trace.result.error {
        lines.push(format!("error_code: {:?}", error.code).to_lowercase());
        lines.push(format!("error_message: {}", error.message));
    }

    lines.join("\n")
}

fn usage() -> String {
    "usage: traverse-cli <bundle|agent|event|trace|workflow|expedition|federation> <inspect|register|execute|peers|sync|status> <artifact-path> [request-path] [--trace-out <trace-path>] | traverse-cli browser-adapter serve [--bind <address>]".to_string()
}

fn write_trace_artifact(path: &Path, trace: &RuntimeTrace) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create trace artifact directory {}: {error}",
                parent.display()
            )
        })?;
    }

    let serialized = serde_json::to_string_pretty(trace).map_err(|error| {
        format!(
            "failed to serialize runtime trace {}: {error}",
            path.display()
        )
    })?;
    fs::write(path, format!("{serialized}\n"))
        .map_err(|error| format!("failed to write runtime trace {}: {error}", path.display()))
}

fn debug_enum_to_snake_case(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 4);
    for (index, ch) in value.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if index > 0 {
                output.push('_');
            }
            output.push(ch.to_ascii_lowercase());
        } else {
            output.push(ch);
        }
    }
    output
}

#[derive(Debug)]
struct RegisteredBundle {
    bundle: RegistryBundle,
    capability_registry: CapabilityRegistry,
    event_registry: EventRegistry,
    workflow_registry: WorkflowRegistry,
    capability_records: Vec<String>,
    event_records: Vec<String>,
    workflow_records: Vec<String>,
}

#[derive(Debug, Default, Clone, Copy)]
struct ExpeditionExampleExecutor;

impl LocalExecutor for ExpeditionExampleExecutor {
    fn execute(
        &self,
        capability: &traverse_registry::ResolvedCapability,
        input: &Value,
    ) -> Result<Value, LocalExecutionFailure> {
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
                "unsupported expedition example capability: {other}"
            ))),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct AgentPackageExampleExecutor;

impl LocalExecutor for AgentPackageExampleExecutor {
    fn execute(
        &self,
        capability: &traverse_registry::ResolvedCapability,
        input: &Value,
    ) -> Result<Value, LocalExecutionFailure> {
        match capability.contract.id.as_str() {
            "hello.world.say-hello" => execute_hello_world(input),
            "expedition.planning.interpret-expedition-intent" => {
                execute_interpret_expedition_intent(input)
            }
            "expedition.planning.validate-team-readiness" => execute_validate_team_readiness(input),
            other => Err(executor_failure(&format!(
                "unsupported AI agent capability: {other}"
            ))),
        }
    }
}

fn build_capability_registration(
    bundle: &RegistryBundle,
    capability: &traverse_registry::CapabilityBundleArtifact,
) -> Result<CapabilityRegistration, String> {
    let raw_contract = read_text_file(&capability.path, "capability contract")?;
    let envelope =
        parse_capability_registration_envelope(&raw_contract, capability.path.as_path())?;
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

fn load_registered_bundle(manifest_path: &Path) -> Result<RegisteredBundle, String> {
    let bundle =
        load_registry_bundle(manifest_path).map_err(|failure| failure.errors[0].message.clone())?;

    let mut capability_registry = CapabilityRegistry::new();
    let mut event_registry = EventRegistry::new();
    let mut workflow_registry = WorkflowRegistry::new();
    let mut capability_records = Vec::new();
    let mut event_records = Vec::new();
    let mut workflow_records = Vec::new();

    for capability in &bundle.capabilities {
        let request = build_capability_registration(&bundle, capability)?;
        let outcome = capability_registry
            .register(request)
            .map_err(render_registry_failure)?;
        capability_records.push(format_capability_record(
            &outcome.record.id,
            &outcome.record.version,
            outcome.record.implementation_kind,
        ));
    }

    for event in &bundle.events {
        let outcome = event_registry
            .register(EventRegistration {
                scope: bundle.scope,
                contract: event.contract.clone(),
                contract_path: event.path.display().to_string(),
                registered_at: bundle_registered_at(&bundle),
                governing_spec: "011-event-registry".to_string(),
                validator_version: env!("CARGO_PKG_VERSION").to_string(),
            })
            .map_err(render_event_registry_failure)?;
        event_records.push(format!("{}@{}", outcome.record.id, outcome.record.version));
    }

    for workflow in &bundle.workflows {
        let outcome = workflow_registry
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
            .map_err(render_workflow_failure)?;
        workflow_records.push(format!("{}@{}", outcome.record.id, outcome.record.version));
    }

    Ok(RegisteredBundle {
        bundle,
        capability_registry,
        event_registry,
        workflow_registry,
        capability_records,
        event_records,
        workflow_records,
    })
}

fn load_runtime_request(request_path: &Path) -> Result<RuntimeRequest, String> {
    let contents = read_text_file(request_path, "runtime request")?;
    parse_runtime_request(&contents).map_err(|error| {
        format!(
            "failed to parse runtime request {}: {error}",
            request_path.display()
        )
    })
}

fn parse_capability_registration_envelope(
    raw_contract: &str,
    path: &Path,
) -> Result<Value, String> {
    serde_json::from_str::<Value>(raw_contract).map_err(|error| {
        format!(
            "failed to parse capability registration metadata {}: {error}",
            path.display()
        )
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
) -> Result<Option<WorkflowReference>, String> {
    composability_value
        .and_then(|composability| composability.get("workflow_ref"))
        .map(parse_workflow_ref)
        .transpose()
}

fn derive_composability_metadata(
    implementation_kind: ImplementationKind,
    workflow_ref: Option<&WorkflowReference>,
    capability: &traverse_registry::CapabilityBundleArtifact,
) -> Result<ComposabilityMetadata, String> {
    let requires = capability
        .contract
        .consumes
        .iter()
        .map(|event| event.event_id.clone())
        .collect();

    match implementation_kind {
        ImplementationKind::Workflow => {
            if workflow_ref.is_none() {
                return Err(format!(
                    "workflow-backed capability {} must declare workflow_ref",
                    capability.contract.id
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
) -> CapabilityArtifactRecord {
    CapabilityArtifactRecord {
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
                format: BinaryFormat::Wasm,
                location: format!(
                    "bundled://{}/{}/module.wasm",
                    capability.contract.id, capability.contract.version
                ),
            }),
            ImplementationKind::Workflow => None,
        },
        workflow_ref,
        digests: ArtifactDigests {
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

fn parse_workflow_ref(value: &Value) -> Result<WorkflowReference, String> {
    let workflow_id = value
        .get("workflow_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "workflow_ref.workflow_id must be a string".to_string())?;
    let workflow_version = value
        .get("workflow_version")
        .and_then(Value::as_str)
        .ok_or_else(|| "workflow_ref.workflow_version must be a string".to_string())?;
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

fn canonical_expedition_bundle_path() -> PathBuf {
    repo_root().join("examples/expedition/registry-bundle/manifest.json")
}

fn canonical_expedition_request_path() -> PathBuf {
    repo_root().join("examples/expedition/runtime-requests/plan-expedition.json")
}

fn execute_expedition_outcome(request_path: &Path) -> Result<RuntimeExecutionOutcome, String> {
    let request = load_runtime_request(request_path)?;
    let registered = load_registered_bundle(&canonical_expedition_bundle_path())?;
    let runtime = Runtime::new(registered.capability_registry, ExpeditionExampleExecutor)
        .with_workflow_registry(registered.workflow_registry);
    Ok(runtime.execute(request))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn format_capability_record(
    id: &str,
    version: &str,
    implementation_kind: ImplementationKind,
) -> String {
    let kind = match implementation_kind {
        ImplementationKind::Executable => "executable",
        ImplementationKind::Workflow => "workflow",
    };
    format!("{id}@{version} ({kind})")
}

fn render_registry_failure(failure: traverse_registry::RegistryFailure) -> String {
    failure
        .errors
        .into_iter()
        .map(|error| format!("{} at {}", error.message, error.target))
        .collect::<Vec<_>>()
        .join("; ")
}

fn render_event_registry_failure(failure: traverse_registry::EventRegistryFailure) -> String {
    failure
        .errors
        .into_iter()
        .map(|error| format!("{} at {}", error.message, error.target))
        .collect::<Vec<_>>()
        .join("; ")
}

fn render_workflow_failure(failure: traverse_registry::WorkflowFailure) -> String {
    failure
        .errors
        .into_iter()
        .map(|error| format!("{} at {}", error.message, error.path))
        .collect::<Vec<_>>()
        .join("; ")
}

fn render_runtime_execution_failure(outcome: &RuntimeExecutionOutcome) -> String {
    match &outcome.result.error {
        Some(error) => format!("runtime execution failed: {}", error.message),
        None => "runtime execution failed".to_string(),
    }
}

fn execute_capture_expedition_objective(input: &Value) -> Result<Value, LocalExecutionFailure> {
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

fn execute_interpret_expedition_intent(input: &Value) -> Result<Value, LocalExecutionFailure> {
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

fn execute_assess_conditions_summary(input: &Value) -> Result<Value, LocalExecutionFailure> {
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

fn execute_validate_team_readiness(input: &Value) -> Result<Value, LocalExecutionFailure> {
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

fn execute_assemble_expedition_plan(input: &Value) -> Result<Value, LocalExecutionFailure> {
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

fn execute_hello_world(input: &Value) -> Result<Value, LocalExecutionFailure> {
    let map = input_object(input)?;
    let name = required_string(map, "name")?;

    Ok(serde_json::json!({
        "name": name,
        "greeting": format!("Hello, {name}!"),
    }))
}

fn event_ref(event_id: &str) -> Value {
    serde_json::json!({
        "event_id": event_id,
        "version": "1.0.0"
    })
}

fn input_object(value: &Value) -> Result<&serde_json::Map<String, Value>, LocalExecutionFailure> {
    value
        .as_object()
        .ok_or_else(|| executor_failure("executor input must be an object"))
}

fn required_object<'a>(
    map: &'a serde_json::Map<String, Value>,
    key: &str,
) -> Result<&'a serde_json::Map<String, Value>, LocalExecutionFailure> {
    map.get(key)
        .and_then(Value::as_object)
        .ok_or_else(|| executor_failure(&format!("missing object field: {key}")))
}

fn required_value<'a>(
    map: &'a serde_json::Map<String, Value>,
    key: &str,
) -> Result<&'a Value, LocalExecutionFailure> {
    map.get(key)
        .ok_or_else(|| executor_failure(&format!("missing field: {key}")))
}

fn required_string<'a>(
    map: &'a serde_json::Map<String, Value>,
    key: &str,
) -> Result<&'a str, LocalExecutionFailure> {
    map.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| executor_failure(&format!("missing string field: {key}")))
}

fn required_bool(
    map: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<bool, LocalExecutionFailure> {
    map.get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| executor_failure(&format!("missing boolean field: {key}")))
}

fn required_string_array(
    map: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Vec<String>, LocalExecutionFailure> {
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

fn executor_failure(message: &str) -> LocalExecutionFailure {
    LocalExecutionFailure {
        code: LocalExecutionFailureCode::ExecutionFailed,
        message: message.to_string(),
    }
}

fn slug(value: &str) -> String {
    let mut slug = String::new();
    for component in Path::new(value).components() {
        if let Component::Normal(part) = component {
            let part = part.to_string_lossy();
            for ch in part.chars() {
                if ch.is_ascii_alphanumeric() {
                    slug.push(ch.to_ascii_lowercase());
                }
            }
        }
    }
    if slug.is_empty() {
        "expedition".to_string()
    } else {
        slug
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::{
        execute_agent, execute_expedition, inspect_agent, inspect_bundle, inspect_event,
        inspect_trace, inspect_workflow, parse_command, register_bundle,
    };
    use crate::agent_packages::fnv1a64;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parse_command_accepts_supported_inspect_commands() {
        let bundle = vec![
            "traverse-cli".to_string(),
            "bundle".to_string(),
            "inspect".to_string(),
            "examples/expedition/registry-bundle/manifest.json".to_string(),
        ];
        let bundle_register = vec![
            "traverse-cli".to_string(),
            "bundle".to_string(),
            "register".to_string(),
            "examples/expedition/registry-bundle/manifest.json".to_string(),
        ];
        let agent_inspect = vec![
            "traverse-cli".to_string(),
            "agent".to_string(),
            "inspect".to_string(),
            "examples/agents/expedition-intent-agent/manifest.json".to_string(),
        ];
        let agent_execute = vec![
            "traverse-cli".to_string(),
            "agent".to_string(),
            "execute".to_string(),
            "examples/agents/expedition-intent-agent/manifest.json".to_string(),
            "examples/agents/runtime-requests/interpret-expedition-intent.json".to_string(),
        ];
        let expedition_execute = vec![
            "traverse-cli".to_string(),
            "expedition".to_string(),
            "execute".to_string(),
            "examples/expedition/runtime-requests/plan-expedition.json".to_string(),
        ];
        let event = vec![
            "traverse-cli".to_string(),
            "event".to_string(),
            "inspect".to_string(),
            "contracts/examples/expedition/events/expedition-objective-captured/contract.json"
                .to_string(),
        ];
        let trace = vec![
            "traverse-cli".to_string(),
            "trace".to_string(),
            "inspect".to_string(),
            "/tmp/plan-expedition-trace.json".to_string(),
        ];
        let workflow = vec![
            "traverse-cli".to_string(),
            "workflow".to_string(),
            "inspect".to_string(),
            "workflows/examples/expedition/plan-expedition/workflow.json".to_string(),
        ];
        let expedition_execute_with_trace = vec![
            "traverse-cli".to_string(),
            "expedition".to_string(),
            "execute".to_string(),
            "examples/expedition/runtime-requests/plan-expedition.json".to_string(),
            "--trace-out".to_string(),
            "/tmp/plan-expedition-trace.json".to_string(),
        ];

        assert!(parse_command(&bundle).is_ok());
        assert!(parse_command(&bundle_register).is_ok());
        assert!(parse_command(&agent_inspect).is_ok());
        assert!(parse_command(&agent_execute).is_ok());
        assert!(parse_command(&expedition_execute).is_ok());
        assert!(parse_command(&expedition_execute_with_trace).is_ok());
        assert!(parse_command(&event).is_ok());
        assert!(parse_command(&trace).is_ok());
        assert!(parse_command(&workflow).is_ok());
    }

    #[test]
    fn parse_command_rejects_unknown_shape() {
        let args = vec!["traverse-cli".to_string()];
        let result = parse_command(&args);
        assert!(result.is_err());
        let error = result.err().unwrap_or_default();
        assert!(error.contains("usage: traverse-cli"));
    }

    #[test]
    fn parse_command_returns_bundle_inspect_help_on_help_flag() {
        let args = vec![
            "traverse-cli".to_string(),
            "bundle".to_string(),
            "inspect".to_string(),
            "--help".to_string(),
        ];
        let result = parse_command(&args);
        assert!(result.is_err(), "expected Err for --help");
        let text = result.err().unwrap_or_default();
        assert!(
            text.contains("bundle inspect"),
            "expected 'bundle inspect' in help text"
        );
        assert!(
            text.contains("<manifest-path>"),
            "expected '<manifest-path>' in help text"
        );
        assert!(
            text.contains("Example:"),
            "expected 'Example:' in help text"
        );
    }

    #[test]
    fn parse_command_returns_bundle_register_help_on_help_flag() {
        let args = vec![
            "traverse-cli".to_string(),
            "bundle".to_string(),
            "register".to_string(),
            "--help".to_string(),
        ];
        let result = parse_command(&args);
        assert!(result.is_err(), "expected Err for --help");
        let text = result.err().unwrap_or_default();
        assert!(text.contains("bundle register"));
        assert!(text.contains("<manifest-path>"));
        assert!(text.contains("Example:"));
    }

    #[test]
    fn parse_command_returns_agent_inspect_help_on_help_flag() {
        let args = vec![
            "traverse-cli".to_string(),
            "agent".to_string(),
            "inspect".to_string(),
            "--help".to_string(),
        ];
        let result = parse_command(&args);
        assert!(result.is_err(), "expected Err for --help");
        let text = result.err().unwrap_or_default();
        assert!(text.contains("agent inspect"));
        assert!(text.contains("<manifest-path>"));
        assert!(text.contains("Example:"));
    }

    #[test]
    fn parse_command_returns_agent_execute_help_on_help_flag() {
        let args = vec![
            "traverse-cli".to_string(),
            "agent".to_string(),
            "execute".to_string(),
            "--help".to_string(),
        ];
        let result = parse_command(&args);
        assert!(result.is_err(), "expected Err for --help");
        let text = result.err().unwrap_or_default();
        assert!(text.contains("agent execute"));
        assert!(text.contains("<manifest-path>"));
        assert!(text.contains("<request-path>"));
        assert!(text.contains("Example:"));
    }

    #[test]
    fn parse_command_returns_workflow_inspect_help_on_help_flag() {
        let args = vec![
            "traverse-cli".to_string(),
            "workflow".to_string(),
            "inspect".to_string(),
            "--help".to_string(),
        ];
        let result = parse_command(&args);
        assert!(result.is_err(), "expected Err for --help");
        let text = result.err().unwrap_or_default();
        assert!(text.contains("workflow inspect"));
        assert!(text.contains("<workflow-path>"));
        assert!(text.contains("Example:"));
    }

    #[test]
    fn parse_command_returns_expedition_execute_help_on_help_flag() {
        let args = vec![
            "traverse-cli".to_string(),
            "expedition".to_string(),
            "execute".to_string(),
            "--help".to_string(),
        ];
        let result = parse_command(&args);
        assert!(result.is_err(), "expected Err for --help");
        let text = result.err().unwrap_or_default();
        assert!(text.contains("expedition execute"));
        assert!(text.contains("<request-path>"));
        assert!(text.contains("--trace-out"));
        assert!(text.contains("Example:"));
    }

    #[test]
    fn parse_command_returns_capability_inspect_help_on_help_flag() {
        let args = vec![
            "traverse-cli".to_string(),
            "capability".to_string(),
            "inspect".to_string(),
            "--help".to_string(),
        ];
        let result = parse_command(&args);
        assert!(result.is_err(), "expected Err for --help");
        let text = result.err().unwrap_or_default();
        assert!(text.contains("capability inspect"));
        assert!(text.contains("<contract-path>"));
        assert!(text.contains("Example:"));
    }

    #[test]
    fn parse_command_returns_event_inspect_help_on_help_flag() {
        let args = vec![
            "traverse-cli".to_string(),
            "event".to_string(),
            "inspect".to_string(),
            "--help".to_string(),
        ];
        let result = parse_command(&args);
        assert!(result.is_err(), "expected Err for --help");
        let text = result.err().unwrap_or_default();
        assert!(text.contains("event inspect"));
        assert!(text.contains("<contract-path>"));
        assert!(text.contains("Example:"));
    }

    #[test]
    fn parse_command_returns_trace_inspect_help_on_help_flag() {
        let args = vec![
            "traverse-cli".to_string(),
            "trace".to_string(),
            "inspect".to_string(),
            "--help".to_string(),
        ];
        let result = parse_command(&args);
        assert!(result.is_err(), "expected Err for --help");
        let text = result.err().unwrap_or_default();
        assert!(text.contains("trace inspect"));
        assert!(text.contains("<trace-path>"));
        assert!(text.contains("Example:"));
    }

    #[test]
    fn parse_command_returns_browser_adapter_serve_help_on_help_flag() {
        let args = vec![
            "traverse-cli".to_string(),
            "browser-adapter".to_string(),
            "serve".to_string(),
            "--help".to_string(),
        ];
        let result = parse_command(&args);
        assert!(result.is_err(), "expected Err for --help");
        let text = result.err().unwrap_or_default();
        assert!(text.contains("browser-adapter serve"));
        assert!(text.contains("--bind"));
        assert!(text.contains("Example:"));
    }

    #[test]
    fn parse_command_returns_family_help_when_only_family_and_help_flag() {
        let cases = vec![
            (vec!["traverse-cli", "bundle", "--help"], "bundle"),
            (vec!["traverse-cli", "agent", "--help"], "agent"),
            (vec!["traverse-cli", "workflow", "--help"], "workflow"),
            (vec!["traverse-cli", "expedition", "--help"], "expedition"),
            (vec!["traverse-cli", "event", "--help"], "event"),
            (vec!["traverse-cli", "trace", "--help"], "trace"),
        ];
        for (raw, expected_family) in cases {
            let args: Vec<String> = raw.into_iter().map(String::from).collect();
            let result = parse_command(&args);
            assert!(
                result.is_err(),
                "expected Err for --help on family {expected_family}"
            );
            let text = result.err().unwrap_or_default();
            assert!(
                text.contains(expected_family),
                "expected '{expected_family}' in family help text"
            );
        }
    }

    #[test]
    fn inspect_bundle_renders_canonical_example_bundle() {
        let manifest_path = repo_root().join("examples/expedition/registry-bundle/manifest.json");

        let output = inspect_bundle(&manifest_path).expect("bundle inspect should succeed");

        assert!(output.contains("bundle_id: expedition.planning.seed-bundle"));
        assert!(output.contains("event_ids:"));
        assert!(output.contains("workflow_ids:"));
    }

    #[test]
    fn inspect_bundle_rejects_missing_artifact_paths() {
        let temp_dir = unique_temp_dir();
        let manifest_path = temp_dir.join("manifest.json");
        fs::write(
            &manifest_path,
            r#"{
  "bundle_id": "expedition.planning.seed-bundle",
  "version": "1.0.0",
  "scope": "public",
  "capabilities": [
    {
      "id": "expedition.planning.capture-expedition-objective",
      "version": "1.0.0",
      "path": "missing/capability.json"
    }
  ],
  "events": [],
  "workflows": []
}"#,
        )
        .expect("manifest should write");

        let error = inspect_bundle(&manifest_path).expect_err("missing artifact path should fail");
        assert!(error.contains("missing artifact file"));
    }

    #[test]
    fn register_bundle_registers_canonical_expedition_artifacts() {
        let manifest_path = repo_root().join("examples/expedition/registry-bundle/manifest.json");

        let output = register_bundle(&manifest_path).expect("bundle register should succeed");

        assert!(output.contains("registered_capabilities: 6"));
        assert!(output.contains("registered_events: 5"));
        assert!(output.contains("registered_workflows: 1"));
        assert!(output.contains("expedition.planning.plan-expedition@1.0.0 (workflow)"));
    }

    #[test]
    fn register_bundle_rejects_duplicate_manifest_entries() {
        let temp_dir = unique_temp_dir();
        let manifest_path = temp_dir.join("manifest.json");
        fs::write(
            &manifest_path,
            r#"{
  "bundle_id": "expedition.planning.seed-bundle",
  "version": "1.0.0",
  "scope": "public",
  "capabilities": [
    {
      "id": "expedition.planning.capture-expedition-objective",
      "version": "1.0.0",
      "path": "../../../contracts/examples/expedition/capabilities/capture-expedition-objective/contract.json"
    },
    {
      "id": "expedition.planning.capture-expedition-objective",
      "version": "1.0.0",
      "path": "../../../contracts/examples/expedition/capabilities/capture-expedition-objective/contract.json"
    }
  ],
  "events": [],
  "workflows": []
}"#,
        )
        .expect("manifest should write");

        let error =
            register_bundle(&manifest_path).expect_err("duplicate bundle entries should fail");

        assert!(error.contains("duplicate capability artifact entry"));
    }

    #[test]
    fn execute_expedition_runs_canonical_plan_request() {
        let request_path =
            repo_root().join("examples/expedition/runtime-requests/plan-expedition.json");

        let output =
            execute_expedition(&request_path, None).expect("expedition execution should succeed");

        assert!(output.contains("capability_id: expedition.planning.plan-expedition"));
        assert!(output.contains("status: completed"));
        assert!(output.contains("recommended_route_style: conservative-alpine-push"));
    }

    #[test]
    fn inspect_agent_renders_governed_wasm_agent_package() {
        let fixture = create_interpret_expedition_intent_agent_fixture();

        let output = inspect_agent(&fixture.manifest_path).expect("agent inspect should succeed");

        assert!(
            output.contains("package_id: expedition.planning.interpret-expedition-intent-agent")
        );
        assert!(output.contains("capability_id: expedition.planning.interpret-expedition-intent"));
        assert!(output.contains("binary_digest: fnv1a64:"));
        assert!(output.contains("workflow_refs: expedition.planning.plan-expedition@1.0.0"));
    }

    #[test]
    fn execute_agent_runs_governed_ai_agent_request() {
        let fixture = create_interpret_expedition_intent_agent_fixture();
        let request_path =
            repo_root().join("examples/agents/runtime-requests/interpret-expedition-intent.json");

        let output = execute_agent(&fixture.manifest_path, &request_path)
            .expect("agent execution should succeed");

        assert!(
            output.contains("package_id: expedition.planning.interpret-expedition-intent-agent")
        );
        assert!(output.contains("capability_id: expedition.planning.interpret-expedition-intent"));
        assert!(output.contains("status: completed"));
        assert!(output.contains("route_preferences: conservative-alpine-push, same-day-return"));
    }

    #[test]
    fn inspect_agent_renders_second_governed_wasm_agent_package() {
        let fixture = create_validate_team_readiness_agent_fixture();

        let output = inspect_agent(&fixture.manifest_path).expect("agent inspect should succeed");

        assert!(output.contains("package_id: expedition.planning.validate-team-readiness-agent"));
        assert!(output.contains("capability_id: expedition.planning.validate-team-readiness"));
        assert!(output.contains("binary_digest: fnv1a64:"));
        assert!(output.contains("workflow_refs: expedition.planning.plan-expedition@1.0.0"));
    }

    #[test]
    fn execute_agent_runs_second_governed_ai_agent_request() {
        let fixture = create_validate_team_readiness_agent_fixture();
        let request_path =
            repo_root().join("examples/agents/runtime-requests/validate-team-readiness.json");

        let output = execute_agent(&fixture.manifest_path, &request_path)
            .expect("agent execution should succeed");

        assert!(output.contains("package_id: expedition.planning.validate-team-readiness-agent"));
        assert!(output.contains("capability_id: expedition.planning.validate-team-readiness"));
        assert!(output.contains("status: completed"));
        assert!(output.contains("readiness_status: ready"));
    }

    #[test]
    fn inspect_agent_renders_hello_world_package() {
        let fixture = create_hello_world_agent_fixture();

        let output = inspect_agent(&fixture.manifest_path).expect("agent inspect should succeed");

        assert!(output.contains("package_id: hello.world.say-hello-agent"));
        assert!(output.contains("capability_id: hello.world.say-hello"));
        assert!(output.contains("binary_digest: fnv1a64:"));
        assert!(output.contains("workflow_refs: hello.world.say-hello@1.0.0"));
    }

    #[test]
    fn execute_agent_runs_hello_world_request() {
        let fixture = create_hello_world_agent_fixture();
        let request_path = repo_root().join("examples/hello-world/runtime-requests/say-hello.json");

        let output = execute_agent(&fixture.manifest_path, &request_path)
            .expect("hello-world agent execution should succeed");

        assert!(output.contains("package_id: hello.world.say-hello-agent"));
        assert!(output.contains("capability_id: hello.world.say-hello"));
        assert!(output.contains("status: completed"));
        assert!(output.contains("name: Traverse"));
        assert!(output.contains("greeting: Hello, Traverse!"));
    }

    #[test]
    fn execute_expedition_writes_trace_artifact_when_requested() {
        let request_path =
            repo_root().join("examples/expedition/runtime-requests/plan-expedition.json");
        let temp_dir = unique_temp_dir();
        let trace_path = temp_dir.join("plan-expedition-trace.json");

        let output = execute_expedition(&request_path, Some(&trace_path))
            .expect("expedition execution with trace output should succeed");

        assert!(output.contains(&format!("trace_path: {}", trace_path.display())));
        let trace_contents = fs::read_to_string(&trace_path).expect("trace file should exist");
        assert!(trace_contents.contains("\"kind\": \"runtime_trace\""));
        assert!(trace_contents.contains("\"trace_id\":"));
    }

    #[test]
    fn execute_expedition_rejects_invalid_request_input() {
        let temp_dir = unique_temp_dir();
        let path = temp_dir.join("invalid-runtime-request.json");
        fs::write(
            &path,
            r#"{
  "kind": "runtime_request",
  "schema_version": "1.0.0",
  "request_id": "invalid-expedition-plan-request",
  "intent": {
    "capability_id": "expedition.planning.plan-expedition",
    "capability_version": "1.0.0"
  },
  "input": {
    "destination": "Sky Pilot",
    "target_window": {
      "start": "2026-07-20T04:30:00Z",
      "end": "2026-07-20T16:00:00Z"
    },
    "preferences": {
      "style": "conservative-alpine-push",
      "risk_tolerance": "moderate",
      "priority": "same-day-return"
    },
    "notes": "Missing planning intent on purpose.",
    "team_profile": {
      "team_id": "team-alpine-01",
      "member_count": 3,
      "experience_level": "advanced",
      "equipment_ready": true
    }
  },
  "lookup": {
    "scope": "public_only",
    "allow_ambiguity": false
  },
  "context": {
    "requested_target": "local"
  },
  "governing_spec": "006-runtime-request-execution"
}"#,
        )
        .expect("runtime request should write");

        let error =
            execute_expedition(&path, None).expect_err("invalid expedition execution should fail");

        assert!(error.contains("runtime execution failed"));
        assert!(error.contains("runtime request input does not satisfy"));
    }

    #[test]
    fn inspect_trace_renders_generated_expedition_trace() {
        let request_path =
            repo_root().join("examples/expedition/runtime-requests/plan-expedition.json");
        let temp_dir = unique_temp_dir();
        let trace_path = temp_dir.join("plan-expedition-trace.json");

        execute_expedition(&request_path, Some(&trace_path))
            .expect("expedition execution with trace output should succeed");

        let output = inspect_trace(&trace_path).expect("trace inspect should succeed");

        assert!(output.contains("trace_id: trace_exec_expedition-plan-request-001"));
        assert!(output.contains("result_status: completed"));
        assert!(output.contains("selected_capability_id: expedition.planning.plan-expedition"));
    }

    #[test]
    fn inspect_trace_rejects_malformed_trace_artifact() {
        let temp_dir = unique_temp_dir();
        let path = temp_dir.join("trace.json");
        fs::write(&path, "{\"trace_id\":true}").expect("trace file should write");

        let error = inspect_trace(&path).expect_err("malformed trace should fail");

        assert!(error.contains("failed to parse runtime trace"));
    }

    #[test]
    fn inspect_event_renders_canonical_event_contract() {
        let path = repo_root().join(
            "contracts/examples/expedition/events/expedition-objective-captured/contract.json",
        );

        let output = inspect_event(&path).expect("event inspect should succeed");

        assert!(output.contains("id: expedition.planning.expedition-objective-captured"));
        assert!(output.contains("event_type: domain"));
        assert!(output.contains("publisher_ids:"));
    }

    #[test]
    fn inspect_event_rejects_malformed_contract() {
        let temp_dir = unique_temp_dir();
        let path = temp_dir.join("event.json");
        fs::write(&path, "{\"kind\":\"event_contract\"}").expect("event file should write");

        let error = inspect_event(&path).expect_err("malformed event contract should fail");

        assert!(error.contains("failed to validate event contract"));
    }

    #[test]
    fn inspect_workflow_renders_canonical_workflow() {
        let path = repo_root().join("workflows/examples/expedition/plan-expedition/workflow.json");

        let output = inspect_workflow(&path).expect("workflow inspect should succeed");

        assert!(output.contains("id: expedition.planning.plan-expedition"));
        assert!(output.contains("start_node: capture_objective"));
        assert!(output.contains("node_capabilities:"));
    }

    #[test]
    fn inspect_workflow_rejects_malformed_definition() {
        let temp_dir = unique_temp_dir();
        let path = temp_dir.join("workflow.json");
        fs::write(&path, "{\"id\":true}").expect("workflow file should write");

        let error = inspect_workflow(&path).expect_err("malformed workflow should fail");

        assert!(error.contains("failed to parse workflow artifact"));
    }

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn unique_temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("traverse-cli-test-{nanos}"));
        fs::create_dir_all(&path).expect("temporary directory should create");
        path
    }

    struct AgentFixture {
        manifest_path: PathBuf,
    }

    fn create_interpret_expedition_intent_agent_fixture() -> AgentFixture {
        create_agent_package_fixture(&AgentPackageFixtureSpec {
            package_id: "expedition.planning.interpret-expedition-intent-agent",
            capability_id: "expedition.planning.interpret-expedition-intent",
            binary_name: "interpret-expedition-intent-agent.wasm",
            summary: "Governed WASM AI agent example for expedition intent interpretation.",
            contract_path: "contracts/examples/expedition/capabilities/interpret-expedition-intent/contract.json",
            model_interface: "expedition-intent-interpretation-v1",
            model_purpose: "Interpret free-form expedition planning intent into governed route preferences and assumptions.",
            workflow_id: "expedition.planning.plan-expedition",
        })
    }

    fn create_validate_team_readiness_agent_fixture() -> AgentFixture {
        create_agent_package_fixture(&AgentPackageFixtureSpec {
            package_id: "expedition.planning.validate-team-readiness-agent",
            capability_id: "expedition.planning.validate-team-readiness",
            binary_name: "validate-team-readiness-agent.wasm",
            summary: "Governed WASM AI agent example for expedition readiness validation.",
            contract_path: "contracts/examples/expedition/capabilities/validate-team-readiness/contract.json",
            model_interface: "expedition-readiness-validation-v1",
            model_purpose: "Validate expedition team readiness against governed objective, conditions, and team profile context.",
            workflow_id: "expedition.planning.plan-expedition",
        })
    }

    fn create_hello_world_agent_fixture() -> AgentFixture {
        create_agent_package_fixture(&AgentPackageFixtureSpec {
            package_id: "hello.world.say-hello-agent",
            capability_id: "hello.world.say-hello",
            binary_name: "say-hello-agent.wasm",
            summary: "Minimal governed hello-world agent package for Traverse onboarding.",
            contract_path: "contracts/examples/hello-world/capabilities/say-hello/contract.json",
            model_interface: "hello-world-greeting-v1",
            model_purpose: "Produce a simple deterministic greeting string for onboarding validation.",
            workflow_id: "hello.world.say-hello",
        })
    }

    struct AgentPackageFixtureSpec<'a> {
        package_id: &'a str,
        capability_id: &'a str,
        binary_name: &'a str,
        summary: &'a str,
        contract_path: &'a str,
        model_interface: &'a str,
        model_purpose: &'a str,
        workflow_id: &'a str,
    }

    fn create_agent_package_fixture(spec: &AgentPackageFixtureSpec<'_>) -> AgentFixture {
        let temp_dir = unique_temp_dir();
        let package_dir = temp_dir.join("agent");
        let artifact_dir = package_dir.join("artifacts");
        let source_dir = package_dir.join("src");
        fs::create_dir_all(&artifact_dir).expect("artifact directory should create");
        fs::create_dir_all(&source_dir).expect("source directory should create");

        let wasm_bytes = hex_to_bytes(
            "0061736d0100000001040160000003020100070a01065f737461727400000a040102000b",
        );
        let binary_path = artifact_dir.join(spec.binary_name);
        fs::write(&binary_path, &wasm_bytes).expect("wasm binary should write");
        fs::write(
            source_dir.join("agent.rs"),
            format!(
                "pub fn run() -> &'static str {{ \"{}\" }}\n",
                spec.capability_id
            ),
        )
        .expect("source file should write");

        let repo_root = repo_root();
        let manifest_path = package_dir.join("manifest.json");
        let manifest = format!(
            r#"{{
  "kind": "agent_package",
  "schema_version": "1.0.0",
  "package_id": "{}",
  "version": "1.0.0",
  "summary": "{}",
  "capability_ref": {{
    "id": "{}",
    "version": "1.0.0",
    "contract_path": "{}"
  }},
  "workflow_refs": [
    {{
      "workflow_id": "{}",
      "workflow_version": "1.0.0"
    }}
  ],
  "source": {{
    "path": "./src/agent.rs",
    "language": "rust",
    "entry": "run"
  }},
  "binary": {{
    "path": "./artifacts/{}",
    "format": "wasm",
    "expected_digest": "{}"
  }},
  "constraints": {{
    "host_api_access": "none",
    "network_access": "forbidden",
    "filesystem_access": "none"
  }},
  "model_dependencies": [
    {{
      "interface": "{}",
      "purpose": "{}"
    }}
  ]
}}"#,
            spec.package_id,
            spec.summary,
            spec.capability_id,
            repo_root.join(spec.contract_path).display(),
            spec.workflow_id,
            spec.binary_name,
            fnv1a64(&wasm_bytes),
            spec.model_interface,
            spec.model_purpose
        );
        fs::write(&manifest_path, manifest).expect("manifest should write");

        AgentFixture { manifest_path }
    }

    fn hex_to_bytes(value: &str) -> Vec<u8> {
        value
            .as_bytes()
            .chunks(2)
            .map(|pair| {
                let pair = std::str::from_utf8(pair).expect("hex pair should be utf8");
                u8::from_str_radix(pair, 16).expect("hex pair should parse")
            })
            .collect()
    }
}

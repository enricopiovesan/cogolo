use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{IpAddr, TcpListener, TcpStream};
use traverse_registry::{CapabilityRegistry, DiscoveryQuery, LookupScope, WorkflowRegistry};
use traverse_runtime::{
    LocalExecutor, Runtime, RuntimeExecutionOutcome, RuntimeRequest, RuntimeResultStatus,
    parse_runtime_request,
};

const MAX_REQUEST_BODY: usize = 4 * 1024 * 1024; // 4 MiB

/// Errors that can occur while serving the HTTP/JSON API.
#[derive(Debug)]
pub enum ServeError {
    BindFailed(String),
    AcceptFailed(String),
}

impl std::fmt::Display for ServeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServeError::BindFailed(msg) => write!(f, "failed to bind HTTP/JSON API server: {msg}"),
            ServeError::AcceptFailed(msg) => {
                write!(f, "HTTP/JSON API server accept loop failed: {msg}")
            }
        }
    }
}

/// Configuration for the HTTP/JSON API server.
pub struct ApiServerConfig<E> {
    pub port: u16,
    pub allow_unauthenticated: bool,
    pub capability_registry: CapabilityRegistry,
    pub workflow_registry: WorkflowRegistry,
    pub executor: E,
}

/// Start the HTTP/JSON API server, blocking until the listener fails.
///
/// # Errors
///
/// Returns [`ServeError`] when the server cannot bind or the accept loop fails.
pub fn serve_http_api<E>(config: ApiServerConfig<E>) -> Result<(), ServeError>
where
    E: LocalExecutor,
{
    let bind_addr = format!("0.0.0.0:{}", config.port);
    let listener = TcpListener::bind(&bind_addr)
        .map_err(|e| ServeError::BindFailed(format!("{bind_addr}: {e}")))?;

    let local_addr = listener
        .local_addr()
        .map_err(|e| ServeError::BindFailed(format!("could not read local address: {e}")))?;

    if config.allow_unauthenticated {
        eprintln!(
            "WARNING: --allow-unauthenticated is set. Any caller on any network interface may \
             invoke this API without credentials. Do not use in production."
        );
    }

    eprintln!(
        "traverse-cli serve: HTTP/JSON API listening on http://{local_addr} (spec 033-http-json-api)"
    );
    let _ = std::io::stderr().flush();

    // Build the runtime once; `execute` takes &self so it can be shared across connections.
    let runtime = Runtime::new(config.capability_registry, config.executor)
        .with_workflow_registry(config.workflow_registry);

    for connection in listener.incoming() {
        match connection {
            Ok(stream) => {
                if let Err(e) = handle_connection(stream, config.allow_unauthenticated, &runtime) {
                    eprintln!("traverse-cli serve: connection error: {e}");
                }
            }
            Err(e) => return Err(ServeError::AcceptFailed(e.to_string())),
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Connection handler
// ---------------------------------------------------------------------------

fn handle_connection<E: LocalExecutor>(
    mut stream: TcpStream,
    allow_unauthenticated: bool,
    runtime: &Runtime<E>,
) -> Result<(), String> {
    let request = read_http_request(&mut stream)?;

    let peer_ip = stream
        .peer_addr()
        .map(|a| a.ip())
        .unwrap_or(IpAddr::from([127, 0, 0, 1]));

    if !allow_unauthenticated && !peer_ip.is_loopback() {
        let has_bearer = request
            .headers
            .get("authorization")
            .map(|v| v.starts_with("Bearer "))
            .unwrap_or(false);

        if !has_bearer {
            return write_json(
                &mut stream,
                401,
                "Unauthorized",
                &error_envelope("unauthorized", "Bearer token required"),
            );
        }
    }

    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/v1/health") => handle_health(&mut stream),
        ("GET", "/v1/capabilities") => handle_list_capabilities(&mut stream, runtime),
        ("POST", "/v1/capabilities/execute") => handle_execute(&mut stream, &request.body, runtime),
        _ => write_json(
            &mut stream,
            404,
            "Not Found",
            &error_envelope("not_found", "route not found"),
        ),
    }
}

// ---------------------------------------------------------------------------
// Route handlers (pub(crate) so tests can call them directly)
// ---------------------------------------------------------------------------

pub(crate) fn handle_health<W: Write>(w: &mut W) -> Result<(), String> {
    write_json(w, 200, "OK", &json!({"status": "ok"}))
}

pub(crate) fn handle_list_capabilities<W: Write, E: LocalExecutor>(
    w: &mut W,
    runtime: &Runtime<E>,
) -> Result<(), String> {
    let entries = runtime
        .capability_registry()
        .discover(LookupScope::PreferPrivate, &DiscoveryQuery::default());
    let json_entries: Vec<Value> = entries
        .iter()
        .map(|e| {
            json!({
                "id": e.id,
                "version": e.version,
                "scope": format!("{:?}", e.scope).to_lowercase(),
                "lifecycle": format!("{:?}", e.lifecycle).to_lowercase(),
                "implementation_kind": format!("{:?}", e.implementation_kind).to_lowercase(),
                "summary": e.summary,
                "tags": e.tags,
            })
        })
        .collect();
    write_json(w, 200, "OK", &Value::Array(json_entries))
}

pub(crate) fn handle_execute<W: Write, E: LocalExecutor>(
    w: &mut W,
    body: &[u8],
    runtime: &Runtime<E>,
) -> Result<(), String> {
    let body_str = match std::str::from_utf8(body) {
        Ok(s) => s,
        Err(e) => {
            return write_json(
                w,
                400,
                "Bad Request",
                &error_envelope(
                    "invalid_request",
                    &format!("request body is not valid UTF-8: {e}"),
                ),
            );
        }
    };

    let request: RuntimeRequest = match parse_runtime_request(body_str) {
        Ok(r) => r,
        Err(e) => {
            return write_json(
                w,
                400,
                "Bad Request",
                &error_envelope(
                    "invalid_request",
                    &format!("failed to parse RuntimeRequest: {e}"),
                ),
            );
        }
    };

    let outcome: RuntimeExecutionOutcome = runtime.execute(request);

    match serialize_outcome(&outcome) {
        Ok(body_str) => write_json_raw(w, 200, "OK", &body_str),
        Err(e) => write_json(
            w,
            500,
            "Internal Server Error",
            &error_envelope("internal_error", &e),
        ),
    }
}

// ---------------------------------------------------------------------------
// Serialization helpers
// ---------------------------------------------------------------------------

fn serialize_outcome(outcome: &RuntimeExecutionOutcome) -> Result<String, String> {
    let trace_value = serde_json::to_value(&outcome.trace)
        .map_err(|e| format!("failed to serialize trace: {e}"))?;

    let status = if outcome.result.status == RuntimeResultStatus::Error {
        "error"
    } else {
        "completed"
    };

    let response = json!({
        "status": status,
        "request_id": outcome.result.request_id,
        "execution_id": outcome.result.execution_id,
        "trace_ref": outcome.result.trace_ref,
        "output": outcome.result.output,
        "error": outcome.result.error.as_ref().map(|e| json!({
            "code": format!("{:?}", e.code).to_lowercase(),
            "message": e.message,
        })),
        "trace": trace_value,
    });

    serde_json::to_string(&response).map_err(|e| format!("failed to serialize outcome: {e}"))
}

pub(crate) fn error_envelope(code: &str, message: &str) -> Value {
    json!({"error": {"code": code, "message": message}})
}

// ---------------------------------------------------------------------------
// Raw HTTP helpers (same pattern as browser_adapter.rs)
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct HttpRequest {
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) headers: HashMap<String, String>,
    pub(crate) body: Vec<u8>,
}

fn read_http_request(stream: &mut TcpStream) -> Result<HttpRequest, String> {
    let mut buffer = Vec::new();
    let mut header_end = None;

    loop {
        let mut chunk = [0_u8; 1024];
        let n = stream
            .read(&mut chunk)
            .map_err(|e| format!("failed to read HTTP request: {e}"))?;
        if n == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..n]);
        if let Some(idx) = find_header_end(&buffer) {
            header_end = Some(idx);
            break;
        }
        if buffer.len() > MAX_REQUEST_BODY {
            return Err("HTTP request headers too large".to_string());
        }
    }

    let header_end = header_end
        .ok_or_else(|| "HTTP request missing \\r\\n\\r\\n header terminator".to_string())?;

    let headers_text = String::from_utf8(buffer[..header_end].to_vec())
        .map_err(|e| format!("HTTP request headers not valid UTF-8: {e}"))?;

    let mut lines = headers_text.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| "HTTP request missing request line".to_string())?;

    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| "HTTP request missing method".to_string())?
        .to_string();
    let path = parts
        .next()
        .ok_or_else(|| "HTTP request missing path".to_string())?
        .to_string();

    let mut headers = HashMap::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }

    let content_length = headers
        .get("content-length")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);

    if content_length > MAX_REQUEST_BODY {
        return Err(format!(
            "HTTP request body too large ({content_length} bytes, max {MAX_REQUEST_BODY})"
        ));
    }

    let mut body = buffer[header_end + 4..].to_vec();
    while body.len() < content_length {
        let mut chunk = vec![0_u8; content_length - body.len()];
        let n = stream
            .read(&mut chunk)
            .map_err(|e| format!("failed to read HTTP request body: {e}"))?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
    }
    body.truncate(content_length);

    Ok(HttpRequest {
        method,
        path,
        headers,
        body,
    })
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|w| w == b"\r\n\r\n")
}

fn write_json<W: Write>(w: &mut W, status: u16, reason: &str, body: &Value) -> Result<(), String> {
    let bytes =
        serde_json::to_vec(body).map_err(|e| format!("failed to serialize response: {e}"))?;
    write_raw(w, status, reason, "application/json", &bytes)
}

fn write_json_raw<W: Write>(
    w: &mut W,
    status: u16,
    reason: &str,
    body: &str,
) -> Result<(), String> {
    write_raw(w, status, reason, "application/json", body.as_bytes())
}

fn write_raw<W: Write>(
    w: &mut W,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &[u8],
) -> Result<(), String> {
    let header = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    w.write_all(header.as_bytes())
        .map_err(|e| format!("failed to write HTTP response header: {e}"))?;
    w.write_all(body)
        .map_err(|e| format!("failed to write HTTP response body: {e}"))?;
    w.flush()
        .map_err(|e| format!("failed to flush HTTP response: {e}"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;
    use serde_json::Value;
    use traverse_contracts::{
        BinaryFormat as ContractBinaryFormat, CapabilityContract, Entrypoint, EntrypointKind,
        Execution, ExecutionConstraints, ExecutionTarget, FilesystemAccess, HostApiAccess,
        Lifecycle, NetworkAccess, Owner, Provenance, ProvenanceSource, SchemaContainer,
        ServiceType, SideEffect, SideEffectKind,
    };
    use traverse_registry::ResolvedCapability;
    use traverse_registry::{
        ArtifactDigests, BinaryFormat, BinaryReference, CapabilityArtifactRecord,
        CapabilityRegistration, ComposabilityMetadata, CompositionKind, CompositionPattern,
        ImplementationKind, RegistryProvenance, RegistryScope, SourceKind, SourceReference,
    };
    use traverse_runtime::{LocalExecutionFailure, LocalExecutionFailureCode};

    // ------------------------------------------------------------------
    // Minimal test executor
    // ------------------------------------------------------------------

    #[derive(Clone)]
    struct TestExecutor {
        result: Result<Value, String>,
    }

    impl TestExecutor {
        fn ok(value: Value) -> Self {
            Self { result: Ok(value) }
        }
    }

    impl LocalExecutor for TestExecutor {
        fn execute(
            &self,
            _capability: &ResolvedCapability,
            _input: &Value,
        ) -> Result<Value, LocalExecutionFailure> {
            self.result.clone().map_err(|msg| LocalExecutionFailure {
                code: LocalExecutionFailureCode::ExecutionFailed,
                message: msg,
            })
        }
    }

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    fn test_contract(id: &str, version: &str) -> CapabilityContract {
        let dot = id.rfind('.').unwrap_or(0);
        let namespace = id[..dot].to_string();
        let name = id[dot + 1..].to_string();
        CapabilityContract {
            kind: "capability_contract".to_string(),
            schema_version: "1.0.0".to_string(),
            id: id.to_string(),
            namespace,
            name,
            version: version.to_string(),
            lifecycle: Lifecycle::Active,
            owner: Owner {
                team: "test-team".to_string(),
                contact: "test@example.com".to_string(),
            },
            summary: "test capability".to_string(),
            description: "test capability for http_api unit tests".to_string(),
            inputs: SchemaContainer {
                schema: json!({"type": "object"}),
            },
            outputs: SchemaContainer {
                schema: json!({"type": "object"}),
            },
            preconditions: vec![],
            postconditions: vec![],
            side_effects: vec![SideEffect {
                kind: SideEffectKind::MemoryOnly,
                description: "none".to_string(),
            }],
            emits: vec![],
            consumes: vec![],
            permissions: vec![],
            execution: Execution {
                binary_format: ContractBinaryFormat::Wasm,
                entrypoint: Entrypoint {
                    kind: EntrypointKind::WasiCommand,
                    command: "run".to_string(),
                },
                preferred_targets: vec![ExecutionTarget::Local],
                constraints: ExecutionConstraints {
                    host_api_access: HostApiAccess::None,
                    network_access: NetworkAccess::Forbidden,
                    filesystem_access: FilesystemAccess::None,
                },
            },
            policies: vec![],
            dependencies: vec![],
            provenance: Provenance {
                source: ProvenanceSource::Greenfield,
                author: "test".to_string(),
                created_at: "2026-01-01T00:00:00Z".to_string(),
                spec_ref: None,
                adr_refs: vec![],
                exception_refs: vec![],
            },
            evidence: vec![],
            service_type: ServiceType::Stateless,
            permitted_targets: vec![ExecutionTarget::Local],
            event_trigger: None,
        }
    }

    fn test_registration(id: &str, version: &str) -> CapabilityRegistration {
        let contract = test_contract(id, version);
        CapabilityRegistration {
            scope: RegistryScope::Private,
            contract_path: format!("test/{id}/{version}/contract.json"),
            artifact: CapabilityArtifactRecord {
                artifact_ref: format!("test:{id}:{version}"),
                implementation_kind: ImplementationKind::Executable,
                source: SourceReference {
                    kind: SourceKind::Local,
                    location: format!("test/{id}/module.wasm"),
                },
                binary: Some(BinaryReference {
                    format: BinaryFormat::Wasm,
                    location: format!("test/{id}/module.wasm"),
                }),
                workflow_ref: None,
                digests: ArtifactDigests {
                    source_digest: "sha256:test".to_string(),
                    binary_digest: Some("sha256:test-bin".to_string()),
                },
                provenance: RegistryProvenance {
                    source: "greenfield".to_string(),
                    author: "test".to_string(),
                    created_at: "2026-01-01T00:00:00Z".to_string(),
                },
            },
            registered_at: "test-bundle@1.0.0".to_string(),
            tags: vec![],
            composability: ComposabilityMetadata {
                kind: CompositionKind::Atomic,
                patterns: vec![CompositionPattern::Sequential],
                provides: vec![id.to_string()],
                requires: vec![],
            },
            governing_spec: "005-capability-registry".to_string(),
            validator_version: "0.2.0".to_string(),
            contract,
        }
    }

    fn test_runtime_with(id: &str, version: &str) -> Runtime<TestExecutor> {
        let mut registry = CapabilityRegistry::new();
        registry
            .register(test_registration(id, version))
            .expect("test registration must succeed");
        Runtime::new(registry, TestExecutor::ok(json!({"result": "ok"})))
    }

    fn empty_runtime() -> Runtime<TestExecutor> {
        Runtime::new(CapabilityRegistry::new(), TestExecutor::ok(json!({})))
    }

    fn make_runtime_request_body(capability_id: &str) -> Vec<u8> {
        json!({
            "kind": "runtime_request",
            "schema_version": "1.0.0",
            "request_id": "test-req-001",
            "intent": {
                "capability_id": capability_id,
                "capability_version": "1.0.0"
            },
            "input": {},
            "lookup": {
                "scope": "prefer_private",
                "allow_ambiguity": false
            },
            "context": {
                "requested_target": "local"
            },
            "governing_spec": "006-runtime-request-execution"
        })
        .to_string()
        .into_bytes()
    }

    fn parse_response_body(response: &[u8]) -> Value {
        let pos = response
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .expect("response must contain \\r\\n\\r\\n");
        serde_json::from_slice(&response[pos + 4..]).expect("response body must be valid JSON")
    }

    fn response_status(response: &[u8]) -> u16 {
        let text = std::str::from_utf8(response).expect("response must be UTF-8");
        let line = text
            .lines()
            .next()
            .expect("response must have a first line");
        let mut parts = line.splitn(3, ' ');
        parts.next();
        parts
            .next()
            .expect("status code must be present")
            .parse()
            .expect("status code must be numeric")
    }

    // ------------------------------------------------------------------
    // health endpoint
    // ------------------------------------------------------------------

    #[test]
    fn health_endpoint_returns_status_ok() {
        let mut out = Vec::new();
        handle_health(&mut out).expect("health must succeed");

        assert_eq!(response_status(&out), 200);
        assert_eq!(parse_response_body(&out)["status"], "ok");
    }

    // ------------------------------------------------------------------
    // capabilities list endpoint
    // ------------------------------------------------------------------

    #[test]
    fn capabilities_endpoint_returns_registered_capability() {
        let runtime = test_runtime_with("test.api.do-something", "1.0.0");
        let mut out = Vec::new();
        handle_list_capabilities(&mut out, &runtime).expect("list must succeed");

        let status = response_status(&out);
        let body = parse_response_body(&out);

        assert_eq!(status, 200);
        assert!(body.is_array());
        let arr = body.as_array().expect("body must be array");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["id"], "test.api.do-something");
        assert_eq!(arr[0]["version"], "1.0.0");
    }

    #[test]
    fn capabilities_endpoint_returns_empty_array_for_empty_registry() {
        let runtime = empty_runtime();
        let mut out = Vec::new();
        handle_list_capabilities(&mut out, &runtime).expect("list must succeed");

        let body = parse_response_body(&out);
        assert!(body.is_array());
        assert!(body.as_array().expect("array").is_empty());
    }

    // ------------------------------------------------------------------
    // execute endpoint — success
    // ------------------------------------------------------------------

    #[test]
    fn execute_endpoint_returns_completed_trace_on_success() {
        let runtime = test_runtime_with("test.api.do-something", "1.0.0");
        let body = make_runtime_request_body("test.api.do-something");

        let mut out = Vec::new();
        handle_execute(&mut out, &body, &runtime).expect("execute must write a response");

        let status = response_status(&out);
        let resp = parse_response_body(&out);

        assert_eq!(status, 200);
        assert_eq!(resp["status"], "completed");
        assert!(resp["trace"].is_object(), "trace must be an object");
        assert_eq!(resp["request_id"], "test-req-001");
    }

    // ------------------------------------------------------------------
    // execute endpoint — unknown capability
    // ------------------------------------------------------------------

    #[test]
    fn execute_endpoint_returns_error_status_for_unknown_capability() {
        let runtime = empty_runtime();
        let body = make_runtime_request_body("unknown.capability.does-not-exist");

        let mut out = Vec::new();
        handle_execute(&mut out, &body, &runtime)
            .expect("handle_execute must write a response even on runtime error");

        let status = response_status(&out);
        let resp = parse_response_body(&out);

        assert_eq!(status, 200);
        assert_eq!(resp["status"], "error");
    }

    // ------------------------------------------------------------------
    // execute endpoint — invalid body
    // ------------------------------------------------------------------

    #[test]
    fn execute_endpoint_rejects_malformed_json_body() {
        let runtime = empty_runtime();

        let mut out = Vec::new();
        handle_execute(&mut out, b"{not valid json", &runtime)
            .expect("handle_execute must write a response");

        let status = response_status(&out);
        let resp = parse_response_body(&out);

        assert_eq!(status, 400);
        assert!(resp["error"]["code"].as_str().is_some());
        assert!(resp["error"]["message"].as_str().is_some());
    }

    // ------------------------------------------------------------------
    // auth helpers — loopback detection via std
    // ------------------------------------------------------------------

    #[test]
    fn loopback_ipv4_is_recognized() {
        let ip: IpAddr = "127.0.0.1".parse().expect("valid IP");
        assert!(ip.is_loopback());
    }

    #[test]
    fn loopback_ipv6_is_recognized() {
        let ip: IpAddr = "::1".parse().expect("valid IP");
        assert!(ip.is_loopback());
    }

    #[test]
    fn non_loopback_ip_is_not_loopback() {
        let ip: IpAddr = "192.168.1.100".parse().expect("valid IP");
        assert!(!ip.is_loopback());
    }

    // ------------------------------------------------------------------
    // error envelope shape
    // ------------------------------------------------------------------

    #[test]
    fn error_envelope_has_correct_json_shape() {
        let env = error_envelope("unauthorized", "Bearer token required");
        assert_eq!(env["error"]["code"], "unauthorized");
        assert_eq!(env["error"]["message"], "Bearer token required");
    }
}

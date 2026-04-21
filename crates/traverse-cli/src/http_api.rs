use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{IpAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use traverse_contracts::{CapabilityContract, parse_contract};
use traverse_registry::{
    ArtifactDigests, BinaryFormat, BinaryReference, CapabilityArtifactRecord,
    CapabilityRegistration, CapabilityRegistry, ComposabilityMetadata, CompositionKind,
    CompositionPattern, DiscoveryQuery, ImplementationKind, LookupScope, RegistryProvenance,
    RegistryScope, SourceKind, SourceReference, WorkflowRegistry,
};
use traverse_runtime::{
    LocalExecutor, Runtime, RuntimeExecutionOutcome, RuntimeRequest, RuntimeResultStatus,
    parse_runtime_request,
};

const MAX_REQUEST_BODY: usize = 4 * 1024 * 1024; // 4 MiB
const DEFAULT_WORKSPACE_ID: &str = "system";
const PERSISTED_REGISTRY_SCHEMA_VERSION: &str = "1.0.0";

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
    pub registry_root: PathBuf,
    pub executor: E,
}

struct ApiState<E> {
    allow_unauthenticated: bool,
    registry_root: PathBuf,
    executor: E,
    workspaces: RefCell<HashMap<String, WorkspaceState<E>>>,
}

struct WorkspaceState<E> {
    runtime: traverse_runtime::Runtime<E>,
    persisted: PersistedWorkspaceRegistryV1,
    loaded_from_disk: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedWorkspaceRegistryV1 {
    schema_version: String,
    registrations: Vec<PersistedCapabilityRegistrationV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedCapabilityRegistrationV1 {
    registry_scope: String,
    contract: CapabilityContract,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RegistrationScope {
    WorkspacePersisted,
    SessionEphemeral,
}

#[derive(Debug, Clone)]
struct ApiError {
    status: u16,
    reason: &'static str,
    code: &'static str,
    message: String,
}

/// Start the HTTP/JSON API server, blocking until the listener fails.
///
/// # Errors
///
/// Returns [`ServeError`] when the server cannot bind or the accept loop fails.
pub fn serve_http_api<E>(config: ApiServerConfig<E>) -> Result<(), ServeError>
where
    E: LocalExecutor + Clone,
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

    let mut workspaces = HashMap::new();
    workspaces.insert(
        DEFAULT_WORKSPACE_ID.to_string(),
        WorkspaceState {
            runtime: Runtime::new(config.capability_registry, config.executor.clone())
                .with_workflow_registry(config.workflow_registry),
            persisted: PersistedWorkspaceRegistryV1 {
                schema_version: PERSISTED_REGISTRY_SCHEMA_VERSION.to_string(),
                registrations: Vec::new(),
            },
            loaded_from_disk: true,
        },
    );

    let state = ApiState {
        allow_unauthenticated: config.allow_unauthenticated,
        registry_root: config.registry_root,
        executor: config.executor,
        workspaces: RefCell::new(workspaces),
    };

    for connection in listener.incoming() {
        match connection {
            Ok(stream) => {
                if let Err(e) = handle_connection(stream, &state) {
                    eprintln!("traverse-cli serve: connection error: {e}");
                }
            }
            Err(e) => return Err(ServeError::AcceptFailed(e.to_string())),
        }
    }

    Ok(())
}

impl<E> ApiState<E>
where
    E: LocalExecutor + Clone,
{
    fn with_workspace_mut<R>(
        &self,
        workspace_id: &str,
        f: impl FnOnce(&mut WorkspaceState<E>) -> Result<R, String>,
    ) -> Result<R, String> {
        let mut workspaces = self.workspaces.borrow_mut();
        let entry = workspaces
            .entry(workspace_id.to_string())
            .or_insert_with(|| WorkspaceState {
                runtime: Runtime::new(CapabilityRegistry::new(), self.executor.clone())
                    .with_workflow_registry(WorkflowRegistry::new()),
                persisted: PersistedWorkspaceRegistryV1 {
                    schema_version: PERSISTED_REGISTRY_SCHEMA_VERSION.to_string(),
                    registrations: Vec::new(),
                },
                loaded_from_disk: false,
            });

        if !entry.loaded_from_disk {
            entry.persisted = load_persisted_registry(&self.registry_root, workspace_id)?;
            for persisted in entry.persisted.registrations.clone() {
                let registration = derive_registration(workspace_id, &persisted).map_err(|e| {
                    format!("persisted registry contains invalid entry: {}", e.message)
                })?;
                let _ = entry
                    .runtime
                    .register_capability(registration)
                    .map_err(render_registry_failure_as_string)?;
            }
            entry.loaded_from_disk = true;
        }

        f(entry)
    }
}

fn load_persisted_registry(
    registry_root: &Path,
    workspace_id: &str,
) -> Result<PersistedWorkspaceRegistryV1, String> {
    let path = persisted_registry_path(registry_root, workspace_id);
    if !path.exists() {
        return Ok(PersistedWorkspaceRegistryV1 {
            schema_version: PERSISTED_REGISTRY_SCHEMA_VERSION.to_string(),
            registrations: Vec::new(),
        });
    }

    let bytes =
        std::fs::read(&path).map_err(|e| format!("failed to read persisted registry: {e}"))?;
    let persisted: PersistedWorkspaceRegistryV1 = serde_json::from_slice(&bytes).map_err(|e| {
        format!(
            "failed to parse persisted registry at {}: {e}",
            path.display()
        )
    })?;
    Ok(persisted)
}

fn persisted_registry_path(registry_root: &Path, workspace_id: &str) -> PathBuf {
    registry_root
        .join("workspaces")
        .join(workspace_id)
        .join("capabilities.json")
}

fn persist_registry(
    registry_root: &Path,
    workspace_id: &str,
    persisted: &PersistedWorkspaceRegistryV1,
) -> Result<(), String> {
    let path = persisted_registry_path(registry_root, workspace_id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create persisted registry directory: {e}"))?;
    }

    let bytes = serde_json::to_vec_pretty(persisted)
        .map_err(|e| format!("failed to serialize persisted registry: {e}"))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, &bytes)
        .map_err(|e| format!("failed to write persisted registry temp file: {e}"))?;
    std::fs::rename(&tmp, &path)
        .map_err(|e| format!("failed to atomically replace persisted registry: {e}"))?;
    Ok(())
}

fn render_registry_failure_as_string(failure: traverse_registry::RegistryFailure) -> String {
    use std::fmt::Write as _;

    let mut rendered = String::new();
    for err in failure.errors {
        let _ = write!(
            &mut rendered,
            "{:?} at {}: {}; ",
            err.code, err.target, err.message
        );
    }
    rendered
}

fn validate_workspace_id(workspace_id: &str) -> Result<(), String> {
    if workspace_id.trim().is_empty() {
        return Err("workspace_id must be non-empty".to_string());
    }
    if workspace_id.len() > 64 {
        return Err("workspace_id must be at most 64 characters".to_string());
    }
    if workspace_id.contains('\0') {
        return Err("workspace_id must not contain null bytes".to_string());
    }

    // Conservative allowlist: avoids path traversal and injection into on-disk layout.
    for ch in workspace_id.chars() {
        let ok = ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.');
        if !ok {
            return Err(
                "workspace_id may contain only ASCII letters, digits, '-', '_', and '.'"
                    .to_string(),
            );
        }
    }
    Ok(())
}

fn parse_registration_scope(value: Option<&Value>) -> Result<RegistrationScope, String> {
    let Some(value) = value else {
        return Ok(RegistrationScope::WorkspacePersisted);
    };
    let Some(scope) = value.as_str() else {
        return Err("scope must be a string".to_string());
    };
    match scope {
        "workspace_persisted" => Ok(RegistrationScope::WorkspacePersisted),
        "session_ephemeral" => Ok(RegistrationScope::SessionEphemeral),
        _ => Err("scope must be workspace_persisted or session_ephemeral".to_string()),
    }
}

fn map_registry_failure_http(
    failure: &traverse_registry::RegistryFailure,
) -> (u16, &'static str, &'static str) {
    use traverse_registry::RegistryErrorCode;

    let mut has_immutable = false;
    for err in &failure.errors {
        if err.code == RegistryErrorCode::ImmutableVersionConflict {
            has_immutable = true;
        }
    }

    if has_immutable {
        return (409, "immutable_version_conflict", "Conflict");
    }

    (422, "registration_failed", "Unprocessable Entity")
}

fn derive_registration(
    workspace_id: &str,
    persisted: &PersistedCapabilityRegistrationV1,
) -> Result<CapabilityRegistration, ApiError> {
    let registry_scope = match persisted.registry_scope.as_str() {
        "public" => RegistryScope::Public,
        "private" => RegistryScope::Private,
        other => {
            return Err(ApiError {
                status: 422,
                reason: "Unprocessable Entity",
                code: "invalid_registry_scope",
                message: format!("registry_scope must be public or private (got {other})"),
            });
        }
    };

    let contract = persisted.contract.clone();
    let entrypoint = contract.execution.entrypoint.command.clone();
    let binary_path = PathBuf::from(&entrypoint);
    if !binary_path.exists() {
        return Err(ApiError {
            status: 422,
            reason: "Unprocessable Entity",
            code: "artifact_not_found",
            message: format!("binary artifact not found at {entrypoint}"),
        });
    }

    let artifact_ref = format!(
        "workspace:{workspace_id}:{}:{}",
        contract.id, contract.version
    );
    let source_digest = format!("sha256:source-{}-{}", contract.id, contract.version);
    let binary_digest = format!("sha256:binary-{}-{}", contract.id, contract.version);

    Ok(CapabilityRegistration {
        scope: registry_scope,
        contract_path: format!(
            "workspaces/{workspace_id}/registry/{}/{}@{}/contract.json",
            format!("{registry_scope:?}").to_lowercase(),
            contract.id,
            contract.version
        ),
        contract,
        artifact: CapabilityArtifactRecord {
            artifact_ref,
            implementation_kind: ImplementationKind::Executable,
            source: SourceReference {
                kind: SourceKind::Local,
                location: entrypoint.clone(),
            },
            binary: Some(BinaryReference {
                format: BinaryFormat::Wasm,
                location: entrypoint,
            }),
            workflow_ref: None,
            digests: ArtifactDigests {
                source_digest,
                binary_digest: Some(binary_digest),
            },
            provenance: RegistryProvenance {
                source: "programmatic_registration".to_string(),
                author: persisted.contract.provenance.author.clone(),
                created_at: persisted.contract.provenance.created_at.clone(),
            },
        },
        registered_at: persisted.contract.provenance.created_at.clone(),
        tags: persisted.tags.clone(),
        composability: ComposabilityMetadata {
            kind: CompositionKind::Atomic,
            patterns: vec![CompositionPattern::Sequential],
            provides: Vec::new(),
            requires: Vec::new(),
        },
        governing_spec: "034-programmatic-registration".to_string(),
        validator_version: "traverse-cli".to_string(),
    })
}

fn parse_register_body(
    body: &[u8],
) -> Result<(String, RegistrationScope, PersistedCapabilityRegistrationV1), ApiError> {
    let body_str = std::str::from_utf8(body).map_err(|e| ApiError {
        status: 400,
        reason: "Bad Request",
        code: "invalid_request",
        message: format!("request body is not valid UTF-8: {e}"),
    })?;

    let value: Value = serde_json::from_str(body_str).map_err(|e| ApiError {
        status: 400,
        reason: "Bad Request",
        code: "invalid_request",
        message: format!("invalid JSON body: {e}"),
    })?;

    let workspace_id = value
        .get("workspace_id")
        .and_then(|v| v.as_str())
        .filter(|ws| !ws.trim().is_empty())
        .ok_or_else(|| ApiError {
            status: 400,
            reason: "Bad Request",
            code: "missing_workspace_id",
            message: "workspace_id is required".to_string(),
        })?
        .to_string();

    validate_workspace_id(&workspace_id).map_err(|msg| ApiError {
        status: 400,
        reason: "Bad Request",
        code: "invalid_workspace_id",
        message: msg,
    })?;

    let scope = parse_registration_scope(value.get("scope")).map_err(|msg| ApiError {
        status: 422,
        reason: "Unprocessable Entity",
        code: "invalid_scope",
        message: msg,
    })?;

    let contract_value = if value
        .get("kind")
        .and_then(|v| v.as_str())
        .is_some_and(|k| k == "capability_contract")
    {
        value.clone()
    } else if let Some(contract) = value.get("contract") {
        contract.clone()
    } else {
        return Err(ApiError {
            status: 422,
            reason: "Unprocessable Entity",
            code: "invalid_contract",
            message: "expected body to be a capability contract or to contain a `contract` field"
                .to_string(),
        });
    };

    let contract_json = serde_json::to_string(&contract_value).map_err(|e| ApiError {
        status: 422,
        reason: "Unprocessable Entity",
        code: "invalid_contract",
        message: format!("failed to serialize contract: {e}"),
    })?;

    let contract: CapabilityContract =
        parse_contract(&contract_json).map_err(|failure| ApiError {
            status: 422,
            reason: "Unprocessable Entity",
            code: "contract_validation_failed",
            message: format!("contract could not be parsed: {failure:?}"),
        })?;

    let registry_scope = value
        .get("registry_scope")
        .and_then(|v| v.as_str())
        .unwrap_or("private")
        .to_string();

    let tags = value
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default();

    Ok((
        workspace_id,
        scope,
        PersistedCapabilityRegistrationV1 {
            registry_scope,
            contract,
            tags,
        },
    ))
}

fn ensure_workspace_loaded<E: LocalExecutor + Clone>(
    state: &ApiState<E>,
    workspace_id: &str,
    ws: &mut WorkspaceState<E>,
) -> Result<(), String> {
    if ws.loaded_from_disk {
        return Ok(());
    }

    ws.persisted = load_persisted_registry(&state.registry_root, workspace_id)?;
    for persisted in ws.persisted.registrations.clone() {
        let registration = derive_registration(workspace_id, &persisted)
            .map_err(|e| format!("persisted registry contains invalid entry: {}", e.message))?;
        let _ = ws
            .runtime
            .register_capability(registration)
            .map_err(render_registry_failure_as_string)?;
    }
    ws.loaded_from_disk = true;
    Ok(())
}

fn apply_registration<E: LocalExecutor + Clone>(
    state: &ApiState<E>,
    workspace_id: &str,
    scope: RegistrationScope,
    persisted_registration: PersistedCapabilityRegistrationV1,
    registration: CapabilityRegistration,
) -> Result<
    Result<traverse_registry::RegistrationOutcome, traverse_registry::RegistryFailure>,
    String,
> {
    let mut workspaces = state.workspaces.borrow_mut();
    let ws = workspaces
        .entry(workspace_id.to_string())
        .or_insert_with(|| WorkspaceState {
            runtime: Runtime::new(CapabilityRegistry::new(), state.executor.clone())
                .with_workflow_registry(WorkflowRegistry::new()),
            persisted: PersistedWorkspaceRegistryV1 {
                schema_version: PERSISTED_REGISTRY_SCHEMA_VERSION.to_string(),
                registrations: Vec::new(),
            },
            loaded_from_disk: false,
        });

    ensure_workspace_loaded(state, workspace_id, ws)?;

    match ws.runtime.register_capability(registration) {
        Ok(outcome) => {
            if scope == RegistrationScope::WorkspacePersisted && !outcome.already_registered {
                ws.persisted.registrations.push(persisted_registration);
                persist_registry(&state.registry_root, workspace_id, &ws.persisted)?;
            }
            Ok(Ok(outcome))
        }
        Err(failure) => Ok(Err(failure)),
    }
}

// ---------------------------------------------------------------------------
// Connection handler
// ---------------------------------------------------------------------------

fn handle_connection<E: LocalExecutor + Clone>(
    mut stream: TcpStream,
    state: &ApiState<E>,
) -> Result<(), String> {
    let request = read_http_request(&mut stream)?;

    let peer_ip = stream
        .peer_addr()
        .map(|a| a.ip())
        .unwrap_or(IpAddr::from([127, 0, 0, 1]));

    if !state.allow_unauthenticated && !peer_ip.is_loopback() {
        let has_bearer = request
            .headers
            .get("authorization")
            .is_some_and(|v| v.starts_with("Bearer "));

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
        ("GET", "/v1/capabilities") => handle_list_capabilities(&mut stream, &request, state),
        ("POST", "/v1/capabilities/register") => {
            handle_register_capability(&mut stream, &request.body, state)
        }
        ("POST", "/v1/capabilities/execute") => handle_execute(&mut stream, &request, state),
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

fn handle_health<W: Write>(w: &mut W) -> Result<(), String> {
    write_json(w, 200, "OK", &json!({"status": "ok"}))
}

fn handle_list_capabilities<W: Write, E: LocalExecutor + Clone>(
    w: &mut W,
    request: &HttpRequest,
    state: &ApiState<E>,
) -> Result<(), String> {
    let workspace_id = request
        .query
        .get("workspace_id")
        .map_or(DEFAULT_WORKSPACE_ID, String::as_str);

    let entries = state.with_workspace_mut(workspace_id, |ws| {
        Ok(ws
            .runtime
            .capability_registry()
            .discover(LookupScope::PreferPrivate, &DiscoveryQuery::default()))
    })?;

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

fn handle_register_capability<W: Write, E: LocalExecutor + Clone>(
    w: &mut W,
    body: &[u8],
    state: &ApiState<E>,
) -> Result<(), String> {
    let (workspace_id, scope, persisted_registration) = match parse_register_body(body) {
        Ok(parsed) => parsed,
        Err(err) => {
            return write_json(
                w,
                err.status,
                err.reason,
                &error_envelope(err.code, &err.message),
            );
        }
    };

    let registration = match derive_registration(&workspace_id, &persisted_registration) {
        Ok(registration) => registration,
        Err(err) => {
            return write_json(
                w,
                err.status,
                err.reason,
                &error_envelope(err.code, &err.message),
            );
        }
    };

    match apply_registration(
        state,
        &workspace_id,
        scope,
        persisted_registration,
        registration,
    )? {
        Ok(outcome) => {
            let status = if outcome.already_registered { 200 } else { 201 };
            write_json(
                w,
                status,
                if status == 200 { "OK" } else { "Created" },
                &json!({
                    "workspace_id": workspace_id,
                    "scope": match scope {
                        RegistrationScope::WorkspacePersisted => "workspace_persisted",
                        RegistrationScope::SessionEphemeral => "session_ephemeral",
                    },
                    "already_registered": outcome.already_registered,
                    "capability": {
                        "id": outcome.record.id,
                        "version": outcome.record.version,
                        "digest": outcome.record.contract_digest,
                        "registry_scope": format!("{:?}", outcome.record.scope).to_lowercase(),
                    }
                }),
            )
        }
        Err(failure) => {
            let (status, code, reason) = map_registry_failure_http(&failure);
            write_json(
                w,
                status,
                reason,
                &json!({
                    "error": {
                        "code": code,
                        "message": render_registry_failure_as_string(failure),
                    }
                }),
            )
        }
    }
}

fn handle_execute<W: Write, E: LocalExecutor + Clone>(
    w: &mut W,
    request: &HttpRequest,
    state: &ApiState<E>,
) -> Result<(), String> {
    let body = request.body.as_slice();
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

    let runtime_request: RuntimeRequest = match parse_runtime_request(body_str) {
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

    let workspace_id = request
        .query
        .get("workspace_id")
        .map_or(DEFAULT_WORKSPACE_ID, String::as_str);

    let outcome: RuntimeExecutionOutcome =
        state.with_workspace_mut(workspace_id, |ws| Ok(ws.runtime.execute(runtime_request)))?;

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
    pub(crate) query: HashMap<String, String>,
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
    let raw_path = parts
        .next()
        .ok_or_else(|| "HTTP request missing path".to_string())?
        .to_string();
    let (path, query) = parse_path_and_query(&raw_path);

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
        query,
        headers,
        body,
    })
}

fn parse_path_and_query(raw_path: &str) -> (String, HashMap<String, String>) {
    let (path, query) = match raw_path.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (raw_path, None),
    };

    let mut params = HashMap::new();
    if let Some(query) = query {
        for pair in query.split('&') {
            if pair.is_empty() {
                continue;
            }
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            params.insert(k.to_string(), v.to_string());
        }
    }
    (path.to_string(), params)
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

    fn test_state_with(id: &str, version: &str) -> ApiState<TestExecutor> {
        let mut registry = CapabilityRegistry::new();
        registry
            .register(test_registration(id, version))
            .expect("test registration must succeed");

        let executor = TestExecutor::ok(json!({"result": "ok"}));
        let mut workspaces = HashMap::new();
        workspaces.insert(
            DEFAULT_WORKSPACE_ID.to_string(),
            WorkspaceState {
                runtime: Runtime::new(registry, executor.clone())
                    .with_workflow_registry(WorkflowRegistry::new()),
                persisted: PersistedWorkspaceRegistryV1 {
                    schema_version: PERSISTED_REGISTRY_SCHEMA_VERSION.to_string(),
                    registrations: Vec::new(),
                },
                loaded_from_disk: true,
            },
        );

        ApiState {
            allow_unauthenticated: true,
            registry_root: std::env::temp_dir().join("traverse-cli-http-api-tests"),
            executor,
            workspaces: RefCell::new(workspaces),
        }
    }

    fn empty_state() -> ApiState<TestExecutor> {
        let executor = TestExecutor::ok(json!({}));
        let mut workspaces = HashMap::new();
        workspaces.insert(
            DEFAULT_WORKSPACE_ID.to_string(),
            WorkspaceState {
                runtime: Runtime::new(CapabilityRegistry::new(), executor.clone())
                    .with_workflow_registry(WorkflowRegistry::new()),
                persisted: PersistedWorkspaceRegistryV1 {
                    schema_version: PERSISTED_REGISTRY_SCHEMA_VERSION.to_string(),
                    registrations: Vec::new(),
                },
                loaded_from_disk: true,
            },
        );

        ApiState {
            allow_unauthenticated: true,
            registry_root: std::env::temp_dir().join("traverse-cli-http-api-tests"),
            executor,
            workspaces: RefCell::new(workspaces),
        }
    }

    fn make_http_request(method: &str, path: &str, body: Vec<u8>) -> HttpRequest {
        HttpRequest {
            method: method.to_string(),
            path: path.to_string(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body,
        }
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
        let state = test_state_with("test.api.do-something", "1.0.0");
        let req = make_http_request("GET", "/v1/capabilities", Vec::new());
        let mut out = Vec::new();
        handle_list_capabilities(&mut out, &req, &state).expect("list must succeed");

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
        let state = empty_state();
        let req = make_http_request("GET", "/v1/capabilities", Vec::new());
        let mut out = Vec::new();
        handle_list_capabilities(&mut out, &req, &state).expect("list must succeed");

        let body = parse_response_body(&out);
        assert!(body.is_array());
        assert!(body.as_array().expect("array").is_empty());
    }

    // ------------------------------------------------------------------
    // execute endpoint — success
    // ------------------------------------------------------------------

    #[test]
    fn execute_endpoint_returns_completed_trace_on_success() {
        let body = make_runtime_request_body("test.api.do-something");
        let state = test_state_with("test.api.do-something", "1.0.0");
        let req = make_http_request("POST", "/v1/capabilities/execute", body);

        let mut out = Vec::new();
        handle_execute(&mut out, &req, &state).expect("execute must write a response");

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
        let body = make_runtime_request_body("unknown.capability.does-not-exist");
        let state = empty_state();
        let req = make_http_request("POST", "/v1/capabilities/execute", body);

        let mut out = Vec::new();
        handle_execute(&mut out, &req, &state)
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
        let state = empty_state();
        let req = make_http_request(
            "POST",
            "/v1/capabilities/execute",
            b"{not valid json".to_vec(),
        );

        let mut out = Vec::new();
        handle_execute(&mut out, &req, &state).expect("handle_execute must write a response");

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

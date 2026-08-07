//! TLS gRPC transport for governed app-runtime events.
//!
//! Governed by `097-websocket-grpc-event-transport` v1.1.0 and ADR-0034.

use crate::http_api::{ApiState, open_grpc_event_source};
use serde_json::json;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio_stream::Stream;
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::{Identity, Server, ServerTlsConfig};
use tonic::{Code, Request, Response, Status};
use traverse_runtime::LocalExecutor;

pub(crate) mod proto {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/traverse.runtime.v1.rs"));
}

use proto::event_service_server::{EventService, EventServiceServer};
use proto::{Event, GovernanceMetadata, StreamEventsRequest};

const STREAM_POLL_INTERVAL: Duration = Duration::from_millis(50);

pub(crate) struct GrpcServerConfig {
    pub(crate) bind_address: String,
    pub(crate) tls_cert_path: std::path::PathBuf,
    pub(crate) tls_key_path: std::path::PathBuf,
}

fn load_tls_identity(
    tls_cert_path: &std::path::Path,
    tls_key_path: &std::path::Path,
) -> Result<(Vec<u8>, Vec<u8>), String> {
    let certificate = std::fs::read(tls_cert_path)
        .map_err(|error| format!("failed to read gRPC TLS certificate: {error}"))?;
    let private_key = std::fs::read(tls_key_path)
        .map_err(|error| format!("failed to read gRPC TLS private key: {error}"))?;
    Ok((certificate, private_key))
}

fn parse_grpc_bind_address(bind_address: &str) -> Result<SocketAddr, String> {
    bind_address
        .parse()
        .map_err(|error| format!("invalid gRPC bind address '{bind_address}': {error}"))
}

pub(crate) fn spawn_event_server<E>(
    state: Arc<ApiState<E>>,
    config: &GrpcServerConfig,
) -> Result<(), String>
where
    E: LocalExecutor + Clone + Send + Sync + 'static,
{
    let (certificate, private_key) =
        load_tls_identity(&config.tls_cert_path, &config.tls_key_path)?;
    let bind_address = parse_grpc_bind_address(&config.bind_address)?;

    std::thread::Builder::new()
        .name("traverse-grpc-events".to_string())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    eprintln!("failed to initialize gRPC runtime: {error}");
                    return;
                }
            };
            runtime.block_on(async move {
                let tls = ServerTlsConfig::new()
                    .identity(Identity::from_pem(certificate, private_key));
                let service = GrpcEventService { state };
                let result = Server::builder().tls_config(tls).map(|mut server| {
                    server
                        .add_service(EventServiceServer::new(service))
                        .serve(bind_address)
                });
                match result {
                    Ok(server) => {
                        eprintln!(
                            "traverse-cli serve: TLS gRPC EventService listening on https://{bind_address}"
                        );
                        if let Err(error) = server.await {
                            eprintln!("gRPC EventService terminated: {error}");
                        }
                    }
                    Err(error) => eprintln!("failed to configure gRPC TLS: {error}"),
                }
            });
        })
        .map_err(|error| format!("failed to start gRPC event listener: {error}"))?;
    Ok(())
}

#[derive(Clone)]
struct GrpcEventService<E> {
    state: Arc<ApiState<E>>,
}

#[tonic::async_trait]
impl<E> EventService for GrpcEventService<E>
where
    E: LocalExecutor + Clone + Send + Sync + 'static,
{
    type StreamEventsStream = Pin<Box<dyn Stream<Item = Result<Event, Status>> + Send + 'static>>;

    async fn stream_events(
        &self,
        request: Request<StreamEventsRequest>,
    ) -> Result<Response<Self::StreamEventsStream>, Status> {
        let metadata = request.metadata();
        let mut headers = HashMap::new();
        if let Some(value) = metadata.get("authorization")
            && let Ok(value) = value.to_str()
        {
            headers.insert("authorization".to_string(), value.to_string());
        }
        let subscription = request.into_inner();
        let source = open_grpc_event_source(
            self.state.as_ref(),
            &headers,
            false,
            &subscription.workspace_id,
            &subscription.app_id,
            &subscription.event_types,
            subscription.cursor.as_deref(),
        )
        .map_err(|error| {
            structured_status(
                grpc_code_for_http_status(error.status),
                error.code,
                &error.message,
            )
        })?;

        let (sender, receiver) = tokio::sync::mpsc::channel(64);
        tokio::spawn(async move {
            let mut source = source;
            for (cursor, event) in source.take_initial_events() {
                if sender
                    .send(Ok(to_proto_event(cursor, event)))
                    .await
                    .is_err()
                {
                    return;
                }
            }
            loop {
                match source.poll() {
                    Ok(events) => {
                        for (cursor, event) in events {
                            if sender
                                .send(Ok(to_proto_event(cursor, event)))
                                .await
                                .is_err()
                            {
                                return;
                            }
                        }
                    }
                    Err(message) => {
                        let _ = sender
                            .send(Err(structured_status(
                                Code::Unavailable,
                                "event_broker_unavailable",
                                &message,
                            )))
                            .await;
                        return;
                    }
                }
                tokio::time::sleep(STREAM_POLL_INTERVAL).await;
            }
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(receiver))))
    }
}

fn to_proto_event(cursor: String, event: traverse_runtime::events::TraverseEvent) -> Event {
    Event {
        specversion: "1.0".to_string(),
        id: event.id,
        source: event.source,
        r#type: event.event_type,
        time: event.time,
        data_json: event.data.to_string(),
        datacontenttype: event.datacontenttype,
        governance: Some(GovernanceMetadata {
            owner: event.owner,
            version: event.version,
            lifecycle_status: format!("{:?}", event.lifecycle_status).to_lowercase(),
            deduplication_id: event.deduplication_id,
            ordering_scope: event.ordering_scope,
            correlation_id: event.correlation_id,
            causation_id: event.causation_id,
            subject_id: event.subject_id,
            actor_id: event.actor_id,
        }),
        cursor,
    }
}

fn grpc_code_for_http_status(status: u16) -> Code {
    match status {
        400 => Code::InvalidArgument,
        401 => Code::Unauthenticated,
        403 => Code::PermissionDenied,
        503 => Code::Unavailable,
        _ => Code::Internal,
    }
}

fn structured_status(code: Code, traverse_code: &str, message: &str) -> Status {
    let details = serde_json::to_vec(&json!({
        "code": traverse_code,
        "message": message,
    }))
    .unwrap_or_else(|_| b"{\"code\":\"internal_error\"}".to_vec());
    Status::with_details(code, message, details.into())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use traverse_runtime::events::LifecycleStatus;

    #[test]
    fn grpc_event_maps_cloudevents_and_governance_fields() {
        let event = traverse_runtime::events::TraverseEvent {
            id: "event-1".to_string(),
            source: "traverse-runtime/example".to_string(),
            event_type: "dev.traverse.runtime.app.state_changed".to_string(),
            datacontenttype: "application/json".to_string(),
            time: "2026-08-07T00:00:00Z".to_string(),
            data: json!({"state": "running"}),
            owner: "traverse-runtime".to_string(),
            version: "1.0.0".to_string(),
            lifecycle_status: LifecycleStatus::Active,
            deduplication_id: Some("dedupe-1".to_string()),
            ordering_scope: Some("workspace/app".to_string()),
            correlation_id: Some("session-1".to_string()),
            causation_id: Some("execution-1".to_string()),
            subject_id: Some("workspace/app".to_string()),
            actor_id: Some("actor-1".to_string()),
        };

        let mapped = to_proto_event("42".to_string(), event);

        assert_eq!(mapped.specversion, "1.0");
        assert_eq!(mapped.r#type, "dev.traverse.runtime.app.state_changed");
        assert_eq!(mapped.cursor, "42");
        assert_eq!(mapped.data_json, r#"{"state":"running"}"#);
        assert_eq!(
            mapped
                .governance
                .as_ref()
                .map(|governance| governance.lifecycle_status.as_str()),
            Some("active")
        );
        assert_eq!(
            mapped
                .governance
                .as_ref()
                .and_then(|governance| governance.deduplication_id.as_deref()),
            Some("dedupe-1")
        );
    }

    #[test]
    fn grpc_failures_include_machine_readable_details() {
        let status = structured_status(Code::Unavailable, "event_broker_unavailable", "offline");
        assert_eq!(status.code(), Code::Unavailable);
        assert_eq!(
            status.details(),
            br#"{"code":"event_broker_unavailable","message":"offline"}"#
        );
    }

    #[test]
    fn load_tls_identity_requires_readable_files() {
        let err = load_tls_identity(
            std::path::Path::new("/no/such/cert.pem"),
            std::path::Path::new("/no/such/key.pem"),
        )
        .expect_err("missing TLS material must fail closed");
        assert!(err.contains("failed to read gRPC TLS certificate"));
    }

    #[test]
    fn load_tls_identity_reads_fixture_pems() {
        let cert = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/grpc-tls/cert.pem");
        let key = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/grpc-tls/key.pem");
        let (certificate, private_key) =
            load_tls_identity(&cert, &key).expect("fixture PEMs must load");
        assert!(!certificate.is_empty());
        assert!(!private_key.is_empty());
    }

    #[test]
    fn parse_grpc_bind_address_rejects_invalid_input() {
        let err = parse_grpc_bind_address("not-an-address").expect_err("must reject");
        assert!(err.contains("invalid gRPC bind address"));
        assert!(parse_grpc_bind_address("127.0.0.1:50051").is_ok());
    }

    #[test]
    fn load_tls_identity_requires_readable_private_key() {
        let cert = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/grpc-tls/cert.pem");
        let err = load_tls_identity(&cert, std::path::Path::new("/no/such/key.pem"))
            .expect_err("missing key must fail");
        assert!(err.contains("failed to read gRPC TLS private key"));
    }

    #[test]
    fn structured_status_covers_auth_failure_codes() {
        for (code, traverse_code) in [
            (Code::Unauthenticated, "unauthorized"),
            (Code::PermissionDenied, "forbidden"),
            (Code::InvalidArgument, "invalid_subscription"),
            (Code::Internal, "internal_error"),
        ] {
            let status = structured_status(code, traverse_code, "detail");
            assert_eq!(status.code(), code);
            assert!(
                status
                    .details()
                    .windows(traverse_code.len())
                    .any(|w| w == traverse_code.as_bytes())
            );
        }
    }

    #[test]
    fn grpc_code_for_http_status_maps_fr008_classes() {
        assert_eq!(grpc_code_for_http_status(400), Code::InvalidArgument);
        assert_eq!(grpc_code_for_http_status(401), Code::Unauthenticated);
        assert_eq!(grpc_code_for_http_status(403), Code::PermissionDenied);
        assert_eq!(grpc_code_for_http_status(503), Code::Unavailable);
        assert_eq!(grpc_code_for_http_status(500), Code::Internal);
        assert_eq!(grpc_code_for_http_status(418), Code::Internal);
        assert_eq!(grpc_code_for_http_status(0), Code::Internal);
        assert_eq!(grpc_code_for_http_status(u16::MAX), Code::Internal);
    }
}

//! WebSocket runtime event transport backed by [`EventBroker`].
//!
//! Governed by `097-websocket-grpc-event-transport` (issue #967) and ADR-0034.
//! Crate choice: `tungstenite` (sync `Read + Write` WebSocket) — matches the
//! existing std `TcpStream` HTTP server without forcing a Tokio rewrite.
//! Features limited to `handshake` (no TLS stack pulled for local/dev TCP).

use crate::app_runtime_events::{
    AppEventsHttpError, collect_app_runtime_events, short_signal_name,
};
use crate::http_api::{ApiState, HttpRequest, error_envelope, write_json};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::Write;
use std::net::TcpStream;
use std::time::Duration;
use traverse_runtime::LocalExecutor;
use traverse_runtime::events::TraverseEvent;
use tungstenite::Message;
use tungstenite::handshake::derive_accept_key;
use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::protocol::{CloseFrame, Role, WebSocket, WebSocketConfig};

/// Max inbound client WebSocket message size (FR-011).
pub(crate) const MAX_WS_INBOUND_MESSAGE_BYTES: usize = 64 * 1024;
/// Bounded live-poll iterations for a single connection turn in tests/dev.
const DEFAULT_LIVE_POLL_ROUNDS: usize = 1;
const LIVE_POLL_BATCH: usize = 64;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum ClientMessage {
    Subscribe(SubscribeRequest),
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SubscribeRequest {
    #[serde(default = "default_mode")]
    pub(crate) mode: String,
    #[serde(default)]
    pub(crate) from_cursor: Option<String>,
    #[serde(default)]
    pub(crate) execution_id: Option<String>,
    /// `browser_subscription` mode's other selector (spec
    /// 013-browser-runtime-subscription FR-001): exactly one of
    /// `request_id`/`execution_id` must be supplied.
    #[serde(default)]
    pub(crate) request_id: Option<String>,
}

fn default_mode() -> String {
    "app_events".to_string()
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ServerEventMessage<'a> {
    #[serde(rename = "type")]
    pub(crate) message_type: &'static str,
    pub(crate) cursor: &'a str,
    pub(crate) signal: &'a str,
    pub(crate) event: &'a TraverseEvent,
}

#[derive(Debug, Clone, Serialize)]
struct StructuredCloseReason {
    #[serde(rename = "type")]
    message_type: &'static str,
    traverse_code: String,
    detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    oldest_available_cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_event_id: Option<String>,
}

pub(crate) fn is_websocket_upgrade(request: &HttpRequest) -> bool {
    let upgrade = request
        .headers
        .get("upgrade")
        .is_some_and(|v| v.eq_ignore_ascii_case("websocket"));
    let connection = request.headers.get("connection").is_some_and(|v| {
        v.split(',')
            .any(|part| part.trim().eq_ignore_ascii_case("upgrade"))
    });
    let has_key = request.headers.contains_key("sec-websocket-key");
    request.method.eq_ignore_ascii_case("GET") && upgrade && connection && has_key
}

pub(crate) fn write_sse_retired_response<W: Write>(w: &mut W) -> Result<(), String> {
    write_json(
        w,
        426,
        "Upgrade Required",
        &error_envelope(
            "sse_retired",
            "SSE app-events transport has been retired; use WebSocket upgrade on this path \
             (097-websocket-grpc-event-transport / ADR-0034)",
        ),
    )
}

pub(crate) fn handle_app_events_websocket<E: LocalExecutor + Clone>(
    stream: &mut TcpStream,
    request: &HttpRequest,
    state: &ApiState<E>,
    loopback: bool,
    workspace_id: &str,
    app_id: &str,
    authorize: impl FnOnce(
        &HttpRequest,
        &ApiState<E>,
        bool,
        &str,
    ) -> Result<(), (u16, &'static str, Value)>,
) -> Result<(), String> {
    if let Err((status, reason, body)) = authorize(request, state, loopback, workspace_id) {
        return write_json(stream, status, reason, &body);
    }

    let mut socket = match complete_websocket_handshake(stream, request) {
        Ok(socket) => socket,
        Err(err) => {
            return write_json(
                stream,
                400,
                "Bad Request",
                &error_envelope("invalid_websocket_handshake", &err),
            );
        }
    };

    let subscribe = match read_subscribe_message(&mut socket) {
        Ok(subscribe) => subscribe,
        Err(CloseSignal::Structured(frame)) => {
            let _ = socket.close(Some(frame));
            let _ = socket.flush();
            return Ok(());
        }
        Err(CloseSignal::Io(err)) => return Err(err),
    };

    match subscribe.mode.as_str() {
        "browser_subscription" => serve_browser_subscription(
            &mut socket,
            state,
            workspace_id,
            subscribe.request_id.as_deref(),
            subscribe.execution_id.as_deref(),
        ),
        "app_events" => serve_app_events_subscription(
            &mut socket,
            state,
            workspace_id,
            app_id,
            subscribe.from_cursor.as_deref(),
        ),
        other => {
            let _ = socket.close(Some(structured_close(
                CloseCode::Policy,
                "invalid_request",
                &format!("unsupported subscribe mode '{other}'"),
                None,
                None,
            )));
            let _ = socket.flush();
            Ok(())
        }
    }
}

#[derive(Debug)]
enum CloseSignal {
    Structured(CloseFrame),
    Io(String),
}

fn complete_websocket_handshake<'a>(
    stream: &'a mut TcpStream,
    request: &HttpRequest,
) -> Result<WebSocket<&'a mut TcpStream>, String> {
    let key = request
        .headers
        .get("sec-websocket-key")
        .ok_or_else(|| "missing Sec-WebSocket-Key".to_string())?;
    let accept = derive_accept_key(key.as_bytes());
    let response = format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: {accept}\r\n\
         \r\n"
    );
    stream
        .write_all(response.as_bytes())
        .map_err(|e| format!("failed to write websocket handshake response: {e}"))?;
    stream
        .flush()
        .map_err(|e| format!("failed to flush websocket handshake response: {e}"))?;

    let config = WebSocketConfig::default().max_message_size(Some(MAX_WS_INBOUND_MESSAGE_BYTES));
    Ok(WebSocket::from_raw_socket(
        stream,
        Role::Server,
        Some(config),
    ))
}

fn read_subscribe_message(
    socket: &mut WebSocket<&mut TcpStream>,
) -> Result<SubscribeRequest, CloseSignal> {
    match socket.read() {
        Ok(Message::Text(text)) => parse_subscribe_text(text.as_str()),
        Ok(Message::Binary(bytes)) => {
            let text = String::from_utf8(bytes.to_vec()).map_err(|_| {
                CloseSignal::Structured(structured_close(
                    CloseCode::Policy,
                    "invalid_request",
                    "subscribe message must be UTF-8 text JSON",
                    None,
                    None,
                ))
            })?;
            parse_subscribe_text(&text)
        }
        Ok(Message::Close(_)) => Err(CloseSignal::Structured(structured_close(
            CloseCode::Normal,
            "client_closed",
            "client closed before subscribe",
            None,
            None,
        ))),
        Ok(Message::Ping(payload)) => {
            let _ = socket.send(Message::Pong(payload));
            read_subscribe_message(socket)
        }
        Ok(other) => Err(CloseSignal::Structured(structured_close(
            CloseCode::Policy,
            "invalid_request",
            &format!("expected subscribe text frame, got {other}"),
            None,
            None,
        ))),
        Err(tungstenite::Error::Capacity(_)) => Err(CloseSignal::Structured(structured_close(
            CloseCode::Size,
            "message_too_large",
            &format!("inbound WebSocket message exceeds {MAX_WS_INBOUND_MESSAGE_BYTES} bytes"),
            None,
            None,
        ))),
        Err(err) => Err(CloseSignal::Io(format!(
            "failed to read websocket subscribe message: {err}"
        ))),
    }
}

fn parse_subscribe_text(text: &str) -> Result<SubscribeRequest, CloseSignal> {
    if text.len() > MAX_WS_INBOUND_MESSAGE_BYTES {
        return Err(CloseSignal::Structured(structured_close(
            CloseCode::Size,
            "message_too_large",
            &format!("inbound WebSocket message exceeds {MAX_WS_INBOUND_MESSAGE_BYTES} bytes"),
            None,
            None,
        )));
    }
    match serde_json::from_str::<ClientMessage>(text) {
        Ok(ClientMessage::Subscribe(subscribe)) => Ok(subscribe),
        Err(err) => Err(CloseSignal::Structured(structured_close(
            CloseCode::Policy,
            "invalid_request",
            &format!("malformed subscribe message: {err}"),
            None,
            None,
        ))),
    }
}

fn serve_app_events_subscription<E: LocalExecutor + Clone>(
    socket: &mut WebSocket<&mut TcpStream>,
    state: &ApiState<E>,
    workspace_id: &str,
    app_id: &str,
    from_cursor: Option<&str>,
) -> Result<(), String> {
    let collected = state.with_workspace_mut(workspace_id, |ws| {
        Ok::<_, String>(collect_app_runtime_events(
            ws.runtime.event_broker().as_ref(),
            &ws.app_event_log,
            workspace_id,
            app_id,
            from_cursor,
        ))
    })?;

    let events = match collected {
        Ok(events) => events,
        Err(err) => {
            let frame = close_frame_for_app_events_error(&err);
            let _ = socket.close(Some(frame));
            let _ = socket.flush();
            return Ok(());
        }
    };

    for (cursor, event) in &events {
        if let Err(err) = send_event_frame(socket, cursor, event) {
            close_for_broker_failure(socket, &err);
            return Ok(());
        }
    }

    // Live follow: poll for newly published events (FR delivery over open connection).
    let mut cursor = events
        .last()
        .map(|(cursor, _)| cursor.clone())
        .or_else(|| from_cursor.map(str::to_string));
    for _ in 0..DEFAULT_LIVE_POLL_ROUNDS {
        // Non-blocking peek for client close / oversized frames between polls.
        if let Err(signal) = drain_client_control_frames(socket) {
            return match signal {
                CloseSignal::Structured(frame) => {
                    let _ = socket.close(Some(frame));
                    let _ = socket.flush();
                    Ok(())
                }
                CloseSignal::Io(err) => Err(err),
            };
        }
        let collected = state.with_workspace_mut(workspace_id, |ws| {
            Ok::<_, String>(collect_app_runtime_events(
                ws.runtime.event_broker().as_ref(),
                &ws.app_event_log,
                workspace_id,
                app_id,
                cursor.as_deref(),
            ))
        })?;
        let batch = match collected {
            Ok(batch) => batch,
            Err(err) => {
                close_for_app_events_error(socket, &err);
                return Ok(());
            }
        };
        if batch.is_empty() {
            break;
        }
        for (next_cursor, event) in batch.into_iter().take(LIVE_POLL_BATCH) {
            if let Err(err) = send_event_frame(socket, &next_cursor, &event) {
                close_for_broker_failure(socket, &err);
                return Ok(());
            }
            cursor = Some(next_cursor);
        }
    }

    let _ = socket.close(Some(structured_close(
        CloseCode::Normal,
        "stream_completed",
        "app events subscription completed",
        None,
        None,
    )));
    let _ = socket.flush();
    Ok(())
}

/// The one selector spec 013-browser-runtime-subscription FR-001 requires a
/// `browser_subscription` request to carry exactly one of.
#[derive(Debug, Clone, Copy)]
enum BrowserSubscriptionSelector<'a> {
    RequestId(&'a str),
    ExecutionId(&'a str),
}

fn serve_browser_subscription<E: LocalExecutor + Clone>(
    socket: &mut WebSocket<&mut TcpStream>,
    state: &ApiState<E>,
    workspace_id: &str,
    request_id: Option<&str>,
    execution_id: Option<&str>,
) -> Result<(), String> {
    // Spec 013 FR-001/FR-002 requires exactly one of request_id/execution_id.
    let selector = match (request_id, execution_id) {
        (Some(request_id), None) => BrowserSubscriptionSelector::RequestId(request_id),
        (None, Some(execution_id)) => BrowserSubscriptionSelector::ExecutionId(execution_id),
        (Some(_), Some(_)) => {
            let _ = socket.close(Some(structured_close(
                CloseCode::Policy,
                "invalid_request",
                "request_id and execution_id are mutually exclusive",
                None,
                None,
            )));
            let _ = socket.flush();
            return Ok(());
        }
        (None, None) => {
            let _ = socket.close(Some(structured_close(
                CloseCode::Policy,
                "invalid_request",
                "browser_subscription mode requires request_id or execution_id",
                None,
                None,
            )));
            let _ = socket.flush();
            return Ok(());
        }
    };

    let outcome = state.with_workspace_mut(workspace_id, |ws| {
        Ok(crate::http_api::resolve_browser_subscription_target(
            ws,
            request_id,
            execution_id,
        ))
    })?;

    let Some((execution_id, trace, succeeded)) = outcome else {
        let not_found_target = match selector {
            BrowserSubscriptionSelector::RequestId(request_id) => {
                format!("request '{request_id}'")
            }
            BrowserSubscriptionSelector::ExecutionId(execution_id) => {
                format!("execution '{execution_id}'")
            }
        };
        let _ = socket.close(Some(structured_close(
            CloseCode::Policy,
            "not_found",
            &format!("{not_found_target} was not found in workspace '{workspace_id}'"),
            None,
            None,
        )));
        let _ = socket.flush();
        return Ok(());
    };

    send_browser_subscription_stream(socket, selector, &execution_id, trace, succeeded)
}

/// Builds the spec 013-browser-runtime-subscription message sequence for a
/// resolved trace and streams it to the client, closing the socket once done.
fn send_browser_subscription_stream(
    socket: &mut WebSocket<&mut TcpStream>,
    selector: BrowserSubscriptionSelector<'_>,
    execution_id: &str,
    trace: traverse_runtime::RuntimeTrace,
    succeeded: bool,
) -> Result<(), String> {
    let request = traverse_runtime::BrowserRuntimeSubscriptionRequest {
        kind: "browser_runtime_subscription_request".to_string(),
        schema_version: "1.0.0".to_string(),
        governing_spec: "013-browser-runtime-subscription".to_string(),
        request_id: matches!(selector, BrowserSubscriptionSelector::RequestId(_))
            .then(|| trace.request.request_id.clone()),
        execution_id: matches!(selector, BrowserSubscriptionSelector::ExecutionId(_))
            .then(|| execution_id.to_string()),
    };
    let runtime_outcome = traverse_runtime::RuntimeExecutionOutcome {
        result: traverse_runtime::RuntimeResult {
            kind: "runtime_result".to_string(),
            schema_version: "1.0.0".to_string(),
            request_id: trace.request.request_id.clone(),
            execution_id: execution_id.to_string(),
            trace_ref: format!("trace_{execution_id}"),
            status: if succeeded {
                traverse_runtime::RuntimeResultStatus::Completed
            } else {
                traverse_runtime::RuntimeResultStatus::Error
            },
            output: None,
            error: None,
            warnings: Vec::new(),
        },
        trace,
        state_events: Vec::new(),
    };
    let messages = traverse_runtime::browser_subscription_messages(&request, &runtime_outcome);
    for message in messages {
        let payload = serde_json::to_string(&json!({
            "type": "browser_subscription",
            "message": message,
        }))
        .map_err(|e| format!("failed to serialize browser subscription message: {e}"))?;
        if let Err(err) = socket.send(Message::Text(payload.into())) {
            close_for_broker_failure(socket, &err.to_string());
            return Ok(());
        }
    }
    let _ = socket.close(Some(structured_close(
        CloseCode::Normal,
        "stream_completed",
        "browser subscription stream completed",
        None,
        None,
    )));
    let _ = socket.flush();
    Ok(())
}

fn send_event_frame(
    socket: &mut WebSocket<&mut TcpStream>,
    cursor: &str,
    event: &TraverseEvent,
) -> Result<(), String> {
    let payload = serde_json::to_string(&ServerEventMessage {
        message_type: "event",
        cursor,
        signal: short_signal_name(&event.event_type),
        event,
    })
    .map_err(|e| format!("failed to serialize websocket event frame: {e}"))?;
    socket
        .send(Message::Text(payload.into()))
        .map_err(|e| format!("failed to send websocket event frame: {e}"))
}

fn drain_client_control_frames(socket: &mut WebSocket<&mut TcpStream>) -> Result<(), CloseSignal> {
    // Best-effort non-blocking drain using a short read timeout.
    let stream = socket.get_mut();
    let previous = stream.read_timeout().unwrap_or(None);
    if stream
        .set_read_timeout(Some(Duration::from_millis(1)))
        .is_err()
    {
        return Ok(());
    }
    let result = match socket.read() {
        Ok(Message::Ping(payload)) => {
            let _ = socket.send(Message::Pong(payload));
            Ok(())
        }
        Ok(Message::Close(_)) => Err(CloseSignal::Structured(structured_close(
            CloseCode::Normal,
            "client_closed",
            "client closed the subscription",
            None,
            None,
        ))),
        Ok(Message::Text(_) | Message::Binary(_)) => {
            Err(CloseSignal::Structured(structured_close(
                CloseCode::Policy,
                "invalid_request",
                "unexpected client data frame after subscribe",
                None,
                None,
            )))
        }
        Err(tungstenite::Error::Capacity(_)) => Err(CloseSignal::Structured(structured_close(
            CloseCode::Size,
            "message_too_large",
            &format!("inbound WebSocket message exceeds {MAX_WS_INBOUND_MESSAGE_BYTES} bytes"),
            None,
            None,
        ))),
        // Idle / control frames / transient IO: keep the subscription open.
        Ok(Message::Pong(_) | Message::Frame(_)) | Err(_) => Ok(()),
    };
    let _ = socket.get_mut().set_read_timeout(previous);
    result
}

fn close_for_app_events_error(socket: &mut WebSocket<&mut TcpStream>, err: &AppEventsHttpError) {
    let _ = socket.close(Some(close_frame_for_app_events_error(err)));
    let _ = socket.flush();
}

fn close_for_broker_failure(socket: &mut WebSocket<&mut TcpStream>, detail: &str) {
    let _ = socket.close(Some(structured_close(
        CloseCode::Error,
        "event_broker_unavailable",
        detail,
        None,
        None,
    )));
    let _ = socket.flush();
}

fn close_frame_for_app_events_error(err: &AppEventsHttpError) -> CloseFrame {
    match err {
        AppEventsHttpError::InvalidCursor { value, detail } => structured_close(
            CloseCode::Policy,
            err.code(),
            detail,
            None,
            Some(value.clone()),
        ),
        AppEventsHttpError::CursorExpired {
            oldest_available_cursor,
        } => structured_close(
            CloseCode::Policy,
            err.code(),
            &err.message(),
            Some(oldest_available_cursor.clone()),
            None,
        ),
        AppEventsHttpError::Unavailable { detail } => {
            structured_close(CloseCode::Error, err.code(), detail, None, None)
        }
    }
}

fn structured_close(
    code: CloseCode,
    traverse_code: &str,
    detail: &str,
    oldest_available_cursor: Option<String>,
    last_event_id: Option<String>,
) -> CloseFrame {
    let reason = StructuredCloseReason {
        message_type: "error",
        traverse_code: traverse_code.to_string(),
        detail: detail.to_string(),
        oldest_available_cursor,
        last_event_id,
    };
    let encoded = serde_json::to_string(&reason).unwrap_or_else(|_| {
        format!(
            "{{\"type\":\"error\",\"traverse_code\":\"{traverse_code}\",\"detail\":\"{detail}\"}}"
        )
    });
    // Close reason is limited to 123 bytes by the WebSocket protocol.
    let truncated = if encoded.len() > 123 {
        encoded.chars().take(120).collect::<String>() + "..."
    } else {
        encoded
    };
    CloseFrame {
        code,
        reason: truncated.into(),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::app_runtime_events::AppEventsHttpError;

    #[test]
    fn subscribe_message_parses_app_events_mode() {
        let text = r#"{"type":"subscribe","mode":"app_events","from_cursor":"3"}"#;
        let subscribe = parse_subscribe_text(text).expect("subscribe must parse");
        assert_eq!(subscribe.mode, "app_events");
        assert_eq!(subscribe.from_cursor.as_deref(), Some("3"));
    }

    #[test]
    fn malformed_subscribe_message_is_structured_invalid_request() {
        let err = parse_subscribe_text(r#"{"type":"nope"}"#).expect_err("must reject");
        match err {
            CloseSignal::Structured(frame) => {
                assert!(frame.reason.contains("invalid_request"));
            }
            CloseSignal::Io(detail) => panic!("expected structured close, got io: {detail}"),
        }
    }

    #[test]
    fn oversized_subscribe_message_is_structured_message_too_large() {
        let oversized = format!(
            r#"{{"type":"subscribe","mode":"app_events","pad":"{}"}}"#,
            "x".repeat(MAX_WS_INBOUND_MESSAGE_BYTES)
        );
        let err = parse_subscribe_text(&oversized).expect_err("must reject oversized");
        match err {
            CloseSignal::Structured(frame) => {
                assert_eq!(frame.code, CloseCode::Size);
                assert!(frame.reason.contains("message_too_large"));
            }
            CloseSignal::Io(detail) => panic!("expected structured close, got io: {detail}"),
        }
    }

    #[test]
    fn browser_subscription_requires_execution_id_shape() {
        let text = r#"{"type":"subscribe","mode":"browser_subscription"}"#;
        let subscribe = parse_subscribe_text(text).expect("subscribe must parse");
        assert_eq!(subscribe.mode, "browser_subscription");
        assert!(subscribe.execution_id.is_none());
    }

    #[test]
    fn close_frames_preserve_cursor_and_broker_error_codes() {
        let invalid = close_frame_for_app_events_error(&AppEventsHttpError::InvalidCursor {
            value: "abc".to_string(),
            detail: "not an integer".to_string(),
        });
        assert_eq!(invalid.code, CloseCode::Policy);
        assert!(invalid.reason.contains("invalid_last_event_id"));

        let expired = close_frame_for_app_events_error(&AppEventsHttpError::CursorExpired {
            oldest_available_cursor: "10".to_string(),
        });
        assert_eq!(expired.code, CloseCode::Policy);
        assert!(expired.reason.contains("last_event_id_expired"));

        let unavailable = close_frame_for_app_events_error(&AppEventsHttpError::Unavailable {
            detail: "broker lock poisoned".to_string(),
        });
        assert_eq!(unavailable.code, CloseCode::Error);
        assert!(unavailable.reason.contains("event_broker_unavailable"));
    }

    #[test]
    fn sse_retired_response_is_426_upgrade_required() {
        let mut out = Vec::new();
        write_sse_retired_response(&mut out).expect("must write");
        let text = String::from_utf8(out).expect("utf8");
        assert!(text.contains("426"));
        assert!(text.contains("sse_retired"));
    }
}

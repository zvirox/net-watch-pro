//! Net Watch Pro UI shell: Unix-socket token handoff and daemon transport bridge.
//!
//! The UI talks to the daemon over a Unix domain socket with peer-credential
//! authorization (ARCHITECTURE.md §5.3, D1a). No token file, no Polkit token
//! helper, no bearer token.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![deny(clippy::unwrap_used, clippy::expect_used)]
#![deny(missing_docs)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tauri::async_runtime::JoinHandle;
use tauri::{Emitter, Manager, State, WebviewWindow};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

const DAEMON_SOCKET: &str = "/run/netwatch-pro/api.sock";
const MAX_WS_FRAME: usize = 64 * 1024;
/// Bounded response size for bridge HTTP requests (contract responses are
/// small JSON envelopes; 1 MiB is a generous ceiling).
const MAX_HTTP_RESPONSE: usize = 1024 * 1024;

#[derive(Default)]
struct LinkState(Arc<Mutex<LinkConnection>>);

#[derive(Default)]
struct LinkConnection {
    sender: Option<mpsc::Sender<String>>,
    task: Option<JoinHandle<()>>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HttpResponse {
    status: u16,
    body: String,
}

/// Connect to the daemon Unix socket and perform WebSocket handshake.
/// Peer-credential authorization is performed by the daemon (`SO_PEERCRED`).
#[tauri::command]
async fn daemon_connect(window: WebviewWindow, state: State<'_, LinkState>) -> Result<(), String> {
    require_main_window(&window)?;

    // Check if already connected
    {
        let connection = state
            .0
            .lock()
            .map_err(|_| "daemon link unavailable".to_string())?;
        if connection.sender.is_some() {
            return Err("daemon WebSocket is already connected".to_string());
        }
    }

    let socket = tokio::net::UnixStream::connect(DAEMON_SOCKET)
        .await
        .map_err(|_| "daemon WebSocket connection failed".to_string())?;

    let (socket, _) = tokio_tungstenite::client_async(
        tokio_tungstenite::tungstenite::http::Request::builder()
            .method("GET")
            .uri("ws://localhost/api/v1/stream")
            .body(())
            .map_err(|_| "cannot construct daemon WebSocket request".to_string())?,
        socket,
    )
    .await
    .map_err(|_| "daemon WebSocket connection failed".to_string())?;

    let (outgoing_tx, mut outgoing_rx) = mpsc::channel::<String>(64);
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let app = window.app_handle().clone();
    let task = tauri::async_runtime::spawn(async move {
        let (mut sink, mut stream) = socket.split();
        let _ = app.emit_to("main", "daemon://state", "connected");
        let _ = ready_tx.send(());
        loop {
            tokio::select! {
                outgoing = outgoing_rx.recv() => match outgoing {
                    Some(frame) => if sink.send(Message::Text(frame.into())).await.is_err() { break; },
                    None => break,
                },
                incoming = stream.next() => match incoming {
                    Some(Ok(Message::Text(frame))) => { let _ = app.emit_to("main", "daemon://frame", frame.to_string()); }
                    Some(Ok(Message::Close(frame))) => {
                        let state = if frame.as_ref().is_some_and(|close| u16::from(close.code) == 4401) { "auth-invalid" } else { "disconnected" };
                        let _ = app.emit_to("main", "daemon://state", state);
                        break;
                    }
                    Some(Ok(_)) => {}
                    Some(Err(_)) | None => {
                        let _ = app.emit_to("main", "daemon://state", "disconnected");
                        break;
                    }
                }
            }
        }
    });

    {
        let mut connection = state
            .0
            .lock()
            .map_err(|_| "daemon link unavailable".to_string())?;
        if let Some(old_task) = connection.task.take() {
            old_task.abort();
        }
        connection.sender = Some(outgoing_tx);
        connection.task = Some(task);
    }
    ready_rx
        .await
        .map_err(|_| "daemon WebSocket closed during setup".to_string())
}

/// Issue a real HTTP/1.1 request to the daemon over a short-lived Unix
/// socket connection (REST half of the transport; WS is the streaming half).
#[tauri::command]
async fn daemon_http_request(
    window: WebviewWindow,
    _state: State<'_, LinkState>,
    method: String,
    path: String,
    body: Option<String>,
) -> Result<HttpResponse, String> {
    require_main_window(&window)?;

    // Path allowlist: v1 API paths only, no query strings, no traversal.
    let api_path = validate_api_path(&path)?;

    let mut stream = tokio::net::UnixStream::connect(DAEMON_SOCKET)
        .await
        .map_err(|_| "daemon connection failed".to_string())?;

    let body_bytes = body.unwrap_or_default();
    let req = format!(
        "{method} {api_path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body_bytes}",
        body_bytes.len(),
    );
    tokio::time::timeout(Duration::from_secs(2), stream.write_all(req.as_bytes()))
        .await
        .map_err(|_| "daemon HTTP request timed out".to_string())?
        .map_err(|_| "daemon connection failed".to_string())?;

    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = tokio::time::timeout(Duration::from_secs(2), stream.read(&mut chunk))
            .await
            .map_err(|_| "daemon HTTP response timed out".to_string())?
            .map_err(|_| "daemon connection failed".to_string())?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        if buf.len() > MAX_HTTP_RESPONSE {
            return Err("daemon HTTP response exceeds limit".to_string());
        }
    }
    let text = String::from_utf8(buf).map_err(|_| "invalid daemon response".to_string())?;
    let status: u16 = text
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| "malformed daemon response".to_string())?;
    let resp_body = text
        .split_once("\r\n\r\n")
        .map(|(_, b)| b.to_string())
        .unwrap_or_default();
    Ok(HttpResponse {
        status,
        body: resp_body,
    })
}

/// Send a WebSocket frame over the daemon connection.
#[tauri::command]
async fn daemon_ws_send(
    window: WebviewWindow,
    state: State<'_, LinkState>,
    frame: String,
) -> Result<(), String> {
    require_main_window(&window)?;
    if frame.len() > MAX_WS_FRAME {
        return Err("WebSocket frame exceeds limit".to_string());
    }
    let sender = state
        .0
        .lock()
        .map_err(|_| "daemon link unavailable".to_string())?
        .sender
        .clone()
        .ok_or_else(|| "daemon WebSocket is disconnected".to_string())?;
    tokio::time::timeout(Duration::from_secs(2), sender.send(frame))
        .await
        .map_err(|_| "daemon WebSocket send timed out".to_string())?
        .map_err(|_| "daemon WebSocket is disconnected".to_string())
}

/// Disconnect from the daemon Unix socket.
#[tauri::command]
async fn daemon_ws_disconnect(
    window: WebviewWindow,
    state: State<'_, LinkState>,
) -> Result<(), String> {
    require_main_window(&window)?;
    let mut connection = state
        .0
        .lock()
        .map_err(|_| "daemon link unavailable".to_string())?;
    connection.sender.take();
    if let Some(task) = connection.task.take() {
        task.abort();
    }
    Ok(())
}

fn require_main_window(window: &WebviewWindow) -> Result<(), String> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err("command is available only to the main window".to_string())
    }
}

fn validate_api_path(path: &str) -> Result<&str, String> {
    if !path.starts_with("/api/v1/")
        || path.starts_with("//")
        || path.contains(['?', '#', '\\', '%'])
        || path
            .to_ascii_lowercase()
            .split('/')
            .any(|part| part == "." || part == "..")
    {
        return Err("invalid API path".to_string());
    }
    let suffix = path
        .strip_prefix("/api/v1")
        .ok_or_else(|| "invalid API path".to_string())?;
    if suffix.contains("//") {
        return Err("invalid API path".to_string());
    }
    Ok(suffix)
}

fn main() {
    let app = tauri::Builder::default()
        .manage(LinkState::default())
        .invoke_handler(tauri::generate_handler![
            daemon_connect,
            daemon_http_request,
            daemon_ws_send,
            daemon_ws_disconnect
        ])
        .run(tauri::generate_context!());
    if let Err(error) = app {
        eprintln!("netwatch-app: failed to start: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::validate_api_path;

    #[test]
    fn api_path_must_be_versioned_relative_path() {
        assert_eq!(validate_api_path("/api/v1/status"), Ok("/status"));
        for invalid in [
            "/status",
            "/api/v1//status",
            "/api/v1/../status",
            "/api/v1/status?x=1",
            "/api/v1/%2fstatus",
            "//127.0.0.1/status",
        ] {
            assert!(validate_api_path(invalid).is_err(), "accepted {invalid}");
        }
    }
}

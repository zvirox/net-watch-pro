//! Net Watch Pro UI shell: Unix-socket token handoff and daemon transport bridge.
//!
//! The UI talks to the daemon over a Unix domain socket with peer-credential
//! authorization (ARCHITECTURE.md §5.3, D1a). No token file, no Polkit token
//! helper, no bearer token.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![deny(clippy::unwrap_used, clippy::expect_used)]
#![deny(missing_docs)]

mod tray;

use std::path::Component;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tauri::async_runtime::JoinHandle;
use tauri::{Emitter, Manager, State, WebviewWindow};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::client::generate_key;
use tokio_tungstenite::tungstenite::http::header;

const DAEMON_SOCKET: &str = "/run/netwatch-pro/api.sock";
const MAX_WS_FRAME: usize = 64 * 1024;
/// Bounded response size for bridge HTTP requests (contract responses are
/// small JSON envelopes; 1 MiB is a generous ceiling).
const MAX_HTTP_RESPONSE: usize = 1024 * 1024;
/// Export files are much larger than JSON envelopes; the bridge buffers them
/// because the daemon's content endpoint has no range requests (API.md E3).
const MAX_EXPORT_RESPONSE: usize = 64 * 1024 * 1024;

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
            // Idempotent: onboarding calls authorize() after start() already
            // connected. Returning an error here traps the UI in reconnect
            // loops that can never succeed while the live socket is held.
            return Ok(());
        }
    }

    let socket = tokio::net::UnixStream::connect(DAEMON_SOCKET)
        .await
        .map_err(|_| "daemon WebSocket connection failed".to_string())?;

    // tungstenite does not add Sec-WebSocket-Key to a manually built request.
    let handshake = tokio_tungstenite::client_async(
        tokio_tungstenite::tungstenite::http::Request::builder()
            .method("GET")
            .uri("ws://localhost/api/v1/stream")
            .header(header::HOST, "localhost")
            .header(header::UPGRADE, "websocket")
            .header(header::CONNECTION, "Upgrade")
            .header(header::SEC_WEBSOCKET_KEY, generate_key())
            .header(header::SEC_WEBSOCKET_VERSION, "13")
            .body(())
            .map_err(|_| "cannot construct daemon WebSocket request".to_string())?,
        socket,
    )
    .await
    .map_err(|_| "daemon WebSocket connection failed".to_string())?;
    let (socket, _) = handshake;

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

    // Method and path are interpolated into a raw HTTP request line below, so
    // both are validated first: a CR/LF or space in either would let the
    // webview smuggle extra headers or a second request past the allowlist.
    let method = validate_http_method(&method)?;
    // Path allowlist: v1 API paths only; the full versioned path (and any
    // encoded query string) must be sent to the daemon unchanged.
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
        let limit = if api_path.starts_with("/exports/") && api_path.ends_with("/content") {
            MAX_EXPORT_RESPONSE
        } else {
            MAX_HTTP_RESPONSE
        };
        if buf.len() > limit {
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

/// One-time onboarding step (P8.5): execute the dedicated root helper via
/// polkit (`pro.netwatch.enable-monitoring`). The helper reads `PKEXEC_UID`,
/// appends it to the root-owned allowlist, configures the socket drop-in,
/// and enables/starts the service. No shell or string interpolation is used.
#[tauri::command]
async fn enable_monitoring(window: WebviewWindow) -> Result<(), String> {
    require_main_window(&window)?;
    let output = Command::new("pkexec")
        .arg("/usr/libexec/netwatch-pro/enable-monitoring")
        .output()
        .map_err(|error| format!("pkexec is unavailable: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let reason = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if reason.is_empty() {
            "monitoring could not be enabled".to_string()
        } else {
            reason
        })
    }
}

/// Update the tray tooltip with today's usage (P7.5). The UI owns the feed;
/// the shell only paints the text it is given.
#[tauri::command]
async fn set_tray_tooltip(app: tauri::AppHandle, text: String) -> Result<(), String> {
    let icon = app
        .tray_by_id(tray::TRAY_ID)
        .ok_or_else(|| "tray icon unavailable".to_string())?;
    icon.set_tooltip(Some(text))
        .map_err(|error| error.to_string())
}

/// Save a generated export to the path chosen in the native dialog (P6.8).
///
/// The path comes from the OS picker, but is re-validated here: the command
/// only accepts absolute `.csv`/`.json` paths with no traversal components.
/// The write runs on a blocking task so the UI loop never stalls; no
/// filesystem plugin is needed, keeping the dependency surface minimal.
#[tauri::command]
async fn save_export_file(
    window: WebviewWindow,
    path: String,
    content: String,
) -> Result<(), String> {
    require_main_window(&window)?;
    validate_export_path(&path)?;
    if content.len() > MAX_EXPORT_RESPONSE {
        return Err("export content exceeds size limit".to_string());
    }
    let bytes = content.into_bytes();
    tokio::task::spawn_blocking(move || std::fs::write(path, bytes))
        .await
        .map_err(|_| "export save task failed".to_string())?
        .map_err(|error| format!("cannot save export: {error}"))
}

/// Only absolute, non-traversing `.csv`/`.json` paths may be written.
fn validate_export_path(path: &str) -> Result<&std::path::Path, String> {
    let target = std::path::Path::new(path);
    if !target.is_absolute()
        || target
            .components()
            .any(|part| matches!(part, Component::CurDir | Component::ParentDir))
    {
        return Err("invalid save path".to_string());
    }
    let extension = target
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if extension != "csv" && extension != "json" {
        return Err("export files must be .csv or .json".to_string());
    }
    Ok(target)
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

/// Accept only the HTTP methods the daemon API uses, as exact upper-case
/// tokens. Anything else (including CR/LF-bearing strings) is rejected.
fn validate_http_method(method: &str) -> Result<&'static str, String> {
    match method {
        "GET" => Ok("GET"),
        "POST" => Ok("POST"),
        "PUT" => Ok("PUT"),
        "PATCH" => Ok("PATCH"),
        "DELETE" => Ok("DELETE"),
        _ => Err("invalid HTTP method".to_string()),
    }
}

/// Validate a caller-supplied API path and return it unchanged for the HTTP
/// request. Query strings are allowed (UI filters use encoded values) but
/// traversal, fragment markers and malformed percent escapes are rejected.
fn validate_api_path(path: &str) -> Result<&str, String> {
    // Raw bytes go into the request line: reject whitespace and every control
    // character (CR, LF, NUL, TAB, DEL) and any non-ASCII byte up front.
    if !path.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err("invalid API path".to_string());
    }
    let (path_part, query) = match path.split_once('?') {
        Some((head, tail)) => (head, Some(tail)),
        None => (path, None),
    };
    if !path_part.starts_with("/api/v1/")
        || path_part.starts_with("//")
        || path_part.contains("//")
        || path_part.contains(['#', '\\', '%'])
        || path_part
            .to_ascii_lowercase()
            .split('/')
            .any(|part| part == "." || part == "..")
    {
        return Err("invalid API path".to_string());
    }
    if let Some(query) = query {
        if query.is_empty() || query.contains(['#', '\\', '/', '?']) {
            return Err("invalid API query".to_string());
        }
        let bytes = query.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index] == b'%' {
                let digits = bytes.get(index + 1..index + 3).ok_or("invalid API query")?;
                if !digits.iter().all(u8::is_ascii_hexdigit) {
                    return Err("invalid API query".to_string());
                }
                index += 3;
            } else {
                index += 1;
            }
        }
    }
    Ok(path)
}

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(LinkState::default())
        .setup(|app| {
            // Tray failure is degraded, not fatal: the window still works.
            if let Err(error) = tray::setup(app.handle()) {
                eprintln!("netwatch-app: tray unavailable: {error}");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            daemon_connect,
            daemon_http_request,
            daemon_ws_send,
            daemon_ws_disconnect,
            save_export_file,
            set_tray_tooltip,
            enable_monitoring
        ])
        .run(tauri::generate_context!());
    if let Err(error) = app {
        eprintln!("netwatch-app: failed to start: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::{validate_api_path, validate_export_path, validate_http_method};

    #[test]
    fn http_method_is_an_exact_allowlisted_token() {
        for valid in ["GET", "POST", "PUT", "PATCH", "DELETE"] {
            assert_eq!(validate_http_method(valid), Ok(valid));
        }
        for invalid in [
            "",
            "get",
            "TRACE",
            "GET /api/v1/status HTTP/1.1\r\nX: y\r\n\r\nDELETE",
            "GET\r\n",
            "GET ",
        ] {
            assert!(
                validate_http_method(invalid).is_err(),
                "accepted {invalid:?}"
            );
        }
    }

    #[test]
    fn api_path_rejects_control_characters_and_whitespace() {
        for invalid in [
            "/api/v1/status\r\nX-Injected: 1",
            "/api/v1/status HTTP/1.1",
            "/api/v1/status\t",
            "/api/v1/status\0",
            "/api/v1/status?x=1\r\n",
            "/api/v1/status?x=a b",
            "/api/v1/stätus",
        ] {
            assert!(validate_api_path(invalid).is_err(), "accepted {invalid:?}");
        }
    }

    #[test]
    fn api_path_keeps_version_prefix_and_allows_encoded_queries() {
        assert_eq!(validate_api_path("/api/v1/status"), Ok("/api/v1/status"));
        assert_eq!(
            validate_api_path("/api/v1/connections?limit=50&app_name=A%20B"),
            Ok("/api/v1/connections?limit=50&app_name=A%20B")
        );
        for invalid in [
            "/status",
            "/api/v1//status",
            "/api/v1/../status",
            "/api/v1/%2fstatus",
            "/api/v1/status?",
            "/api/v1/status?x=%zz",
            "/api/v1/status#frag",
            "//127.0.0.1/status",
        ] {
            assert!(validate_api_path(invalid).is_err(), "accepted {invalid}");
        }
    }

    #[test]
    fn export_save_path_is_absolute_and_extension_checked() {
        assert!(validate_export_path("/tmp/report.csv").is_ok());
        assert!(validate_export_path("/tmp/report.JSON").is_ok());
        for invalid in [
            "relative.csv",
            "/tmp/../report.csv",
            "/tmp/report.txt",
            "/tmp/report",
        ] {
            assert!(validate_export_path(invalid).is_err(), "accepted {invalid}");
        }
    }
}

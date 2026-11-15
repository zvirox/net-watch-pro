//! Net Watch Pro UI shell: fixed-path token handoff and daemon transport bridge.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![deny(clippy::unwrap_used, clippy::expect_used)]
#![deny(missing_docs)]

mod token_link;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tauri::async_runtime::JoinHandle;
use tauri::{Emitter, Manager, State, WebviewWindow};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

const DAEMON_HTTP_BASE: &str = "http://127.0.0.1:7897/api/v1";
const DAEMON_WS_URL: &str = "ws://127.0.0.1:7897/api/v1/stream";
const MAX_WS_FRAME: usize = 64 * 1024;

#[derive(Default)]
struct LinkState(Arc<Mutex<LinkConnection>>);

#[derive(Default)]
struct LinkConnection {
    sender: Option<mpsc::Sender<String>>,
    task: Option<JoinHandle<()>>,
    generation: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HttpResponse {
    status: u16,
    body: String,
}

#[tauri::command]
async fn retrieve_daemon_token(window: WebviewWindow) -> Result<String, token_link::TokenError> {
    if require_main_window(&window).is_err() {
        return Err(token_link::TokenError::AuthorizationFailed);
    }
    tauri::async_runtime::spawn_blocking(token_link::retrieve_token)
        .await
        .map_err(|_| token_link::TokenError::ExecutionFailed)?
}

#[tauri::command]
async fn daemon_http_request(
    window: WebviewWindow,
    token: String,
    method: String,
    path: String,
    body: Option<String>,
) -> Result<HttpResponse, String> {
    require_main_window(&window)?;
    validate_token(&token)?;
    let path = validate_api_path(&path)?;
    let method = reqwest::Method::from_bytes(method.as_bytes())
        .map_err(|_| "unsupported HTTP method".to_string())?;
    if body.is_some() && matches!(method, reqwest::Method::GET | reqwest::Method::HEAD) {
        return Err("request bodies are not allowed for this method".to_string());
    }
    if !matches!(
        method,
        reqwest::Method::GET
            | reqwest::Method::POST
            | reqwest::Method::PUT
            | reqwest::Method::PATCH
            | reqwest::Method::DELETE
    ) {
        return Err("unsupported HTTP method".to_string());
    }

    let url = format!("{DAEMON_HTTP_BASE}{path}");
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|_| "cannot initialize daemon HTTP client".to_string())?;
    let mut request = client
        .request(method, url)
        .bearer_auth(token)
        .header(reqwest::header::ACCEPT, "application/json");
    if let Some(body) = body {
        if body.len() > 1024 * 1024 {
            return Err("request body exceeds limit".to_string());
        }
        request = request
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body);
    }
    let response = request
        .send()
        .await
        .map_err(|_| "daemon request failed".to_string())?;
    let status = response.status().as_u16();
    if response
        .content_length()
        .is_some_and(|length| length > 4 * 1024 * 1024)
    {
        return Err("daemon response exceeds limit".to_string());
    }
    let body = response
        .text()
        .await
        .map_err(|_| "cannot read daemon response".to_string())?;
    if body.len() > 4 * 1024 * 1024 {
        return Err("daemon response exceeds limit".to_string());
    }
    Ok(HttpResponse { status, body })
}

#[tauri::command]
async fn daemon_ws_connect(
    window: WebviewWindow,
    state: State<'_, LinkState>,
    token: String,
) -> Result<(), String> {
    require_main_window(&window)?;
    validate_token(&token)?;
    {
        let connection = state
            .0
            .lock()
            .map_err(|_| "daemon link unavailable".to_string())?;
        if connection.sender.is_some() {
            return Err("daemon WebSocket is already connected".to_string());
        }
    }
    let request = tokio_tungstenite::tungstenite::http::Request::builder()
        .method("GET")
        .uri(DAEMON_WS_URL)
        .header("Authorization", format!("Bearer {token}"))
        .body(())
        .map_err(|_| "cannot construct daemon WebSocket request".to_string())?;
    let (socket, _) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(|_| "daemon WebSocket connection failed".to_string())?;

    let (outgoing_tx, mut outgoing_rx) = mpsc::channel::<String>(64);
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let _generation = {
        let mut connection = state
            .0
            .lock()
            .map_err(|_| "daemon link unavailable".to_string())?;
        connection.generation = connection.generation.wrapping_add(1);
        connection.generation
    };
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

fn validate_token(token: &str) -> Result<(), String> {
    if token.len() == 64 && token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err("invalid daemon token".to_string())
    }
}

fn main() {
    let app = tauri::Builder::default()
        .manage(LinkState::default())
        .invoke_handler(tauri::generate_handler![
            retrieve_daemon_token,
            daemon_http_request,
            daemon_ws_connect,
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
    use super::{validate_api_path, validate_token};

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

    #[test]
    fn token_requires_exact_canonical_hex_length() {
        assert!(validate_token(&"a".repeat(64)).is_ok());
        assert!(validate_token("short").is_err());
        assert!(validate_token(&format!("{}g", "a".repeat(63))).is_err());
    }
}

//! Phase 1 integration tests — API.md contracts for §3 stream.
//!
//! Transport is the Unix domain socket (D1a): the server is spawned on an
//! ephemeral socket path and clients connect with `tokio-tungstenite`
//! `client_async` over a `UnixStream`.
//!
//! REST (`/health`, `/status`) is asserted end-to-end with raw HTTP/1.1
//! requests over a `UnixStream` (no HTTP client crate needed for GETs).
//!
//! Peer-credential auth (D1a) cannot be simulated in-process (denied UID
//! requires a foreign process), so the old 401/4401 token tests are gone;
//! auth is a socket-level concern tested manually on a real deployment.

// Integration tests are not daemon paths (ARCHITECTURE.md §7.1): unwrapping
// in test setup/assertions is standard and expected.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use netwatch_api::state::AppState;
use netwatch_common::{DaemonState, EbpfState, StatusSnapshot};
use serde_json::{Value, json};
use tokio::net::UnixStream;
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::Message;

type WsStream = tokio_tungstenite::WebSocketStream<UnixStream>;

/// Boot the server on an ephemeral Unix socket path; returns (path, status
/// sender).
async fn spawn_server() -> (std::path::PathBuf, watch::Sender<StatusSnapshot>) {
    let (tx, rx) = watch::channel(StatusSnapshot {
        daemon: DaemonState::Monitoring,
        ebpf: EbpfState::Attached,
        recording: true,
    });

    let dir = std::env::temp_dir().join(format!(
        "netwatch-api-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let sock = dir.join("api.sock");

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let sock_for_serve = sock.clone();
    let handle = tokio::spawn(async move {
        if let Err(e) = netwatch_api::serve_unix(&sock_for_serve, rx, async move {
            let _ = shutdown_rx.await;
        })
        .await
        {
            eprintln!("serve task failed: {e}");
        }
    });

    // Keep the shutdown sender alive for the whole test (dropping it would
    // cancel the oneshot and end the server immediately).
    std::mem::forget(shutdown_tx);

    // Wait for the socket file to accept (serve binds inside the task).
    for _ in 0..100 {
        if UnixStream::connect(&sock).await.is_ok() {
            return (sock, tx);
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    handle.abort();
    panic!("server did not start on {sock:?}");
}

/// Connect to the WS endpoint over the Unix socket.
async fn ws_connect(sock: &std::path::Path) -> WsStream {
    let stream = UnixStream::connect(sock).await.unwrap();
    // Host is ignored on Unix-socket transport; path selects the endpoint.
    let req = tokio_tungstenite::tungstenite::client::IntoClientRequest::into_client_request(
        "ws://localhost/api/v1/stream",
    )
    .unwrap();
    let (stream, _) = tokio_tungstenite::client_async(req, stream).await.unwrap();
    stream
}

/// Next text frame as JSON (5 s bound so a hang fails loudly, not slowly).
async fn ws_next(stream: &mut WsStream) -> Value {
    let msg = tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("timed out waiting for WS frame")
        .expect("WS stream ended unexpectedly");
    match msg.expect("WS error") {
        Message::Text(t) => serde_json::from_str(&t).unwrap(),
        other => panic!("expected text frame, got {other:?}"),
    }
}

async fn ws_send(stream: &mut WsStream, v: Value) {
    stream
        .send(Message::Text(v.to_string().into()))
        .await
        .unwrap();
}

// -------------------------------------------------------------- §3 stream

#[tokio::test(flavor = "multi_thread")]
async fn subscribe_live_connections_returns_snapshot() {
    let (sock, _tx) = spawn_server().await;
    let mut ws = ws_connect(&sock).await;
    ws_send(
        &mut ws,
        json!({ "op": "subscribe", "channel": "live.connections" }),
    )
    .await;
    let v = ws_next(&mut ws).await;
    assert_eq!(v["channel"], "live.connections");
    assert_eq!(v["kind"], "snapshot");
    assert_eq!(v["data"], json!([])); // Phase 1 stub: no traffic data yet
}

#[tokio::test(flavor = "multi_thread")]
async fn subscribe_status_pushes_current_then_updates_on_change() {
    let (sock, tx) = spawn_server().await;
    let mut ws = ws_connect(&sock).await;
    ws_send(&mut ws, json!({ "op": "subscribe", "channel": "status" })).await;

    let v = ws_next(&mut ws).await;
    assert_eq!(v["channel"], "status");
    assert_eq!(v["kind"], "update");
    assert_eq!(v["data"]["daemon"], "monitoring");

    // Publish a change through the daemon's watch channel → client gets it.
    tx.send_replace(StatusSnapshot {
        daemon: DaemonState::Stopped,
        ebpf: EbpfState::Failed,
        recording: false,
    });
    let v = ws_next(&mut ws).await;
    assert_eq!(v["data"]["daemon"], "stopped");
    assert_eq!(v["data"]["recording"], false);
}

#[tokio::test(flavor = "multi_thread")]
async fn subscribe_usage_totals_returns_stub_update() {
    let (sock, _tx) = spawn_server().await;
    let mut ws = ws_connect(&sock).await;
    ws_send(
        &mut ws,
        json!({ "op": "subscribe", "channel": "usage.totals" }),
    )
    .await;
    let v = ws_next(&mut ws).await;
    assert_eq!(v["channel"], "usage.totals");
    assert_eq!(v["kind"], "update");
    assert_eq!(v["data"]["today"], json!({ "up": 0, "down": 0 }));
}

#[tokio::test(flavor = "multi_thread")]
async fn subscribe_unknown_channel_returns_query_invalid_frame() {
    let (sock, _tx) = spawn_server().await;
    let mut ws = ws_connect(&sock).await;
    ws_send(&mut ws, json!({ "op": "subscribe", "channel": "x" })).await;
    let v = ws_next(&mut ws).await;
    assert_eq!(v["op"], "error");
    assert_eq!(v["error"]["code"], "QUERY_INVALID");
    assert_eq!(v["error"]["message"], "Unknown channel 'x'");
}

#[tokio::test(flavor = "multi_thread")]
async fn unknown_op_returns_query_invalid_frame() {
    let (sock, _tx) = spawn_server().await;
    let mut ws = ws_connect(&sock).await;
    ws_send(&mut ws, json!({ "op": "nonsense" })).await;
    let v = ws_next(&mut ws).await;
    assert_eq!(v["op"], "error");
    assert_eq!(v["error"]["code"], "QUERY_INVALID");
}

#[tokio::test(flavor = "multi_thread")]
async fn ping_replies_with_pong() {
    let (sock, _tx) = spawn_server().await;
    let mut ws = ws_connect(&sock).await;
    ws_send(&mut ws, json!({ "op": "ping" })).await;
    let v = ws_next(&mut ws).await;
    assert_eq!(v["op"], "pong");
}

// ------------------------------------------------- S1/S2 REST over socket

/// Raw HTTP/1.1 GET over the Unix socket; returns (status_code, body_text).
/// Bounded read — the daemon only serves small JSON envelopes.
async fn http_get(sock: &std::path::Path, path: &str) -> (u16, String) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut stream = UnixStream::connect(sock).await.unwrap();
    let req = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).await.unwrap();
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = stream.read(&mut chunk).await.unwrap();
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        // 64 KiB bound: far above any contract response; guards a runaway.
        if buf.len() > 64 * 1024 {
            panic!("response exceeds 64 KiB bound");
        }
    }
    let text = String::from_utf8(buf).unwrap();
    let status: u16 = text
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .expect("no HTTP status line in response");
    let body = text
        .split_once("\r\n\r\n")
        .map(|(_, b)| b.to_string())
        .unwrap_or_default();
    (status, body)
}

#[tokio::test(flavor = "multi_thread")]
async fn health_over_socket_returns_contract_shape() {
    let (sock, _tx) = spawn_server().await;
    let (status, body) = http_get(&sock, "/api/v1/health").await;
    assert_eq!(status, 200);
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["status"], "ok");
    assert_eq!(v["api"], "v1");
    assert!(v["version"].as_str().is_some_and(|s| !s.is_empty()));
}

#[tokio::test(flavor = "multi_thread")]
async fn status_over_socket_returns_contract_shape() {
    let (sock, _tx) = spawn_server().await;
    let (status, body) = http_get(&sock, "/api/v1/status").await;
    assert_eq!(status, 200);
    let v: Value = serde_json::from_str(&body).unwrap();
    // Stub state per PLAN.md P1.6 until the Phase 2 collector publishes real states.
    assert_eq!(v["daemon"], "monitoring");
    assert_eq!(v["ebpf"], "attached");
    assert_eq!(v["recording"], true);
    assert_eq!(v["ws_clients"], 0);
    assert!(v["uptime_s"].is_u64());
    assert_eq!(v["drops"]["ring_buffer_dropped"], 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn unknown_route_over_socket_is_404_envelope() {
    let (sock, _tx) = spawn_server().await;
    let (status, body) = http_get(&sock, "/api/v1/nope").await;
    assert_eq!(status, 404);
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["error"]["code"], "NOT_FOUND");
    assert!(v["error"]["request_id"].as_str().is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn wrong_method_over_socket_is_405_envelope() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let (sock, _tx) = spawn_server().await;
    let mut stream = UnixStream::connect(&sock).await.unwrap();
    stream
        .write_all(b"POST /api/v1/status HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = stream.read(&mut chunk).await.unwrap();
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    let text = String::from_utf8(buf).unwrap();
    let status: u16 = text
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .expect("no HTTP status line");
    assert_eq!(status, 405);
    let body = text.split_once("\r\n\r\n").map(|(_, b)| b).unwrap_or("");
    let v: Value = serde_json::from_str(body).unwrap();
    assert_eq!(v["error"]["code"], "METHOD_NOT_ALLOWED");
}

/// WS client counting feeds `ws_clients` in `GET /status` (S2).
#[tokio::test]
async fn client_counting_bumps_status_counter() {
    let (_tx, rx) = watch::channel(StatusSnapshot {
        daemon: DaemonState::Monitoring,
        ebpf: EbpfState::Attached,
        recording: true,
    });
    let state = Arc::new(AppState::new(rx));
    state.client_delta(1);
    assert_eq!(state.client_count(), 1);
    state.client_delta(-1);
    assert_eq!(state.client_count(), 0);
}

/// Stale socket files are removed at bind (serve_unix contract).
#[tokio::test(flavor = "multi_thread")]
async fn stale_socket_file_is_removed_before_bind() {
    let dir = std::env::temp_dir().join(format!(
        "netwatch-api-stale-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let sock = dir.join("stale.sock");
    // Pre-existing regular file at the socket path → serve must clear it.
    std::fs::write(&sock, b"stale").unwrap();

    let (_stale_tx, rx) = watch::channel(StatusSnapshot {
        daemon: DaemonState::Monitoring,
        ebpf: EbpfState::Attached,
        recording: true,
    });
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let serve_sock = sock.clone();
    let handle = tokio::spawn(async move {
        let _ = netwatch_api::serve_unix(&serve_sock, rx, async move {
            let _ = shutdown_rx.await;
        })
        .await;
    });

    // Server should bind successfully (stale file removed), serving WS.
    let mut connected = false;
    for _ in 0..100 {
        if UnixStream::connect(&sock).await.is_ok() {
            connected = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    shutdown_tx.send(()).ok();
    handle.abort();
    assert!(connected, "serve_unix did not replace a stale socket file");
}

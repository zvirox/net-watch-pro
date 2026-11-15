//! Phase 1 integration tests — API.md contracts for S1/S2 and §3 stream.
//!
//! Each test boots the real server on an ephemeral loopback port and talks
//! to it with the approved test clients (reqwest / tokio-tungstenite).

// Integration tests are not daemon paths (ARCHITECTURE.md §7.1): unwrapping
// in test setup/assertions is standard and expected.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use netwatch_common::{DaemonState, EbpfState, StatusSnapshot};
use serde_json::{Value, json};
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::Message;

type WsStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

const TEST_TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// Boot the server on 127.0.0.1:<ephemeral>; returns (addr, status sender).
async fn spawn_server() -> (SocketAddr, watch::Sender<StatusSnapshot>) {
    let (tx, rx) = watch::channel(StatusSnapshot {
        daemon: DaemonState::Monitoring,
        ebpf: EbpfState::Attached,
        recording: true,
    });
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    // Discover an ephemeral port: bind once, take the addr, free it for serve.
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);

    let handle = tokio::spawn(async move {
        if let Err(e) = netwatch_api::serve(addr, Arc::new(TEST_TOKEN.to_string()), rx, async {
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

    // Wait for the socket to accept (serve binds inside the task).
    for _ in 0..100 {
        if tokio::net::TcpStream::connect(addr).await.is_ok() {
            return (addr, tx);
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    handle.abort();
    panic!("server did not start on {addr}");
}

/// HTTP client with the bearer header pre-set.
fn client(token: &str) -> reqwest::Client {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        reqwest::header::HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    );
    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap()
}

/// Connect to the WS endpoint with the given (optional) token.
async fn ws_connect(addr: SocketAddr, token: Option<&str>) -> WsStream {
    let req = tokio_tungstenite::tungstenite::client::IntoClientRequest::into_client_request(
        format!("ws://{addr}/api/v1/stream"),
    )
    .unwrap();
    let req = match token {
        Some(t) => {
            let mut r = req;
            r.headers_mut().insert(
                tokio_tungstenite::tungstenite::http::header::AUTHORIZATION,
                tokio_tungstenite::tungstenite::http::HeaderValue::from_str(&format!("Bearer {t}"))
                    .unwrap(),
            );
            r
        }
        None => req,
    };
    let (stream, _) = tokio_tungstenite::connect_async(req).await.unwrap();
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

/// Next WS close, asserting the close code (e.g. 4401).
async fn ws_close_code(stream: &mut WsStream) -> u16 {
    loop {
        let msg = tokio::time::timeout(Duration::from_secs(5), stream.next())
            .await
            .expect("timed out waiting for close")
            .expect("stream ended without close frame");
        match msg {
            Ok(Message::Close(frame)) => return u16::from(frame.expect("close frame").code),
            Ok(_) => {} // skip anything before close
            Err(e) => panic!("WS error while awaiting close: {e}"),
        }
    }
}

async fn ws_send(stream: &mut WsStream, v: Value) {
    stream
        .send(Message::Text(v.to_string().into()))
        .await
        .unwrap();
}

// ---------------------------------------------------------------- S1 health

#[tokio::test(flavor = "multi_thread")]
async fn health_needs_no_auth_and_returns_contract_shape() {
    let (addr, _tx) = spawn_server().await;
    let resp = reqwest::get(format!("http://{addr}/api/v1/health"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let v: Value = resp.json().await.unwrap();
    assert_eq!(v["status"], "ok");
    assert_eq!(v["api"], "v1");
    assert!(v["version"].as_str().is_some_and(|s| !s.is_empty()));
}

// ---------------------------------------------------------------- S2 status

#[tokio::test(flavor = "multi_thread")]
async fn status_without_token_is_401_auth_invalid_envelope() {
    let (addr, _tx) = spawn_server().await;
    let resp = reqwest::get(format!("http://{addr}/api/v1/status"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
    let v: Value = resp.json().await.unwrap();
    assert_eq!(v["error"]["code"], "AUTH_INVALID");
    assert_eq!(v["error"]["details"], json!({}));
    assert!(v["error"]["request_id"].as_str().is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn status_with_wrong_token_is_401() {
    let (addr, _tx) = spawn_server().await;
    let resp = client("wrong-token")
        .get(format!("http://{addr}/api/v1/status"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test(flavor = "multi_thread")]
async fn status_with_token_returns_contract_shape() {
    let (addr, _tx) = spawn_server().await;
    let resp = client(TEST_TOKEN)
        .get(format!("http://{addr}/api/v1/status"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let v: Value = resp.json().await.unwrap();
    // Stub state per PLAN.md P1.6 until the Phase 2 collector publishes real states.
    assert_eq!(v["daemon"], "monitoring");
    assert_eq!(v["ebpf"], "attached");
    assert_eq!(v["recording"], true);
    assert_eq!(v["ws_clients"], 0);
    assert!(v["uptime_s"].is_u64());
    assert_eq!(v["drops"]["ring_buffer_dropped"], 0);
}

// ------------------------------------------------- envelope + route errors

#[tokio::test(flavor = "multi_thread")]
async fn unknown_route_is_404_not_found_envelope() {
    let (addr, _tx) = spawn_server().await;
    let resp = reqwest::get(format!("http://{addr}/api/v1/nope"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
    let v: Value = resp.json().await.unwrap();
    assert_eq!(v["error"]["code"], "NOT_FOUND");
}

#[tokio::test(flavor = "multi_thread")]
async fn wrong_method_is_405_method_not_allowed_envelope() {
    let (addr, _tx) = spawn_server().await;
    let resp = client(TEST_TOKEN)
        .post(format!("http://{addr}/api/v1/status"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 405);
    let v: Value = resp.json().await.unwrap();
    assert_eq!(v["error"]["code"], "METHOD_NOT_ALLOWED");
}

// -------------------------------------------------------------- §3 stream

#[tokio::test(flavor = "multi_thread")]
async fn ws_without_token_closes_with_4401() {
    let (addr, _tx) = spawn_server().await;
    let mut ws = ws_connect(addr, None).await;
    assert_eq!(ws_close_code(&mut ws).await, 4401);
}

#[tokio::test(flavor = "multi_thread")]
async fn ws_wrong_token_closes_with_4401() {
    let (addr, _tx) = spawn_server().await;
    let mut ws = ws_connect(addr, Some("nope")).await;
    assert_eq!(ws_close_code(&mut ws).await, 4401);
}

#[tokio::test(flavor = "multi_thread")]
async fn subscribe_live_connections_returns_snapshot() {
    let (addr, _tx) = spawn_server().await;
    let mut ws = ws_connect(addr, Some(TEST_TOKEN)).await;
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
    let (addr, tx) = spawn_server().await;
    let mut ws = ws_connect(addr, Some(TEST_TOKEN)).await;
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
    let (addr, _tx) = spawn_server().await;
    let mut ws = ws_connect(addr, Some(TEST_TOKEN)).await;
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
    let (addr, _tx) = spawn_server().await;
    let mut ws = ws_connect(addr, Some(TEST_TOKEN)).await;
    ws_send(&mut ws, json!({ "op": "subscribe", "channel": "x" })).await;
    let v = ws_next(&mut ws).await;
    assert_eq!(v["op"], "error");
    assert_eq!(v["error"]["code"], "QUERY_INVALID");
    assert_eq!(v["error"]["message"], "Unknown channel 'x'");
}

#[tokio::test(flavor = "multi_thread")]
async fn unknown_op_returns_query_invalid_frame() {
    let (addr, _tx) = spawn_server().await;
    let mut ws = ws_connect(addr, Some(TEST_TOKEN)).await;
    ws_send(&mut ws, json!({ "op": "nonsense" })).await;
    let v = ws_next(&mut ws).await;
    assert_eq!(v["op"], "error");
    assert_eq!(v["error"]["code"], "QUERY_INVALID");
}

#[tokio::test(flavor = "multi_thread")]
async fn ping_replies_with_pong() {
    let (addr, _tx) = spawn_server().await;
    let mut ws = ws_connect(addr, Some(TEST_TOKEN)).await;
    ws_send(&mut ws, json!({ "op": "ping" })).await;
    let v = ws_next(&mut ws).await;
    assert_eq!(v["op"], "pong");
}

#[tokio::test(flavor = "multi_thread")]
async fn ws_clients_count_surfaces_in_status() {
    let (addr, _tx) = spawn_server().await;
    let _ws = ws_connect(addr, Some(TEST_TOKEN)).await;
    // Brief settle for the upgrade to complete.
    tokio::time::sleep(Duration::from_millis(100)).await;
    let resp = client(TEST_TOKEN)
        .get(format!("http://{addr}/api/v1/status"))
        .send()
        .await
        .unwrap();
    let v: Value = resp.json().await.unwrap();
    assert_eq!(v["ws_clients"], 1);
}

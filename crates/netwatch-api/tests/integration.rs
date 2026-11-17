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
use netwatch_common::{DaemonState, EbpfState, StatusSnapshot, UsageTotals};
use serde_json::{Value, json};
use tokio::net::UnixStream;
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::Message;

type WsStream = tokio_tungstenite::WebSocketStream<UnixStream>;

/// Empty live feed for tests that don't inject live data; returns the feed
/// and its delta sender (to publish if needed).
fn live_feed() -> (
    netwatch_api::LiveFeed,
    tokio::sync::broadcast::Sender<netwatch_common::LiveDelta>,
) {
    let (feed, deltas, _) = live_feed_with_totals();
    (feed, deltas)
}

/// Feed with an injectable totals sender for the `usage.totals` channel test.
fn live_feed_with_totals() -> (
    netwatch_api::LiveFeed,
    tokio::sync::broadcast::Sender<netwatch_common::LiveDelta>,
    watch::Sender<Arc<UsageTotals>>,
) {
    let (_, snapshot_rx) = watch::channel(Arc::new(Vec::new()));
    let (delta_tx, _) = tokio::sync::broadcast::channel(16);
    let (totals_tx, totals_rx) = watch::channel(Arc::new(UsageTotals::default()));
    (
        netwatch_api::LiveFeed {
            snapshot: snapshot_rx,
            deltas: delta_tx.clone(),
            totals: totals_rx,
        },
        delta_tx,
        totals_tx,
    )
}

/// Migrated-empty in-memory database for tests that don't seed history data
/// (single connection so every query sees the same schema).
async fn test_db() -> sqlx::SqlitePool {
    sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(sqlx::sqlite::SqliteConnectOptions::new().in_memory(true))
        .await
        .unwrap()
}

/// Minimal history schema for tests (mirrors migrations 0001–0003 columns
/// the history queries touch).
pub(crate) async fn seed_history_schema(pool: &sqlx::SqlitePool) {
    sqlx::query("CREATE TABLE license (id INTEGER PRIMARY KEY CHECK (id = 1), tier TEXT NOT NULL DEFAULT 'free')")
        .execute(pool).await.unwrap();
    sqlx::query("CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO settings (key, value) VALUES ('raw_window_days', '7')")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO license (id, tier) VALUES (1, 'free')")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "CREATE TABLE apps (
            id INTEGER PRIMARY KEY, name TEXT NOT NULL, icon_path TEXT,
            attribution_confidence TEXT NOT NULL DEFAULT 'exact',
            first_seen_at INTEGER NOT NULL DEFAULT 0,
            last_seen_at INTEGER NOT NULL DEFAULT 0,
            deleted_at INTEGER)",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "CREATE TABLE connections (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            app_id INTEGER NOT NULL, pid INTEGER NOT NULL, process_name TEXT,
            protocol TEXT NOT NULL, local_ip TEXT NOT NULL, local_port INTEGER NOT NULL,
            remote_ip TEXT NOT NULL, remote_port INTEGER NOT NULL, remote_host TEXT,
            bytes_up INTEGER NOT NULL DEFAULT 0, bytes_down INTEGER NOT NULL DEFAULT 0,
            packets_up INTEGER NOT NULL DEFAULT 0, packets_down INTEGER NOT NULL DEFAULT 0,
            first_seen_at INTEGER NOT NULL, last_seen_at INTEGER NOT NULL,
            is_open INTEGER NOT NULL DEFAULT 0, interface_id INTEGER)",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "CREATE TABLE usage_endpoint_day (
            app_id INTEGER NOT NULL, bucket_start INTEGER NOT NULL,
            remote_ip TEXT NOT NULL, remote_port INTEGER NOT NULL, protocol TEXT NOT NULL,
            bytes_up INTEGER NOT NULL DEFAULT 0, bytes_down INTEGER NOT NULL DEFAULT 0,
            connection_count INTEGER NOT NULL DEFAULT 0,
            first_seen_at INTEGER NOT NULL, last_seen_at INTEGER NOT NULL,
            PRIMARY KEY (app_id, bucket_start, remote_ip, remote_port, protocol))",
    )
    .execute(pool)
    .await
    .unwrap();
}

/// Minimal U1 schema: the two rollup tables used by `/usage/summary`.
async fn seed_usage_schema(pool: &sqlx::SqlitePool) {
    sqlx::query(
        "CREATE TABLE usage_interface_15m (
            interface_id INTEGER NOT NULL, bucket_start INTEGER NOT NULL,
            rx_bytes INTEGER NOT NULL DEFAULT 0, tx_bytes INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (interface_id, bucket_start))",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "CREATE TABLE usage_rollup_15m (
            app_id INTEGER NOT NULL, bucket_start INTEGER NOT NULL,
            bytes_up INTEGER NOT NULL DEFAULT 0, bytes_down INTEGER NOT NULL DEFAULT 0,
            connection_count INTEGER NOT NULL DEFAULT 0, interfaces_mask TEXT,
            PRIMARY KEY (app_id, bucket_start))",
    )
    .execute(pool)
    .await
    .unwrap();
}

/// Boot the server on an ephemeral Unix socket path with an empty database;
/// returns (path, status sender).
async fn spawn_server() -> (std::path::PathBuf, watch::Sender<StatusSnapshot>) {
    spawn_server_with(test_db().await).await
}

/// Boot the server against a caller-supplied database (history tests seed it
/// first so REST queries have rows to page through).
async fn spawn_server_with(
    db: sqlx::SqlitePool,
) -> (std::path::PathBuf, watch::Sender<StatusSnapshot>) {
    let (feed, _) = live_feed();
    spawn_server_with_feed(db, feed).await
}

/// Boot the server with a caller-supplied feed (totals injection test).
async fn spawn_server_with_feed(
    db: sqlx::SqlitePool,
    feed: netwatch_api::LiveFeed,
) -> (std::path::PathBuf, watch::Sender<StatusSnapshot>) {
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
        if let Err(e) = netwatch_api::serve_unix(&sock_for_serve, rx, feed, db, async move {
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
    panic!("server did not start on {}", sock.display());
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
async fn subscribe_live_connections_returns_injected_snapshot_then_delta() {
    // P2.6: the live feed is shared state; a test publisher injects a
    // snapshot row and a delta (the daemon's live service does this in
    // production) and the WS client sees snapshot → delta, API.md §3.
    let dir = std::env::temp_dir().join(format!(
        "netwatch-api-live-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let sock = dir.join("api.sock");

    let (_status_tx, status_rx) = watch::channel(StatusSnapshot {
        daemon: DaemonState::Monitoring,
        ebpf: EbpfState::Attached,
        recording: true,
    });
    let row = netwatch_common::LiveConnectionRow {
        id: 88231,
        app: netwatch_common::LiveApp {
            id: 1,
            name: "Firefox".to_string(),
            icon: None,
        },
        pid: 2213,
        process_name: Some("firefox".to_string()),
        protocol: netwatch_common::Protocol::Tcp,
        local_ip: "192.168.1.23".to_string(),
        local_port: 53210,
        remote_ip: "151.101.1.1".to_string(),
        remote_port: 443,
        remote_host: None,
        rate_up_bps: 40_960,
        rate_down_bps: 262_144,
        bytes_up: 240_000,
        bytes_down: 1_980_000,
    };
    let (_snapshot_tx, snapshot_rx) = watch::channel(Arc::new(vec![row.clone()]));
    let (delta_tx, _) = tokio::sync::broadcast::channel(16);
    let (_totals_tx, totals_rx) = watch::channel(Arc::new(UsageTotals::default()));
    let live = netwatch_api::LiveFeed {
        snapshot: snapshot_rx,
        deltas: delta_tx.clone(),
        totals: totals_rx,
    };

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let serve_sock = sock.clone();
    let handle = tokio::spawn(async move {
        let db = test_db().await;
        let _ = netwatch_api::serve_unix(&serve_sock, status_rx, live, db, async move {
            let _ = shutdown_rx.await;
        })
        .await;
    });
    for _ in 0..100 {
        if UnixStream::connect(&sock).await.is_ok() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    let mut ws = ws_connect(&sock).await;
    ws_send(
        &mut ws,
        json!({ "op": "subscribe", "channel": "live.connections" }),
    )
    .await;

    // Snapshot carries the injected row (C1 shape).
    let v = ws_next(&mut ws).await;
    assert_eq!(v["kind"], "snapshot");
    assert_eq!(v["data"][0]["id"], 88231);
    assert_eq!(v["data"][0]["app"]["name"], "Firefox");
    assert_eq!(v["data"][0]["rate_down_bps"], 262_144);

    // Delta replaces by row id.
    delta_tx
        .send(netwatch_common::LiveDelta {
            added: vec![],
            updated: vec![netwatch_common::LiveUpdateRow {
                id: 88231,
                rate_up_bps: 0,
                rate_down_bps: 300_000,
                bytes_up: 240_000,
                bytes_down: 2_200_000,
            }],
            removed: vec![],
        })
        .unwrap();
    let v = ws_next(&mut ws).await;
    assert_eq!(v["kind"], "delta");
    assert_eq!(v["updated"][0]["id"], 88231);
    assert_eq!(v["updated"][0]["bytes_down"], 2_200_000);

    shutdown_tx.send(()).ok();
    handle.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn connections_live_rest_matches_ws_snapshot() {
    // P2.8 acceptance: REST snapshot == WS snapshot for the same feed.
    let dir = std::env::temp_dir().join(format!(
        "netwatch-api-c1-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let sock = dir.join("api.sock");

    let (_status_tx, status_rx) = watch::channel(StatusSnapshot {
        daemon: DaemonState::Monitoring,
        ebpf: EbpfState::Attached,
        recording: true,
    });
    let row = netwatch_common::LiveConnectionRow {
        id: 88231,
        app: netwatch_common::LiveApp {
            id: 1,
            name: "Firefox".to_string(),
            icon: None,
        },
        pid: 2213,
        process_name: Some("firefox".to_string()),
        protocol: netwatch_common::Protocol::Tcp,
        local_ip: "192.168.1.23".to_string(),
        local_port: 53210,
        remote_ip: "151.101.1.1".to_string(),
        remote_port: 443,
        remote_host: None,
        rate_up_bps: 40_960,
        rate_down_bps: 262_144,
        bytes_up: 240_000,
        bytes_down: 1_980_000,
    };
    let (_snapshot_tx, snapshot_rx) = watch::channel(Arc::new(vec![row]));
    let (delta_tx, _) = tokio::sync::broadcast::channel(16);
    let (_totals_tx, totals_rx) = watch::channel(Arc::new(UsageTotals::default()));
    let live = netwatch_api::LiveFeed {
        snapshot: snapshot_rx,
        deltas: delta_tx.clone(),
        totals: totals_rx,
    };

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let serve_sock = sock.clone();
    let handle = tokio::spawn(async move {
        let db = test_db().await;
        let _ = netwatch_api::serve_unix(&serve_sock, status_rx, live, db, async move {
            let _ = shutdown_rx.await;
        })
        .await;
    });
    for _ in 0..100 {
        if UnixStream::connect(&sock).await.is_ok() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // WS: subscribe and capture the snapshot data array.
    let mut ws = ws_connect(&sock).await;
    ws_send(
        &mut ws,
        json!({ "op": "subscribe", "channel": "live.connections" }),
    )
    .await;
    let ws_frame = ws_next(&mut ws).await;
    assert_eq!(ws_frame["kind"], "snapshot");

    // REST: same feed, same rows (C1 envelope).
    let (status, body) = http_get(&sock, "/api/v1/connections/live").await;
    assert_eq!(status, 200);
    let rest: Value = serde_json::from_str(&body).unwrap();
    assert!(rest["captured_at"].is_u64());
    assert_eq!(rest["frozen"], false);
    assert_eq!(rest["data"], ws_frame["data"], "REST == WS snapshot");

    shutdown_tx.send(()).ok();
    handle.abort();
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
async fn subscribe_usage_totals_streams_publisher_updates() {
    let (feed, _deltas, totals_tx) = live_feed_with_totals();
    let (sock, _tx) = spawn_server_with_feed(test_db().await, feed).await;
    let mut ws = ws_connect(&sock).await;
    ws_send(
        &mut ws,
        json!({ "op": "subscribe", "channel": "usage.totals" }),
    )
    .await;
    let initial = ws_next(&mut ws).await;
    assert_eq!(initial["channel"], "usage.totals");
    assert_eq!(initial["kind"], "update");
    assert_eq!(initial["data"]["today"], json!({ "up": 0, "down": 0 }));

    totals_tx.send_replace(Arc::new(UsageTotals {
        today: netwatch_common::DirectionTotals {
            up: 52_428_800,
            down: 934_000_000,
        },
    }));
    let update = ws_next(&mut ws).await;
    assert_eq!(update["channel"], "usage.totals");
    assert_eq!(
        update["data"]["today"],
        json!({ "up": 52_428_800, "down": 934_000_000 })
    );
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

/// Raw HTTP/1.1 GET over the Unix socket; returns (`status_code`, `body_text`).
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
        assert!(buf.len() <= 64 * 1024, "response exceeds 64 KiB bound");
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
    let body = text.split_once("\r\n\r\n").map_or("", |(_, b)| b);
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
    let state = Arc::new(AppState::new(rx, live_feed().0, test_db().await));
    state.client_delta(1);
    assert_eq!(state.client_count(), 1);
    state.client_delta(-1);
    assert_eq!(state.client_count(), 0);
}

/// Stale socket files are removed at bind (`serve_unix` contract).
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
    let (feed, _) = live_feed();
    let serve_sock = sock.clone();
    let handle = tokio::spawn(async move {
        let db = test_db().await;
        let _ = netwatch_api::serve_unix(&serve_sock, rx, feed, db, async move {
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

// ------------------------------------------- §2.2 C2 history (GET /connections)

/// Seed one app plus raw episodes inside the raw window.
async fn seed_raw_history(pool: &sqlx::SqlitePool, count: i64) {
    sqlx::query("INSERT INTO apps (id, name) VALUES (1, 'Firefox')")
        .execute(pool)
        .await
        .unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().cast_signed())
        .unwrap_or_default();
    for i in 0..count {
        sqlx::query(
            "INSERT INTO connections (app_id, pid, protocol, local_ip, local_port,
             remote_ip, remote_port, bytes_up, bytes_down, first_seen_at, last_seen_at)
             VALUES (1, 4242, 'tcp', '10.0.0.2', 51000, '151.101.1.1', 443, ?, 0, ?, ?)",
        )
        .bind(1000 + i)
        .bind(now - 3600 - i)
        .bind(now - 3600 - i)
        .execute(pool)
        .await
        .unwrap();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn connections_history_returns_envelope_over_socket() {
    let db = test_db().await;
    seed_history_schema(&db).await;
    seed_raw_history(&db, 3).await;
    let (sock, _tx) = spawn_server_with(db).await;

    let (status, body) = http_get(&sock, "/api/v1/connections").await;
    assert_eq!(status, 200, "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    // API.md §1.4 / C2 envelope.
    assert_eq!(v["has_more"], false);
    assert!(v["next_cursor"].is_null(), "omitted when has_more is false");
    let data = v["data"].as_array().unwrap();
    assert_eq!(data.len(), 3);
    // Raw rows carry an id and the C1 app block; only aggregates carry kind.
    assert!(data[0]["id"].as_i64().is_some());
    assert_eq!(data[0]["app"]["name"], "Firefox");
    assert!(data[0]["kind"].is_null());
    assert!(data[0]["packets_up"].as_i64().is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn connections_history_paginates_with_cursor_over_socket() {
    let db = test_db().await;
    seed_history_schema(&db).await;
    seed_raw_history(&db, 5).await;
    let (sock, _tx) = spawn_server_with(db).await;

    let (status, body) = http_get(&sock, "/api/v1/connections?limit=2").await;
    assert_eq!(status, 200, "body: {body}");
    let first: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(first["data"].as_array().unwrap().len(), 2);
    assert_eq!(first["has_more"], true);
    let cursor = first["next_cursor"].as_str().unwrap().to_string();

    // Walk the rest; the union must be 5 distinct ids in stable order.
    let mut ids: Vec<i64> = first["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_i64().unwrap())
        .collect();
    let mut next = Some(cursor);
    while let Some(c) = next {
        let (status, body) =
            http_get(&sock, &format!("/api/v1/connections?limit=2&cursor={c}")).await;
        assert_eq!(status, 200, "body: {body}");
        let page: Value = serde_json::from_str(&body).unwrap();
        ids.extend(
            page["data"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r["id"].as_i64().unwrap()),
        );
        next = page["next_cursor"].as_str().map(str::to_string);
        assert!(page["data"].as_array().unwrap().len() <= 2);
    }
    assert_eq!(ids.len(), 5, "no dupes, no gaps: {ids:?}");
    let mut unique = ids.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), 5, "cursor walk repeated a row: {ids:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn connections_history_rejects_unknown_param_with_400() {
    let db = test_db().await;
    seed_history_schema(&db).await;
    let (sock, _tx) = spawn_server_with(db).await;

    let (status, body) = http_get(&sock, "/api/v1/connections?bogus=1").await;
    assert_eq!(status, 400, "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["error"]["code"], "QUERY_INVALID");
    assert!(
        v["error"]["message"].as_str().unwrap().contains("bogus"),
        "message must name the offending key: {body}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn connection_by_id_over_socket_contract() {
    let db = test_db().await;
    seed_history_schema(&db).await;
    seed_raw_history(&db, 2).await;
    let (sock, _tx) = spawn_server_with(db).await;

    // Found: 200, C1 shape + packets.
    let (status, body) = http_get(&sock, "/api/v1/connections/1").await;
    assert_eq!(status, 200, "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["id"], 1);
    assert_eq!(v["app"]["name"], "Firefox");
    assert!(v["packets_up"].as_i64().is_some());

    // Fake id: 404 envelope, code NOT_FOUND (PLAN P3.4 verify criterion).
    let (status, body) = http_get(&sock, "/api/v1/connections/424242").await;
    assert_eq!(status, 404, "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["error"]["code"], "NOT_FOUND");

    // Non-numeric id: 404 (C3 has no 400 in its error list).
    let (status, body) = http_get(&sock, "/api/v1/connections/abc").await;
    assert_eq!(status, 404, "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["error"]["code"], "NOT_FOUND");
}

// ----------------------------------------------------- §2.5 U1 usage summary

async fn seed_usage_data(pool: &sqlx::SqlitePool) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .cast_signed();
    let bucket = now / 900 * 900;
    sqlx::query("INSERT INTO apps (id, name) VALUES (1, 'Firefox'), (2, 'Chrome')")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO usage_interface_15m (interface_id, bucket_start, rx_bytes, tx_bytes)
         VALUES (2, ?1, 1000, 500), (2, ?2, 100, 50)",
    )
    .bind(bucket)
    .bind(bucket + 900)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO usage_rollup_15m (app_id, bucket_start, bytes_up, bytes_down)
         VALUES (1, ?1, 800, 400), (2, ?1, 200, 100)",
    )
    .bind(bucket)
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn usage_summary_reports_totals_gap_and_top_apps() {
    let db = test_db().await;
    seed_history_schema(&db).await;
    seed_usage_schema(&db).await;
    seed_usage_data(&db).await;
    let (sock, _tx) = spawn_server_with(db).await;

    let (status, body) = http_get(&sock, "/api/v1/usage/summary").await;
    assert_eq!(status, 200, "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["state"], "ok");
    for period in ["today", "month"] {
        assert_eq!(v[period]["total"], 1_650, "{period}: {body}");
        assert_eq!(v[period]["attributed"], 1_500, "{period}: {body}");
        assert_eq!(v[period]["unattributed"], 150, "{period}: {body}");
    }
    assert_eq!(v["top_apps"].as_array().unwrap().len(), 2);
    assert_eq!(v["top_apps"][0]["name"], "Firefox");
    assert_eq!(v["top_apps"][0]["bytes_total"], 1_200);
    assert_eq!(v["top_apps"][1]["name"], "Chrome");
}

#[tokio::test(flavor = "multi_thread")]
async fn usage_summary_empty_db_is_collecting() {
    let db = test_db().await;
    seed_history_schema(&db).await;
    seed_usage_schema(&db).await;
    let (sock, _tx) = spawn_server_with(db).await;

    let (status, body) = http_get(&sock, "/api/v1/usage/summary").await;
    assert_eq!(status, 200, "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["state"], "collecting");
    assert_eq!(v["today"]["total"], 0);
    assert_eq!(v["month"]["unattributed"], 0);
    assert_eq!(v["top_apps"], json!([]));
}

#[tokio::test(flavor = "multi_thread")]
async fn usage_summary_rejects_invalid_top_limit() {
    let db = test_db().await;
    seed_history_schema(&db).await;
    seed_usage_schema(&db).await;
    let (sock, _tx) = spawn_server_with(db).await;

    for query in [
        "?top_limit=0",
        "?top_limit=11",
        "?top_limit=abc",
        "?bogus=1",
    ] {
        let (status, body) = http_get(&sock, &format!("/api/v1/usage/summary{query}")).await;
        assert_eq!(status, 400, "query {query}: {body}");
        let v: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["error"]["code"], "QUERY_INVALID");
    }
}

// ---------------------------------------------------- §2.5 U2 usage timeline

async fn set_timezone(pool: &sqlx::SqlitePool, name: &str) {
    sqlx::query("INSERT INTO settings (key, value) VALUES ('timezone', ?1)")
        .bind(name)
        .execute(pool)
        .await
        .unwrap();
}

async fn insert_rollup(pool: &sqlx::SqlitePool, bucket: i64, up: i64, down: i64, connections: i64) {
    sqlx::query(
        "INSERT INTO usage_rollup_15m (app_id, bucket_start, bytes_up, bytes_down, connection_count)
         VALUES (1, ?1, ?2, ?3, ?4)",
    )
    .bind(bucket)
    .bind(up)
    .bind(down)
    .bind(connections)
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn usage_timeline_merges_raw_and_rollup_at_seam() {
    let db = test_db().await;
    seed_history_schema(&db).await;
    seed_usage_schema(&db).await;
    set_timezone(&db, "UTC").await;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .cast_signed();
    let cutoff = now - 7 * 86_400;
    let since = cutoff - 7_200;
    let until = cutoff + 7_200;

    insert_rollup(&db, cutoff - 3_660, 100, 200, 3).await;
    // A stale rollup inside the raw window must not be double-counted.
    insert_rollup(&db, cutoff + 900, 9_999, 9_999, 9).await;
    sqlx::query(
        "INSERT INTO connections (app_id, pid, protocol, local_ip, local_port,
                                  remote_ip, remote_port, bytes_up, bytes_down,
                                  first_seen_at, last_seen_at)
         VALUES (1, 42, 'tcp', '10.0.0.2', 51000, '198.51.100.1', 443,
                 10, 20, ?1, ?2)",
    )
    .bind(cutoff)
    .bind(cutoff + 60)
    .execute(&db)
    .await
    .unwrap();

    let (sock, _tx) = spawn_server_with(db).await;
    let (status, body) = http_get(
        &sock,
        &format!("/api/v1/usage/timeline?granularity=hour&since={since}&until={until}"),
    )
    .await;
    assert_eq!(status, 200, "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["granularity"], "hour");
    let data = v["data"].as_array().unwrap();
    assert_eq!(data.len(), 2, "one rollup bucket + one raw bucket: {body}");
    let up: i64 = data.iter().map(|row| row["up"].as_i64().unwrap()).sum();
    let down: i64 = data.iter().map(|row| row["down"].as_i64().unwrap()).sum();
    let connections: i64 = data
        .iter()
        .map(|row| row["connections"].as_i64().unwrap())
        .sum();
    assert_eq!((up, down, connections), (110, 220, 4));
}

#[tokio::test(flavor = "multi_thread")]
async fn usage_timeline_groups_15m_rollups_into_local_days() {
    let db = test_db().await;
    seed_history_schema(&db).await;
    seed_usage_schema(&db).await;
    set_timezone(&db, "Asia/Kathmandu").await;
    let tz: chrono_tz::Tz = "Asia/Kathmandu".parse().unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .cast_signed();
    let day = netwatch_api::calendar::bucket_start(
        now - 30 * 86_400,
        netwatch_api::calendar::Bucket::Day,
        tz,
    );
    insert_rollup(&db, day, 10, 20, 1).await;
    insert_rollup(&db, day + 900, 30, 40, 2).await;
    insert_rollup(&db, day + 1_800, 50, 60, 3).await;

    let (sock, _tx) = spawn_server_with(db).await;
    let (status, body) = http_get(
        &sock,
        &format!(
            "/api/v1/usage/timeline?granularity=day&since={}&until={}",
            day - 86_400,
            day + 86_400
        ),
    )
    .await;
    assert_eq!(status, 200, "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    let data = v["data"].as_array().unwrap();
    assert_eq!(
        data.len(),
        1,
        "three 15m rows must form one local day: {body}"
    );
    assert_eq!(data[0]["bucket_start"], day);
    assert_eq!(data[0]["up"], 90);
    assert_eq!(data[0]["down"], 120);
    assert_eq!(data[0]["connections"], 6);
}

#[tokio::test(flavor = "multi_thread")]
async fn usage_timeline_interface_filter_uses_counter_table() {
    let db = test_db().await;
    seed_history_schema(&db).await;
    seed_usage_schema(&db).await;
    set_timezone(&db, "UTC").await;
    sqlx::query(
        "INSERT INTO usage_interface_15m (interface_id, bucket_start, rx_bytes, tx_bytes)
         VALUES (2, ?1, 500, 100)",
    )
    .bind(1_800_000_000 / 900 * 900)
    .execute(&db)
    .await
    .unwrap();

    let (sock, _tx) = spawn_server_with(db).await;
    let (status, body) = http_get(
        &sock,
        "/api/v1/usage/timeline?granularity=hour&since=1799900000&until=1800100000&interface_id=2",
    )
    .await;
    assert_eq!(status, 200, "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    let data = v["data"].as_array().unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data[0]["up"], 100, "tx_bytes is upload");
    assert_eq!(data[0]["down"], 500, "rx_bytes is download");
    assert_eq!(data[0]["connections"], 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn usage_timeline_rejects_invalid_queries() {
    let db = test_db().await;
    seed_history_schema(&db).await;
    seed_usage_schema(&db).await;
    let (sock, _tx) = spawn_server_with(db).await;
    for query in [
        "?granularity=week",
        "?since=100&until=100",
        "?since=200&until=100",
        "?limit=0",
        "?bogus=1",
    ] {
        let (status, body) = http_get(&sock, &format!("/api/v1/usage/timeline{query}")).await;
        assert_eq!(status, 400, "query {query}: {body}");
        let v: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["error"]["code"], "QUERY_INVALID");
    }
}

async fn seed_interfaces(pool: &sqlx::SqlitePool) {
    sqlx::query(
        "CREATE TABLE interfaces (
            id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE,
            kind TEXT NOT NULL DEFAULT 'ethernet',
            is_metered INTEGER NOT NULL DEFAULT 0,
            is_excluded INTEGER NOT NULL DEFAULT 0)",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO interfaces (id, name, kind, is_metered, is_excluded) VALUES
            (1, 'lo', 'loopback', 0, 1),
            (2, 'wlan0', 'wifi', 1, 0),
            (3, 'eth0', 'ethernet', 0, 0)",
    )
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn usage_interfaces_excludes_loopback_and_sums_counters() {
    let db = test_db().await;
    seed_history_schema(&db).await;
    seed_usage_schema(&db).await;
    seed_interfaces(&db).await;
    set_timezone(&db, "UTC").await;
    sqlx::query(
        "INSERT INTO usage_interface_15m (interface_id, bucket_start, rx_bytes, tx_bytes) VALUES
            (1, ?1, 999_999, 999_999),
            (2, ?1, 500, 100), (2, ?2, 50, 10),
            (3, ?1, 20, 5)",
    )
    .bind(1_800_000_000)
    .bind(1_800_000_900)
    .execute(&db)
    .await
    .unwrap();

    let (sock, _tx) = spawn_server_with(db).await;
    let (status, body) = http_get(
        &sock,
        "/api/v1/usage/interfaces?since=1700000000&until=1900000000",
    )
    .await;
    assert_eq!(status, 200, "body: {body}");
    let rows: Value = serde_json::from_str(&body).unwrap();
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 2, "lo must never appear: {body}");
    assert_eq!(rows[0]["id"], 2);
    assert_eq!(rows[0]["name"], "wlan0");
    assert_eq!(rows[0]["kind"], "wifi");
    assert_eq!(rows[0]["is_metered"], true);
    assert_eq!(rows[0]["bytes"]["up"], 110);
    assert_eq!(rows[0]["bytes"]["down"], 550);
    assert_eq!(rows[1]["id"], 3);
    assert_eq!(rows[1]["bytes"]["up"], 5);
    assert_eq!(rows[1]["bytes"]["down"], 20);
}

#[tokio::test(flavor = "multi_thread")]
async fn usage_interfaces_rejects_invalid_queries() {
    let db = test_db().await;
    seed_history_schema(&db).await;
    seed_usage_schema(&db).await;
    seed_interfaces(&db).await;
    let (sock, _tx) = spawn_server_with(db).await;
    for query in ["?since=100&until=100", "?since=200&until=100", "?bogus=1"] {
        let (status, body) = http_get(&sock, &format!("/api/v1/usage/interfaces{query}")).await;
        assert_eq!(status, 400, "query {query}: {body}");
        let v: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["error"]["code"], "QUERY_INVALID");
    }
}

// ------------------------------------------------------- §2.4 A1 apps ranking

async fn seed_apps_for_ranking(pool: &sqlx::SqlitePool) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .cast_signed();
    set_timezone(pool, "UTC").await;
    sqlx::query(
        "INSERT INTO apps (id, name, icon_path, attribution_confidence, first_seen_at, last_seen_at, deleted_at) VALUES
            (1, 'Firefox', '/icons/firefox.png', 'exact', ?1, ?1, NULL),
            (2, 'Chrome', NULL, 'heuristic', ?1, ?2, NULL),
            (3, 'Old App', NULL, 'exact', ?1, ?1, ?1)",
    )
    .bind(now - 86_400)
    .bind(now - 60)
    .execute(pool)
    .await
    .unwrap();
    let bucket = now / 900 * 900;
    sqlx::query(
        "INSERT INTO usage_rollup_15m (app_id, bucket_start, bytes_up, bytes_down) VALUES
            (1, ?1, 100, 200), (2, ?1, 300, 50), (3, ?1, 900, 900)",
    )
    .bind(bucket)
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn apps_ranking_matches_summary_and_hides_deleted() {
    let db = test_db().await;
    seed_history_schema(&db).await;
    seed_usage_schema(&db).await;
    seed_apps_for_ranking(&db).await;
    let (sock, _tx) = spawn_server_with(db).await;

    let (status, body) = http_get(&sock, "/api/v1/apps?sort=usage_month").await;
    assert_eq!(status, 200, "body: {body}");
    let rows: Value = serde_json::from_str(&body).unwrap();
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 2, "soft-deleted app hidden by default");
    assert_eq!(rows[0]["name"], "Chrome", "350 > 300");
    assert_eq!(rows[0]["usage"]["month"]["up"], 300);
    assert_eq!(rows[0]["usage"]["today"]["down"], 50);
    assert_eq!(rows[0]["attribution_confidence"], "heuristic");
    assert_eq!(rows[1]["icon"], "ic:1", "filesystem path must not leak");

    let (_, summary_body) = http_get(&sock, "/api/v1/usage/summary?top_limit=5").await;
    let summary: Value = serde_json::from_str(&summary_body).unwrap();
    let top_ids: Vec<i64> = summary["top_apps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|app| app["id"].as_i64().unwrap())
        .collect();
    let app_ids: Vec<i64> = rows.iter().map(|app| app["id"].as_i64().unwrap()).collect();
    assert_eq!(top_ids, app_ids, "A1 ranking must match U1 top apps");

    let (status, body) = http_get(&sock, "/api/v1/apps?include_deleted=true").await;
    assert_eq!(status, 200, "body: {body}");
    let rows: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 3);
    assert_eq!(rows[0]["name"], "Old App");
}

#[tokio::test(flavor = "multi_thread")]
async fn apps_ranking_sorts_by_today_and_last_seen() {
    let db = test_db().await;
    seed_history_schema(&db).await;
    seed_usage_schema(&db).await;
    seed_apps_for_ranking(&db).await;
    let (sock, _tx) = spawn_server_with(db).await;

    let (_, body) = http_get(&sock, "/api/v1/apps?sort=usage_today").await;
    let rows: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(rows[0]["name"], "Chrome");

    let (_, body) = http_get(&sock, "/api/v1/apps?sort=last_seen_at").await;
    let rows: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(rows[0]["name"], "Chrome", "most recent first");
    assert_eq!(rows[1]["name"], "Firefox");

    let (_, body) = http_get(&sock, "/api/v1/apps?sort=last_seen_at&order=asc").await;
    let rows: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(rows[0]["name"], "Firefox");
}

#[tokio::test(flavor = "multi_thread")]
async fn apps_ranking_rejects_invalid_queries() {
    let db = test_db().await;
    seed_history_schema(&db).await;
    seed_usage_schema(&db).await;
    let (sock, _tx) = spawn_server_with(db).await;
    for query in [
        "?sort=bogus",
        "?order=sideways",
        "?limit=0",
        "?limit=201",
        "?include_deleted=maybe",
        "?bogus=1",
    ] {
        let (status, body) = http_get(&sock, &format!("/api/v1/apps{query}")).await;
        assert_eq!(status, 400, "query {query}: {body}");
        let v: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["error"]["code"], "QUERY_INVALID");
    }
}

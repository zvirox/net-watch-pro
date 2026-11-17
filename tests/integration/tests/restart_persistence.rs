//! P3.6: history survives an abrupt daemon stop and restart (TESTING.md F5).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::Value;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::process::{Child, Command};

const START_TIMEOUT: Duration = Duration::from_secs(5);
const HTTP_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_RESPONSE_BYTES: usize = 64 * 1024;

struct TestPaths {
    root: PathBuf,
    data_dir: PathBuf,
    socket: PathBuf,
    database: PathBuf,
}

impl TestPaths {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "netwatch-restart-test-{}-{nonce}",
            std::process::id()
        ));
        let data_dir = root.join("data");
        std::fs::create_dir_all(&data_dir).unwrap();
        Self {
            database: data_dir.join("netwatch.db"),
            socket: root.join("api.sock"),
            root,
            data_dir,
        }
    }
}

impl Drop for TestPaths {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

struct DaemonProcess {
    child: Child,
    pid: u32,
}

impl DaemonProcess {
    async fn start(paths: &TestPaths) -> Self {
        let mut child = Command::new(netwatchd_binary())
            .env("NETWATCH_DATA_DIR", &paths.data_dir)
            .env("NETWATCH_API_SOCKET", &paths.socket)
            .env("NETWATCH_LOG_LEVEL", "warn")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .expect("failed to spawn netwatchd");
        let pid = child.id().expect("spawned daemon has no pid");
        wait_until_ready(&mut child, &paths.socket).await;
        Self { child, pid }
    }

    async fn kill_and_wait(&mut self) {
        self.child
            .kill()
            .await
            .expect("failed to kill and reap netwatchd");
    }
}

impl Drop for DaemonProcess {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

fn netwatchd_binary() -> PathBuf {
    let test_binary = std::env::current_exe().unwrap();
    let target_profile_dir = test_binary
        .parent()
        .and_then(Path::parent)
        .expect("integration test is not under a Cargo profile directory");
    let binary = target_profile_dir.join(format!("netwatchd{}", std::env::consts::EXE_SUFFIX));
    assert!(
        binary.is_file(),
        "netwatchd binary not found at {}; run `cargo build -p netwatchd` first",
        binary.display()
    );
    binary
}

async fn wait_until_ready(child: &mut Child, socket: &Path) {
    for _ in 0..100 {
        if let Some(status) = child.try_wait().unwrap() {
            panic!("netwatchd exited with {status} before opening its socket");
        }
        if UnixStream::connect(socket).await.is_ok() {
            return;
        }
        tokio::time::sleep(START_TIMEOUT / 100).await;
    }
    panic!(
        "netwatchd did not open {} within {START_TIMEOUT:?}",
        socket.display()
    );
}

async fn seed_history(database: &Path) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(SqliteConnectOptions::new().filename(database))
        .await
        .unwrap();
    let now = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    )
    .unwrap();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query(
        "INSERT INTO apps (id, name, app_key, exe_path, first_seen_at, last_seen_at)
         VALUES (1, 'Firefox', 'exe:firefox', '/usr/bin/firefox', ?1, ?1)",
    )
    .bind(now - 400)
    .execute(&mut *tx)
    .await
    .unwrap();
    for index in 0..3_i64 {
        sqlx::query(
            "INSERT INTO connections (
                app_id, pid, process_name, protocol, local_ip, local_port,
                remote_ip, remote_port, remote_host, bytes_up, bytes_down,
                packets_up, packets_down, first_seen_at, last_seen_at, is_open
            ) VALUES (1, 4242, 'firefox', 'tcp', '10.0.0.2', ?1,
                      '151.101.1.1', 443, 'example.com', ?2, ?3, 4, 6, ?4, ?5, 0)",
        )
        .bind(51_000 + index)
        .bind(1_000 + index)
        .bind(9_000 + index)
        .bind(now - 300 - index)
        .bind(now - 200 - index)
        .execute(&mut *tx)
        .await
        .unwrap();
    }
    tx.commit().await.unwrap();
    pool.close().await;
}

async fn http_get(socket: &Path, path: &str) -> (u16, String) {
    tokio::time::timeout(HTTP_TIMEOUT, http_get_inner(socket, path))
        .await
        .expect("timed out waiting for an API response")
}

async fn http_get_inner(socket: &Path, path: &str) -> (u16, String) {
    let mut stream = UnixStream::connect(socket).await.unwrap();
    let request = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = Vec::new();
    stream
        .take(u64::try_from(MAX_RESPONSE_BYTES).unwrap() + 1)
        .read_to_end(&mut response)
        .await
        .unwrap();
    assert!(
        response.len() <= MAX_RESPONSE_BYTES,
        "API response too large"
    );
    let response = String::from_utf8(response).unwrap();
    let status = response
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse().ok())
        .expect("HTTP response has no status");
    let body = response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.to_string())
        .unwrap_or_default();
    (status, body)
}

#[tokio::test(flavor = "multi_thread")]
async fn history_is_identical_after_daemon_kill_and_restart() {
    let paths = TestPaths::new();
    let mut first = DaemonProcess::start(&paths).await;
    seed_history(&paths.database).await;

    let (before_status, before_body) = http_get(&paths.socket, "/api/v1/connections").await;
    assert_eq!(before_status, 200, "before restart: {before_body}");
    let before_json: Value = serde_json::from_str(&before_body).unwrap();
    assert_eq!(before_json["data"].as_array().unwrap().len(), 3);
    assert_eq!(before_json["data"][0]["app"]["name"], "Firefox");

    let first_pid = first.pid;
    first.kill_and_wait().await;
    let mut second = DaemonProcess::start(&paths).await;
    assert_ne!(second.pid, first_pid, "test must use a new daemon process");

    let (after_status, after_body) = http_get(&paths.socket, "/api/v1/connections").await;
    assert_eq!(after_status, 200, "after restart: {after_body}");
    let after_json: Value = serde_json::from_str(&after_body).unwrap();
    assert_eq!(after_json, before_json, "history changed across restart");
    assert_eq!(
        after_body, before_body,
        "serialized history changed across restart"
    );

    let (row_status, row_body) = http_get(&paths.socket, "/api/v1/connections/1").await;
    assert_eq!(
        row_status, 200,
        "restarted daemon cannot query episode 1: {row_body}"
    );
    let row: Value = serde_json::from_str(&row_body).unwrap();
    assert_eq!(row["id"], 1);
    assert_eq!(row["bytes_down"], 9_000);

    second.kill_and_wait().await;
}

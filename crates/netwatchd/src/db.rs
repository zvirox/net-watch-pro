//! Daemon database layer: SQLite open + forward-only migrations (P2.4).
//!
//! DATABASE.md §8 runtime rules: `PRAGMA foreign_keys=ON`, `journal_mode=WAL`,
//! `synchronous=NORMAL` set on every connection open; migrations embedded in
//! the binary; the daemon refuses to start on an unknown/newer schema
//! (honest degraded state, not silent best-effort).

use std::path::Path;
use std::time::Duration;

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use thiserror::Error;

/// Database-layer errors (ARCHITECTURE.md §7.1: typed errors per crate).
#[derive(Debug, Error)]
pub enum DbError {
    /// Opening the database file failed (unwritable dir, corruption).
    #[error("cannot open database: {0}")]
    Open(#[from] sqlx::Error),
    /// The database holds migrations this binary does not know (newer
    /// daemon previously ran here) — refuse to start (DATABASE.md §8).
    #[error("schema version mismatch (newer than this build); refusing to start: {0}")]
    Schema(#[from] sqlx::migrate::MigrateError),
}

/// Open (creating if missing) the daemon database with the §8 pragmas set on
/// every pooled connection.
pub async fn open(path: &Path) -> Result<SqlitePool, DbError> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(5));

    // Small pool: the daemon is a single writer (batched, §5.1 step 3);
    // extra connections only serve read queries.
    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await?;
    Ok(pool)
}

/// Apply the embedded forward-only migrations (DATABASE.md §8). A database
/// written by a newer daemon makes this fail with [`DbError::Schema`] —
/// sqlx refuses unknown migration versions, which is exactly the §8 rule.
pub async fn migrate(pool: &SqlitePool) -> Result<(), DbError> {
    sqlx::migrate!("./src/db/migrations").run(pool).await?;
    Ok(())
}

/// Highest persisted connection id, for seeding the live id space
/// (`crate::live` starts episode ids above it so the two never collide).
///
/// # Errors
///
/// Propagates SQLite failures (API.md S2 `DB_ERROR` class).
pub async fn max_connection_id(pool: &SqlitePool) -> Result<u64, DbError> {
    let max: Option<i64> = sqlx::query_scalar("SELECT MAX(id) FROM connections")
        .fetch_one(pool)
        .await?;
    Ok(max.unwrap_or(0).max(0) as u64)
}

/// Persist a batch of episode rows in one transaction (F-09: closes persist
/// immediately, open episodes on the ~60 s checkpoint; every write is
/// transactional, §7.1 — partial flushes are never persisted). Absolute
/// totals: upsert, never accumulate.
///
/// The caller stamps `updated_at`-equivalent cadence via `last_seen_at`;
/// `created_at` keeps its insert default.
///
/// # Errors
///
/// Propagates SQLite failures; the live service logs and continues on
/// batch failure (degraded, not dead — §7.1).
pub async fn persist_episodes(
    pool: &SqlitePool,
    rows: &[netwatch_core::episode::EpisodeRow],
) -> Result<(), DbError> {
    if rows.is_empty() {
        return Ok(());
    }
    let mut tx = pool.begin().await?;
    for row in rows {
        sqlx::query(
            "INSERT INTO connections (
                id, app_id, interface_id, pid, process_name, protocol,
                local_ip, local_port, remote_ip, remote_port, remote_host,
                bytes_up, bytes_down, packets_up, packets_down,
                first_seen_at, last_seen_at, is_open
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
            ON CONFLICT(id) DO UPDATE SET
                bytes_up = excluded.bytes_up,
                bytes_down = excluded.bytes_down,
                packets_up = excluded.packets_up,
                packets_down = excluded.packets_down,
                last_seen_at = excluded.last_seen_at,
                is_open = excluded.is_open,
                updated_at = strftime('%s','now')",
        )
        .bind(row.id as i64)
        .bind(row.app_id)
        .bind(row.interface_id)
        .bind(row.pid as i64)
        .bind(row.process_name.as_deref())
        .bind(row.protocol.as_str())
        .bind(&row.local_ip)
        .bind(row.local_port as i64)
        .bind(&row.remote_ip)
        .bind(row.remote_port as i64)
        .bind(row.remote_host.as_deref())
        .bind(row.bytes_up as i64)
        .bind(row.bytes_down as i64)
        .bind(row.packets_up as i64)
        .bind(row.packets_down as i64)
        .bind(row.first_seen_at as i64)
        .bind(row.last_seen_at as i64)
        .bind(i64::from(row.is_open))
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)] // tests, not daemon paths

    use super::*;

    /// Fresh database → migrations 0001+0002 applied, tables + seeds exist
    /// (P2.4 acceptance, DATABASE.md §8).
    #[tokio::test]
    async fn fresh_db_migrates_with_seeds() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .in_memory(true)
                    .foreign_keys(true)
                    .journal_mode(SqliteJournalMode::Wal)
                    .synchronous(SqliteSynchronous::Normal),
            )
            .await
            .unwrap();
        migrate(&pool).await.unwrap();

        // All migration 0001 seeds present.
        let settings: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM settings")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(settings, 7); // closed key list, DATABASE.md §2.6

        let (tier,): (String,) = sqlx::query_as("SELECT tier FROM license WHERE id = 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(tier, "free");

        let (lo_excluded,): (i64,) =
            sqlx::query_as("SELECT is_excluded FROM interfaces WHERE name = 'lo'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(lo_excluded, 1);

        // Migration 0002 tables exist.
        let (apps,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='apps'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(apps, 1);

        let (connections,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='connections'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(connections, 1);

        // All migrations recorded in the human ledger (DATABASE.md §2.1):
        // 0001 init_core, 0002 apps_connections, 0003 rollups.
        let rows: Vec<(i64, String)> =
            sqlx::query_as("SELECT version, name FROM schema_migrations ORDER BY version")
                .fetch_all(&pool)
                .await
                .unwrap();
        println!("schema_migrations rows: {rows:?}");
        assert_eq!(rows.len(), 3);
        // Rollup tables + the deferred remote-host index exist (§2.5/§5).
        let rollups: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table'
             AND name IN ('usage_rollup_15m','usage_endpoint_day','usage_interface_15m')",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(rollups, 3);
    }

    /// §8 runtime rules hold on a fresh open: WAL + foreign_keys ON.
    #[tokio::test]
    async fn pragmas_applied() {
        let dir = std::env::temp_dir().join(format!(
            "netwatchd-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("test.db");

        let pool = open(&db_path).await.unwrap();
        let (fk,): (i64,) = sqlx::query_as("PRAGMA foreign_keys")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(fk, 1);
        let (journal,): (String,) = sqlx::query_as("PRAGMA journal_mode")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(journal.to_lowercase(), "wal");

        pool.close().await;
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P2.5: episode flush persists a row and later batches upsert absolute
    /// totals (never accumulate). FK requires a seeded app row — the real
    /// app upsert path lands with attribution (P2.3).
    #[tokio::test]
    async fn persist_episodes_upserts_absolute_totals() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(SqliteConnectOptions::new().in_memory(true))
            .await
            .unwrap();
        migrate(&pool).await.unwrap();
        sqlx::query("INSERT INTO apps (id, name, app_key, exe_path, first_seen_at, last_seen_at) VALUES (1, 'Firefox', 'cgroup:test', '/usr/bin/firefox', 0, 0)")
            .execute(&pool)
            .await
            .unwrap();

        let row = netwatch_core::episode::EpisodeRow {
            id: 7,
            app_id: 1,
            interface_id: Some(1),
            pid: 2213,
            process_name: Some("firefox".to_string()),
            protocol: netwatch_common::Protocol::Tcp,
            local_ip: "192.168.1.23".to_string(),
            local_port: 53210,
            remote_ip: "151.101.1.1".to_string(),
            remote_port: 443,
            remote_host: None,
            bytes_up: 100,
            bytes_down: 200,
            packets_up: 1,
            packets_down: 2,
            first_seen_at: 1000,
            last_seen_at: 1005,
            is_open: true,
        };

        persist_episodes(&pool, &[row.clone()]).await.unwrap();
        // Second batch with higher totals → replaced, not accumulated.
        let mut grown = row.clone();
        grown.bytes_up = 400;
        grown.bytes_down = 900;
        grown.last_seen_at = 1060;
        persist_episodes(&pool, &[grown]).await.unwrap();

        let (up, down, last_seen, is_open): (i64, i64, i64, i64) = sqlx::query_as(
            "SELECT bytes_up, bytes_down, last_seen_at, is_open FROM connections WHERE id = 7",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!((up, down), (400, 900), "absolute upsert, not accumulate");
        assert_eq!(last_seen, 1060);
        assert_eq!(is_open, 1);
        assert_eq!(max_connection_id(&pool).await.unwrap(), 7);
    }
}

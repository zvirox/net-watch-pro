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

        // Both migrations recorded in the human ledger (DATABASE.md §2.1).
        let rows: Vec<(i64, String)> =
            sqlx::query_as("SELECT version, name FROM schema_migrations ORDER BY version")
                .fetch_all(&pool)
                .await
                .unwrap();
        println!("schema_migrations rows: {rows:?}");
        assert_eq!(rows.len(), 2);
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
}

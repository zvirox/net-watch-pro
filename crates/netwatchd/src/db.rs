//! Daemon database layer: SQLite open + forward-only migrations (P2.4).
//!
//! DATABASE.md §8 runtime rules: `PRAGMA foreign_keys=ON`, `journal_mode=WAL`,
//! `synchronous=NORMAL` set on every connection open; migrations embedded in
//! the binary; the daemon refuses to start on an unknown/newer schema
//! (honest degraded state, not silent best-effort).

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::Path;
use std::time::Duration;

use netwatch_common::DaemonEvent;
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

/// Append one immutable diagnostics event and keep the newest 5,000 rows
/// (DATABASE.md §2.8). A failed event write never replaces the original
/// degraded-state handling; callers log the write failure.
///
/// # Errors
///
/// Propagates SQLite failures (including an over-long message).
pub async fn record_event(pool: &SqlitePool, now: i64, event: &DaemonEvent) -> Result<(), DbError> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO daemon_events (
            occurred_at, level, code, message, detail_json, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?1)",
    )
    .bind(now)
    .bind(event.level.as_str())
    .bind(&event.code)
    .bind(&event.message)
    .bind(event.detail_json.as_deref())
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "DELETE FROM daemon_events
          WHERE id NOT IN (SELECT id FROM daemon_events ORDER BY id DESC LIMIT 5000)",
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
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
    Ok(u64::try_from(max.unwrap_or(0).max(0)).unwrap_or(u64::MAX))
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
        .bind(i64::try_from(row.id).unwrap_or(i64::MAX))
        .bind(row.app_id)
        .bind(row.interface_id)
        .bind(i64::from(row.pid))
        .bind(row.process_name.as_deref())
        .bind(row.protocol.as_str())
        .bind(&row.local_ip)
        .bind(i64::from(row.local_port))
        .bind(&row.remote_ip)
        .bind(i64::from(row.remote_port))
        .bind(row.remote_host.as_deref())
        .bind(i64::try_from(row.bytes_up).unwrap_or(i64::MAX))
        .bind(i64::try_from(row.bytes_down).unwrap_or(i64::MAX))
        .bind(i64::try_from(row.packets_up).unwrap_or(i64::MAX))
        .bind(i64::try_from(row.packets_down).unwrap_or(i64::MAX))
        .bind(i64::try_from(row.first_seen_at).unwrap_or(i64::MAX))
        .bind(i64::try_from(row.last_seen_at).unwrap_or(i64::MAX))
        .bind(i64::from(row.is_open))
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Soft-delete apps whose last activity predates `stale_before`
/// (DATABASE.md §2.2: `deleted_at` is set when pruning removes the last
/// activity; the row is never deleted so history still resolves names).
///
/// Returns the number of apps newly marked. Idempotent: already deleted apps
/// are left untouched. `deleted_at` is caller-supplied so the pruner and its
/// tests share one clock value.
///
/// # Errors
///
/// Propagates SQLite failures (API.md S2 `DB_ERROR` class).
// P7.1's pruner will call this on its schedule; until then only the P3.7
// test exercises it.
#[cfg_attr(not(test), allow(dead_code))]
pub async fn soft_delete_stale_apps(
    pool: &SqlitePool,
    stale_before: i64,
    deleted_at: i64,
) -> Result<u64, DbError> {
    let result = sqlx::query(
        "UPDATE apps
            SET deleted_at = ?1, updated_at = ?1
          WHERE deleted_at IS NULL AND last_seen_at < ?2",
    )
    .bind(deleted_at)
    .bind(stale_before)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

/// Result of one compaction pass (DATABASE.md §2.5/§2.5a).
#[derive(Debug, Default, PartialEq, Eq)]
pub struct CompactionReport {
    /// Raw episodes replaced by rollups.
    pub compacted_connections: usize,
    /// 15-minute rollup rows written.
    pub rollup_rows: usize,
    /// Endpoint-day rows written (including the folded `other` row).
    pub endpoint_rows: usize,
}

#[derive(Debug, sqlx::FromRow)]
struct RawConnection {
    app_id: i64,
    interface_name: Option<String>,
    remote_ip: String,
    remote_port: i64,
    protocol: String,
    bytes_up: i64,
    bytes_down: i64,
    first_seen_at: i64,
    last_seen_at: i64,
}

#[derive(Debug)]
struct Rollup {
    app_id: i64,
    bucket_start: i64,
    bytes_up: i64,
    bytes_down: i64,
    connection_count: i64,
    interfaces: BTreeSet<String>,
}

impl Rollup {
    fn new(row: &RawConnection) -> Self {
        let mut rollup = Self {
            app_id: row.app_id,
            bucket_start: netwatch_core::retention::bucket_15m(row.last_seen_at),
            bytes_up: 0,
            bytes_down: 0,
            connection_count: 0,
            interfaces: BTreeSet::new(),
        };
        rollup.add(row);
        rollup
    }

    fn add(&mut self, row: &RawConnection) {
        self.bytes_up = self.bytes_up.saturating_add(row.bytes_up);
        self.bytes_down = self.bytes_down.saturating_add(row.bytes_down);
        self.connection_count = self.connection_count.saturating_add(1);
        if let Some(name) = &row.interface_name {
            self.interfaces.insert(name.clone());
        }
    }

    fn interfaces_mask(&self) -> String {
        self.interfaces
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct EndpointKey {
    app_id: i64,
    bucket_start: i64,
    remote_ip: String,
    remote_port: i64,
    protocol: String,
}

#[derive(Debug, Clone)]
struct EndpointAggregate {
    key: EndpointKey,
    bytes_up: i64,
    bytes_down: i64,
    connection_count: i64,
    first_seen_at: i64,
    last_seen_at: i64,
}

impl EndpointAggregate {
    fn new(row: &RawConnection) -> Self {
        let key = EndpointKey {
            app_id: row.app_id,
            bucket_start: netwatch_core::retention::bucket_day(row.last_seen_at),
            remote_ip: row.remote_ip.clone(),
            remote_port: row.remote_port,
            protocol: row.protocol.clone(),
        };
        Self {
            key,
            bytes_up: 0,
            bytes_down: 0,
            connection_count: 0,
            first_seen_at: i64::MAX,
            last_seen_at: i64::MIN,
        }
        .merged_row(row)
    }

    fn merged_row(mut self, row: &RawConnection) -> Self {
        self.bytes_up = self.bytes_up.saturating_add(row.bytes_up);
        self.bytes_down = self.bytes_down.saturating_add(row.bytes_down);
        self.connection_count = self.connection_count.saturating_add(1);
        self.first_seen_at = self.first_seen_at.min(row.first_seen_at);
        self.last_seen_at = self.last_seen_at.max(row.last_seen_at);
        self
    }

    fn merge(&mut self, other: &Self) {
        self.bytes_up = self.bytes_up.saturating_add(other.bytes_up);
        self.bytes_down = self.bytes_down.saturating_add(other.bytes_down);
        self.connection_count = self.connection_count.saturating_add(other.connection_count);
        self.first_seen_at = self.first_seen_at.min(other.first_seen_at);
        self.last_seen_at = self.last_seen_at.max(other.last_seen_at);
    }

    fn bytes_total(&self) -> i64 {
        self.bytes_up.saturating_add(self.bytes_down)
    }
}

/// Compact every raw episode older than `raw_window_days` in one transaction
/// (P4.1, DATABASE.md §2.5): aggregate into 15-minute buckets and capped
/// endpoint-day rows, then delete the raw rows. Never partially persists.
///
/// # Errors
///
/// Propagates SQLite failures (API.md S2 `DB_ERROR` class).
pub async fn compact_old_connections(
    pool: &SqlitePool,
    now: i64,
    endpoint_cap: usize,
) -> Result<CompactionReport, DbError> {
    let cutoff = raw_window_cutoff(pool, now).await?;
    let mut tx = pool.begin().await?;
    let rows = fetch_expired_rows(&mut tx, cutoff).await?;
    if rows.is_empty() {
        return Ok(CompactionReport::default());
    }

    let rollups = build_rollups(&rows);
    let endpoints = build_endpoint_rows(&rows, endpoint_cap);
    upsert_rollups(&mut tx, &rollups).await?;
    upsert_endpoints(&mut tx, &endpoints).await?;
    sqlx::query("DELETE FROM connections WHERE last_seen_at < ?1")
        .bind(cutoff)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(CompactionReport {
        compacted_connections: rows.len(),
        rollup_rows: rollups.len(),
        endpoint_rows: endpoints.len(),
    })
}

async fn raw_window_cutoff(pool: &SqlitePool, now: i64) -> Result<i64, DbError> {
    let window: Option<String> =
        sqlx::query_scalar("SELECT value FROM settings WHERE key = 'raw_window_days'")
            .fetch_optional(pool)
            .await?;
    let window = window
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(7);
    Ok(netwatch_core::retention::raw_cutoff(now, window))
}

async fn fetch_expired_rows(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    cutoff: i64,
) -> Result<Vec<RawConnection>, DbError> {
    let rows = sqlx::query_as(
        "SELECT c.app_id AS app_id,
                CASE WHEN i.is_excluded = 0 THEN i.name END AS interface_name,
                c.remote_ip AS remote_ip, c.remote_port AS remote_port,
                c.protocol AS protocol, c.bytes_up AS bytes_up,
                c.bytes_down AS bytes_down, c.first_seen_at AS first_seen_at,
                c.last_seen_at AS last_seen_at
           FROM connections c LEFT JOIN interfaces i ON i.id = c.interface_id
          WHERE c.last_seen_at < ?1 ORDER BY c.id",
    )
    .bind(cutoff)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows)
}

fn build_rollups(rows: &[RawConnection]) -> BTreeMap<(i64, i64), Rollup> {
    let mut rollups = BTreeMap::new();
    for row in rows {
        let key = (
            row.app_id,
            netwatch_core::retention::bucket_15m(row.last_seen_at),
        );
        rollups
            .entry(key)
            .and_modify(|rollup: &mut Rollup| rollup.add(row))
            .or_insert_with(|| Rollup::new(row));
    }
    rollups
}

fn build_endpoint_rows(rows: &[RawConnection], cap: usize) -> Vec<EndpointAggregate> {
    let mut by_day: BTreeMap<(i64, i64), Vec<(EndpointKey, EndpointAggregate)>> = BTreeMap::new();
    for row in rows {
        let aggregate = EndpointAggregate::new(row);
        let day = (aggregate.key.app_id, aggregate.key.bucket_start);
        by_day
            .entry(day)
            .or_default()
            .push((aggregate.key.clone(), aggregate));
    }
    let mut result = Vec::new();
    for endpoints in by_day.into_values() {
        result.extend(fold_endpoints(endpoints, cap));
    }
    result
}

fn fold_endpoints(
    mut endpoints: Vec<(EndpointKey, EndpointAggregate)>,
    cap: usize,
) -> Vec<EndpointAggregate> {
    let cap = cap.max(1);
    endpoints.sort_by(|(left_key, left), (right_key, right)| {
        right
            .bytes_total()
            .cmp(&left.bytes_total())
            .then_with(|| left_key.cmp(right_key))
    });
    if endpoints.len() <= cap {
        return endpoints.into_iter().map(|(_, row)| row).collect();
    }
    let mut kept: Vec<EndpointAggregate> = endpoints[..cap]
        .iter()
        .map(|(_, row)| row.clone())
        .collect();
    let mut other = EndpointAggregate {
        key: EndpointKey {
            app_id: endpoints[0].0.app_id,
            bucket_start: endpoints[0].0.bucket_start,
            remote_ip: "other".to_string(),
            remote_port: 0,
            protocol: "other".to_string(),
        },
        bytes_up: 0,
        bytes_down: 0,
        connection_count: 0,
        first_seen_at: i64::MAX,
        last_seen_at: i64::MIN,
    };
    for (_, row) in &endpoints[cap..] {
        other.merge(row);
    }
    kept.push(other);
    kept
}

async fn upsert_rollups(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    rollups: &BTreeMap<(i64, i64), Rollup>,
) -> Result<(), DbError> {
    for rollup in rollups.values() {
        sqlx::query(
            "INSERT INTO usage_rollup_15m (
                app_id, bucket_start, bytes_up, bytes_down, connection_count, interfaces_mask
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(app_id, bucket_start) DO UPDATE SET
                bytes_up = usage_rollup_15m.bytes_up + excluded.bytes_up,
                bytes_down = usage_rollup_15m.bytes_down + excluded.bytes_down,
                connection_count = usage_rollup_15m.connection_count + excluded.connection_count,
                interfaces_mask = excluded.interfaces_mask",
        )
        .bind(rollup.app_id)
        .bind(rollup.bucket_start)
        .bind(rollup.bytes_up)
        .bind(rollup.bytes_down)
        .bind(rollup.connection_count)
        .bind(rollup.interfaces_mask())
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn upsert_endpoints(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    endpoints: &[EndpointAggregate],
) -> Result<(), DbError> {
    for row in endpoints {
        sqlx::query(
            "INSERT INTO usage_endpoint_day (
                app_id, bucket_start, remote_ip, remote_port, protocol,
                bytes_up, bytes_down, connection_count, first_seen_at, last_seen_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(app_id, bucket_start, remote_ip, remote_port, protocol) DO UPDATE SET
                bytes_up = usage_endpoint_day.bytes_up + excluded.bytes_up,
                bytes_down = usage_endpoint_day.bytes_down + excluded.bytes_down,
                connection_count = usage_endpoint_day.connection_count + excluded.connection_count,
                first_seen_at = MIN(usage_endpoint_day.first_seen_at, excluded.first_seen_at),
                last_seen_at = MAX(usage_endpoint_day.last_seen_at, excluded.last_seen_at)",
        )
        .bind(row.key.app_id)
        .bind(row.key.bucket_start)
        .bind(&row.key.remote_ip)
        .bind(row.key.remote_port)
        .bind(&row.key.protocol)
        .bind(row.bytes_up)
        .bind(row.bytes_down)
        .bind(row.connection_count)
        .bind(row.first_seen_at)
        .bind(row.last_seen_at)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

/// Names the retention policy excludes from headline totals
/// (DATABASE.md §2.3/§2.5b). Kept as a set so a per-sample DB query is not
/// needed for every sysfs directory entry.
///
/// # Errors
///
/// Propagates SQLite failures (API.md S2 `DB_ERROR` class).
pub async fn excluded_interface_names(pool: &SqlitePool) -> Result<HashSet<String>, DbError> {
    let names: Vec<String> =
        sqlx::query_scalar("SELECT name FROM interfaces WHERE is_excluded = 1")
            .fetch_all(pool)
            .await?;
    Ok(names.into_iter().collect())
}

/// Persist one batch of interface-counter deltas in a single transaction
/// (ARCHITECTURE.md §7.1: every DB write is transactional).
///
/// New interfaces are registered on first sight; existing rows keep their
/// metered/excluded flags. Bucket rows accumulate deltas from repeated
/// samples.
///
/// # Errors
///
/// Propagates SQLite failures (API.md S2 `DB_ERROR` class).
pub async fn persist_interface_samples(
    pool: &SqlitePool,
    samples: &[netwatch_core::interfaces::InterfaceSample],
) -> Result<(), DbError> {
    if samples.is_empty() {
        return Ok(());
    }
    let mut tx = pool.begin().await?;
    for sample in samples {
        let kind = netwatch_core::interfaces::interface_kind(&sample.name);
        sqlx::query(
            "INSERT INTO interfaces (name, kind) VALUES (?1, ?2)
             ON CONFLICT(name) DO NOTHING",
        )
        .bind(&sample.name)
        .bind(kind)
        .execute(&mut *tx)
        .await?;
        let interface_id: i64 = sqlx::query_scalar("SELECT id FROM interfaces WHERE name = ?1")
            .bind(&sample.name)
            .fetch_one(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO usage_interface_15m (
                interface_id, bucket_start, rx_bytes, tx_bytes
             ) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(interface_id, bucket_start) DO UPDATE SET
                rx_bytes = usage_interface_15m.rx_bytes + excluded.rx_bytes,
                tx_bytes = usage_interface_15m.tx_bytes + excluded.tx_bytes",
        )
        .bind(interface_id)
        .bind(sample.bucket_start)
        .bind(i64::try_from(sample.rx_bytes).unwrap_or(i64::MAX))
        .bind(i64::try_from(sample.tx_bytes).unwrap_or(i64::MAX))
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
    use netwatch_common::EventLevel;

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
        // 0001 init_core, 0002 apps_connections, 0003 rollups, 0004 daemon_events.
        let rows: Vec<(i64, String)> =
            sqlx::query_as("SELECT version, name FROM schema_migrations ORDER BY version")
                .fetch_all(&pool)
                .await
                .unwrap();
        println!("schema_migrations rows: {rows:?}");
        assert_eq!(rows.len(), 4);
        assert_eq!(
            rows.last().map(|(_, name)| name.as_str()),
            Some("daemon_events")
        );
        // Rollup tables + the deferred remote-host index exist (§2.5/§5).
        let rollups: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table'
             AND name IN ('usage_rollup_15m','usage_endpoint_day','usage_interface_15m')",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(rollups, 3);
        let (events_table, events_index): (i64, i64) = sqlx::query_as(
            "SELECT
                (SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='daemon_events'),
                (SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_daemon_events_occurred')",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!((events_table, events_index), (1, 1));
    }

    /// §8 runtime rules hold on a fresh open: WAL + `foreign_keys` ON.
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

        persist_episodes(&pool, std::slice::from_ref(&row))
            .await
            .unwrap();
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

    /// P3.7: stale apps are marked deleted, never removed; the history
    /// endpoint still resolves their display names (TESTING.md §4 F2/F4).
    #[tokio::test]
    async fn soft_deleted_app_still_resolves_in_history() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(SqliteConnectOptions::new().in_memory(true))
            .await
            .unwrap();
        migrate(&pool).await.unwrap();
        for (id, name, last_seen) in [(1, "Old App", 100), (2, "Current App", 900)] {
            sqlx::query(
                "INSERT INTO apps (id, name, app_key, exe_path, first_seen_at, last_seen_at)
                 VALUES (?1, ?2, ?3, '/usr/bin/test', ?4, ?5)",
            )
            .bind(id)
            .bind(name)
            .bind(format!("exe:{name}"))
            .bind(last_seen)
            .bind(last_seen)
            .execute(&pool)
            .await
            .unwrap();
        }
        for (id, app_id) in [(1, 1), (2, 2)] {
            sqlx::query(
                "INSERT INTO connections (
                    id, app_id, pid, protocol, local_ip, local_port,
                    remote_ip, remote_port, first_seen_at, last_seen_at
                 ) VALUES (?1, ?2, 4242, 'tcp', '10.0.0.2', 51000,
                           '151.101.1.1', 443, 100, 900)",
            )
            .bind(id)
            .bind(app_id)
            .execute(&pool)
            .await
            .unwrap();
        }

        assert_eq!(soft_delete_stale_apps(&pool, 500, 1_000).await.unwrap(), 1);
        // Idempotent: a second pruner pass changes nothing.
        assert_eq!(soft_delete_stale_apps(&pool, 500, 1_100).await.unwrap(), 0);

        let apps: Vec<(i64, Option<i64>)> =
            sqlx::query_as("SELECT id, deleted_at FROM apps ORDER BY id")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(apps, vec![(1, Some(1_000)), (2, None)]);
        assert_eq!(apps.len(), 2, "soft delete must not remove app rows");

        let page = netwatch_api::history::connections_page(&pool, "", 1_000)
            .await
            .unwrap();
        let names: Vec<&str> = page.data.iter().map(|row| row.app.name.as_str()).collect();
        assert!(
            names.contains(&"Old App"),
            "history lost the stale app: {names:?}"
        );
        assert!(
            names.contains(&"Current App"),
            "history lost the active app: {names:?}"
        );
    }

    /// Seed 60 expired episodes across distinct endpoints and 15m buckets.
    /// Returns the expected uplink sum used by the total-preservation check.
    async fn seed_compaction_fixture(pool: &sqlx::SqlitePool, now: i64) -> i64 {
        sqlx::query("INSERT INTO apps (id, name, app_key, exe_path, first_seen_at, last_seen_at) VALUES (1, 'Firefox', 'exe:firefox', '/usr/bin/firefox', 0, 0)")
            .execute(pool).await.unwrap();
        sqlx::query("INSERT INTO interfaces (id, name) VALUES (2, 'eth0')")
            .execute(pool)
            .await
            .unwrap();
        let base = now - 8 * netwatch_core::retention::DAY_S;
        let mut expected_up = 0_i64;
        for index in 0..60_i64 {
            let up = index + 1;
            expected_up += up;
            let last_seen = base + index * netwatch_core::retention::BUCKET_15M_S;
            sqlx::query(
                "INSERT INTO connections (
                    app_id, interface_id, pid, protocol, local_ip, local_port,
                    remote_ip, remote_port, bytes_up, bytes_down,
                    first_seen_at, last_seen_at, is_open
                 ) VALUES (1, 2, 4242, 'tcp', '10.0.0.2', 51000,
                           '198.51.100.1', ?1, ?2, ?3, ?4, ?5, 0)",
            )
            .bind(5_000 + index)
            .bind(up)
            .bind(up * 2)
            .bind(last_seen - 60)
            .bind(last_seen)
            .execute(pool)
            .await
            .unwrap();
        }
        expected_up
    }

    /// P4.1: old raw rows become 15m/endpoint rollups without changing
    /// totals; overflow endpoints fold into one `other` row (DATABASE.md §2.5a).
    #[tokio::test]
    async fn compaction_preserves_totals_and_caps_endpoints() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(SqliteConnectOptions::new().in_memory(true))
            .await
            .unwrap();
        migrate(&pool).await.unwrap();
        let now = 2_000_000_000_i64;
        let expected_up = seed_compaction_fixture(&pool, now).await;

        let report =
            compact_old_connections(&pool, now, netwatch_core::retention::DEFAULT_ENDPOINT_CAP)
                .await
                .unwrap();
        assert_eq!(report.compacted_connections, 60);
        assert_eq!(report.rollup_rows, 60);
        assert_eq!(
            report.endpoint_rows,
            netwatch_core::retention::DEFAULT_ENDPOINT_CAP + 1
        );
        let raw: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM connections")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(raw, 0, "raw rows must be pruned after rollups are written");

        let (up, down, count, mask): (i64, i64, i64, String) = sqlx::query_as(
            "SELECT SUM(bytes_up), SUM(bytes_down), SUM(connection_count),
                    COALESCE(GROUP_CONCAT(DISTINCT interfaces_mask), '')
               FROM usage_rollup_15m",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!((up, down, count), (expected_up, expected_up * 2, 60));
        assert!(mask.contains("eth0"), "interface mask missing eth0: {mask}");
        assert!(!mask.contains("lo"), "excluded lo leaked into mask: {mask}");

        let (other_up, other_count): (i64, i64) = sqlx::query_as(
            "SELECT bytes_up, connection_count FROM usage_endpoint_day
              WHERE app_id = 1 AND remote_ip = 'other' AND protocol = 'other'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        // The 10 smallest endpoints (1..=10) are folded into the `other` row.
        assert_eq!((other_up, other_count), (55, 10));

        let top: i64 = sqlx::query_scalar(
            "SELECT bytes_up FROM usage_endpoint_day
              WHERE app_id = 1 AND remote_port = 5059 AND protocol = 'tcp'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(top, 60);

        // Re-running is a no-op; a fresh row stays in the raw window.
        assert_eq!(
            compact_old_connections(&pool, now, 50).await.unwrap(),
            CompactionReport::default()
        );
        sqlx::query(
            "INSERT INTO connections (
                app_id, pid, protocol, local_ip, local_port, remote_ip,
                remote_port, first_seen_at, last_seen_at
             ) VALUES (1, 4242, 'tcp', '10.0.0.2', 51000, '203.0.113.1', 443, ?1, ?2)",
        )
        .bind(now - 120)
        .bind(now - 60)
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(
            compact_old_connections(&pool, now, 50).await.unwrap(),
            CompactionReport::default()
        );
        let raw: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM connections")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(raw, 1);
    }

    /// P6.1: diagnostics events are appended as immutable rows and the
    /// newest-5,000 ring is enforced (DATABASE.md §2.8).
    #[tokio::test]
    async fn daemon_events_are_bounded() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(SqliteConnectOptions::new().in_memory(true))
            .await
            .unwrap();
        migrate(&pool).await.unwrap();
        for index in 0..5_001 {
            record_event(
                &pool,
                1_000 + index,
                &DaemonEvent {
                    level: EventLevel::Warn,
                    code: "EBPF_ATTACH_FAILED".to_string(),
                    message: format!("attach failed {index}"),
                    detail_json: None,
                },
            )
            .await
            .unwrap();
        }
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM daemon_events")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 5_000);
        let (code, level, occurred_at, created_at): (String, String, i64, i64) = sqlx::query_as(
            "SELECT code, level, occurred_at, created_at
                   FROM daemon_events ORDER BY id DESC LIMIT 1",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(code, "EBPF_ATTACH_FAILED");
        assert_eq!(level, "warn");
        assert_eq!(occurred_at, 6_000);
        assert_eq!(created_at, 6_000);
        let oldest: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM daemon_events WHERE message = 'attach failed 0'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(oldest, 0, "oldest row must be pruned");
    }
}

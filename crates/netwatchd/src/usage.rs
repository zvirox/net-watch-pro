//! Headline totals publisher (P4.8, API.md §3 `usage.totals`).
//!
//! Reuses U3's timezone-aware day query so the streamed numbers match the
//! REST dashboard exactly. Publishes at most one frame per second; a failed
//! sample keeps the previous totals and logs a warning (§7.1 degraded path).

use std::sync::Arc;
use std::time::Duration;

use netwatch_common::{DirectionTotals, UsageTotals};
use sqlx::SqlitePool;
use tokio::sync::watch;
use tracing::warn;

const PUBLISH_INTERVAL: Duration = Duration::from_secs(1);

/// Spawn the publisher. Returns the receiver's matching sender for the API
/// feed plus an abort handle for daemon shutdown.
pub fn spawn(pool: SqlitePool) -> (watch::Sender<Arc<UsageTotals>>, tokio::task::AbortHandle) {
    let (tx, _) = watch::channel(Arc::new(UsageTotals::default()));
    let publisher = tx.clone();
    let handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(PUBLISH_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            let now = epoch_now();
            match read_totals(&pool, now).await {
                Ok(totals) => {
                    publisher.send_replace(Arc::new(totals));
                }
                Err(e) => {
                    warn!("usage totals sample failed; keeping last values: {e}");
                    crate::record_degraded_event(
                        &pool,
                        now,
                        "USAGE_TOTALS_FAILED",
                        format!("usage totals sample failed; keeping last values: {e}"),
                    )
                    .await;
                }
            }
        }
    });
    let abort = handle.abort_handle();
    (tx, abort)
}

async fn read_totals(pool: &SqlitePool, now: i64) -> Result<UsageTotals, String> {
    let rows = netwatch_api::interfaces_usage::interfaces_usage(pool, now, None)
        .await
        .map_err(|e| e.to_string())?;
    let mut today = DirectionTotals::default();
    for row in rows {
        today.up = today
            .up
            .saturating_add(u64::try_from(row.bytes.up).unwrap_or(u64::MAX));
        today.down = today
            .down
            .saturating_add(u64::try_from(row.bytes.down).unwrap_or(u64::MAX));
    }
    Ok(UsageTotals { today })
}

fn epoch_now() -> i64 {
    i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    )
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)] // tests, not daemon paths

    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    #[tokio::test]
    async fn totals_match_non_excluded_interface_counters() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(SqliteConnectOptions::new().in_memory(true))
            .await
            .unwrap();
        crate::db::migrate(&pool).await.unwrap();
        sqlx::query("UPDATE settings SET value = 'UTC' WHERE key = 'timezone'")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO interfaces (id, name, kind, is_metered) VALUES
                (2, 'wlan0', 'wifi', 1)",
        )
        .execute(&pool)
        .await
        .unwrap();
        let now = 2_000_000_000_i64;
        let bucket = netwatch_core::retention::bucket_15m(now);
        sqlx::query(
            "INSERT INTO usage_interface_15m (interface_id, bucket_start, rx_bytes, tx_bytes) VALUES
                (2, ?1, 500, 100), (2, ?2, 50, 10), (1, ?1, 9999, 9999)",
        )
        .bind(bucket)
        .bind(bucket + 900)
        .execute(&pool)
        .await
        .unwrap();

        let totals = read_totals(&pool, now).await.unwrap();
        assert_eq!(totals.today.up, 110);
        assert_eq!(totals.today.down, 550);
    }
}

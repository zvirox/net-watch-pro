//! Scheduled retention pruner (P7.1, DATABASE.md §6).
//!
//! Runs once at startup and then hourly. A failed pass is a degraded state:
//! it records a `daemon_events` warning and lets the next tick retry, never
//! killing the daemon (ARCHITECTURE.md §7.1).

use std::time::Duration;

use sqlx::SqlitePool;
use tokio::task::JoinHandle;

/// Hourly cadence; retention is a slow-moving policy, not a hot path.
const INTERVAL: Duration = Duration::from_hours(1);

/// Spawn the recurring prune loop. The returned handle is aborted on shutdown.
#[must_use]
pub fn spawn(pool: SqlitePool) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let now = crate::epoch_now();
            match crate::db::prune_expired(&pool, now).await {
                Ok(report)
                    if report.connections_deleted > 0
                        || report.endpoint_days_deleted > 0
                        || report.apps_soft_deleted > 0 =>
                {
                    tracing::info!(
                        connections = report.connections_deleted,
                        endpoint_days = report.endpoint_days_deleted,
                        apps = report.apps_soft_deleted,
                        retention_days = report.retention_days,
                        "retention prune complete"
                    );
                }
                Ok(_) => {}
                Err(error) => {
                    let message = format!("retention prune failed: {error}");
                    tracing::warn!("{message}");
                    crate::record_degraded_event(&pool, now, "RETENTION_PRUNE_FAILED", message)
                        .await;
                }
            }
            tokio::time::sleep(INTERVAL).await;
        }
    })
}

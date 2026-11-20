//! Phase C (C6): persist eBPF per-app byte deltas to the rollup tables.
//!
//! The collector publishes 1 Hz `AppUsage` rows (cumulative bytes + deltas).
//! This task accumulates the deltas at checkpoint cadence (~60 s,
//! ARCHITECTURE.md §5.1 step 3) and writes them in one SQLite transaction.
//! A failed flush is degraded (logged + diagnostics event), never fatal.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use netwatch_common::AppUsage;
use netwatch_common::app_names::AppNames;
use sqlx::SqlitePool;
use tokio::sync::watch;
use tokio::task::JoinHandle;

const CHECKPOINT: Duration = Duration::from_secs(60);

/// Spawn the checkpoint writer. `rx` is the collector's 1 Hz app-usage feed;
/// `names` is the shared cgroup-id -> display-name resolver.
pub fn spawn(
    pool: SqlitePool,
    mut rx: watch::Receiver<Arc<Vec<AppUsage>>>,
    names: Arc<AppNames>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut accumulated: BTreeMap<u64, (u64, u64)> = BTreeMap::new();
        let mut tick = tokio::time::interval(CHECKPOINT);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                changed = rx.changed() => {
                    if changed.is_err() {
                        tracing::info!("app-usage feed closed; checkpoint writer stopping");
                        break;
                    }
                    for row in rx.borrow_and_update().iter() {
                        // AppUsage.down_bps is bytes-per-second * 8 for the
                        // 1 s collector interval; convert back to bytes.
                        let down = row.down_bps / 8;
                        let up = row.up_bps / 8;
                        let entry = accumulated.entry(row.cgroup_id).or_insert((0, 0));
                        entry.0 = entry.0.saturating_add(down);
                        entry.1 = entry.1.saturating_add(up);
                    }
                }
                _ = tick.tick() => {
                    if accumulated.is_empty() {
                        continue;
                    }
                    let now = crate::epoch_now();
                    let mut rows = Vec::with_capacity(accumulated.len());
                    for (cgroup_id, (down, up)) in &accumulated {
                        if *down == 0 && *up == 0 {
                            continue;
                        }
                        // Represent the app with its resolved display name at
                        // checkpoint time; no connection detail is invented.
                        let resolved = names.resolve(*cgroup_id, 0, 0);
                        rows.push((*cgroup_id, *down, *up, resolved.name, resolved.confidence));
                    }
                    match crate::db::persist_app_bytes(&pool, &rows, now).await {
                        Ok(()) => {
                            tracing::debug!(apps = rows.len(), "app-usage checkpoint written");
                            accumulated.clear();
                        }
                        Err(error) => {
                            // Degraded, not dead (§7.1): keep accumulating so
                            // the next checkpoint upserts absolute totals.
                            let message = format!("app-usage checkpoint failed: {error}");
                            tracing::warn!("{message}");
                            crate::record_degraded_event(
                                &pool,
                                now,
                                "APP_USAGE_WRITE_FAILED",
                                message,
                            ).await;
                        }
                    }
                }
            }
        }
    })
}

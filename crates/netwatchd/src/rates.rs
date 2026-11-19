//! 1 Hz PC live-speed sampler (API.md §3 `net.rates`, live plan A3).
//!
//! Reads the same sysfs counters as the 15 s history sampler, but converts
//! deltas into bits/second for the Dashboard "Network now" card. It never
//! writes SQLite. Read failures are degraded events, never daemon crashes.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use sqlx::SqlitePool;
use tokio::sync::watch;
use tokio::task::JoinHandle;

use netwatch_common::RateSnapshot;
use netwatch_core::rates::RateMeter;

/// Live cadence — one frame per second while a client is subscribed.
const SAMPLE_INTERVAL: Duration = Duration::from_secs(1);
const SYSFS_ROOT: &str = "/sys/class/net";

/// Spawn the rate task; the caller owns the receiver and abort handle.
#[must_use]
pub fn spawn(pool: SqlitePool, rates_tx: watch::Sender<Arc<RateSnapshot>>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut meter = RateMeter::default();
        let mut interval = tokio::time::interval(SAMPLE_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let start = std::time::Instant::now();
        loop {
            interval.tick().await;
            let elapsed_s = start.elapsed().as_secs_f64();
            let now = crate::epoch_now();
            match sample_once(
                Path::new(SYSFS_ROOT),
                &pool,
                &mut meter,
                elapsed_s,
                u64::try_from(now).unwrap_or(0),
            )
            .await
            {
                Ok(snapshot) => {
                    let _ = rates_tx.send(Arc::new(snapshot));
                }
                Err(error) => {
                    let message = format!("rate sampling failed: {error}");
                    tracing::warn!("{message}");
                    crate::record_degraded_event(&pool, now, "RATE_SAMPLE_FAILED", message).await;
                }
            }
        }
    })
}

/// Read one sysfs snapshot and compute a rate snapshot.
///
/// The `root` parameter is injectable so tests can use a synthetic sysfs tree.
async fn sample_once(
    root: &Path,
    pool: &SqlitePool,
    meter: &mut RateMeter,
    elapsed_s: f64,
    ts: u64,
) -> Result<RateSnapshot, String> {
    let excluded = crate::db::excluded_interface_names(pool)
        .await
        .map_err(|error| format!("cannot read excluded interfaces: {error}"))?;
    let entries = std::fs::read_dir(root)
        .map_err(|error| format!("cannot read {}: {error}", root.display()))?;

    let mut readings = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "lo" || excluded.contains(&name) || !entry.path().is_dir() {
            continue;
        }
        let dir = entry.path().join("statistics");
        let rx = read_counter(&dir.join("rx_bytes"))?;
        let tx = read_counter(&dir.join("tx_bytes"))?;
        readings.push((name, rx, tx));
    }

    let mut snapshot = meter.observe(elapsed_s, &readings);
    snapshot.ts = ts;
    Ok(snapshot)
}

fn read_counter(path: &Path) -> Result<u64, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    raw.trim()
        .parse::<u64>()
        .map_err(|error| format!("invalid counter {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)] // tests

    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    async fn test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(SqliteConnectOptions::new().in_memory(true))
            .await
            .unwrap();
        crate::db::migrate(&pool).await.unwrap();
        pool
    }

    fn fixture(root: &Path, name: &str, rx: u64, tx: u64) {
        let dir = root.join(name).join("statistics");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("rx_bytes"), rx.to_string()).unwrap();
        std::fs::write(dir.join("tx_bytes"), tx.to_string()).unwrap();
    }

    #[tokio::test]
    async fn rate_sampling_uses_sysfs_and_sets_timestamp() {
        let dir = std::env::temp_dir().join(format!(
            "nwp-rates-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        fixture(&dir, "lo", 999, 999);
        fixture(&dir, "wlan0", 100, 50);
        fixture(&dir, "eth0", 5_000, 5_000);
        let pool = test_db().await;
        sqlx::query(
            "INSERT INTO interfaces (name, kind, is_metered, is_excluded)
             VALUES ('eth0', 'ethernet', 0, 1)",
        )
        .execute(&pool)
        .await
        .unwrap();

        let mut meter = RateMeter::default();
        let first = sample_once(&dir, &pool, &mut meter, 0.0, 1_700)
            .await
            .unwrap();
        assert_eq!(
            first.interfaces.len(),
            1,
            "lo and excluded eth0 are skipped"
        );
        assert_eq!(first.interfaces[0].name, "wlan0");

        fixture(&dir, "wlan0", 1_000_100, 50);
        let second = sample_once(&dir, &pool, &mut meter, 1.0, 1_701)
            .await
            .unwrap();
        assert_eq!(second.ts, 1_701);
        assert!(second.interfaces[0].down_bps > 7_000_000);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

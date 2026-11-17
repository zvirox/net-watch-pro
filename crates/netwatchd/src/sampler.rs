//! Periodic interface-counter sampler (ARCHITECTURE.md §5.1 step 4).
//!
//! Cumulative counters come from sysfs; [`InterfaceSampler`] turns them into
//! deltas, and the DB layer writes one row per interface and 15-minute UTC
//! bucket. Loopback (and any interface marked excluded) is never sampled.

use std::fs;
use std::path::Path;
use std::time::Duration;

use netwatch_core::interfaces::{InterfaceCounters, InterfaceSampler};
use sqlx::SqlitePool;
use tracing::warn;

use crate::db::{self, DbError};

/// sysfs statistics root in production (ARCHITECTURE.md §5.1).
const SYSFS_ROOT: &str = "/sys/class/net";

/// Sampling cadence. Counter deltas accumulate into the current 15-minute
/// bucket; the headline API never reads sysfs directly.
const SAMPLE_INTERVAL: Duration = Duration::from_secs(15);

/// Spawn the sampler task. Returns an abort handle so daemon shutdown can stop
/// it alongside the API server.
pub fn spawn(pool: SqlitePool) -> tokio::task::AbortHandle {
    tokio::spawn(async move {
        let mut state = InterfaceSampler::new();
        let mut interval = tokio::time::interval(SAMPLE_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            let now = epoch_now();
            if let Err(e) = sample_once(&pool, Path::new(SYSFS_ROOT), &mut state, now).await {
                // Degraded, not dead (ARCHITECTURE.md §7.1): a failed sample
                // leaves earlier totals intact and the next tick retries.
                warn!("interface sampling failed; totals unchanged: {e}");
            }
        }
    })
    .abort_handle()
}

/// Read one sysfs snapshot, compute deltas, and persist them.
///
/// `root` is injectable so tests can use a synthetic sysfs tree without
/// touching the host kernel. The caller supplies `now` in epoch seconds to
/// keep bucket alignment deterministic.
async fn sample_once(
    pool: &SqlitePool,
    root: &Path,
    state: &mut InterfaceSampler,
    now: i64,
) -> Result<usize, DbError> {
    let excluded = db::excluded_interface_names(pool).await?;
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(e) => {
            warn!(root = %root.display(), error = %e, "cannot read sysfs net directory");
            return Ok(0);
        }
    };

    let mut samples = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "lo" || excluded.contains(&name) || !entry.path().is_dir() {
            continue;
        }
        match read_counters(&entry.path()) {
            Ok(counters) => {
                if let Some(sample) = state.observe(name, counters, now) {
                    samples.push(sample);
                }
            }
            Err(e) => warn!(interface = %name, error = %e, "cannot read interface counters"),
        }
    }
    db::persist_interface_samples(pool, &samples).await?;
    Ok(samples.len())
}

fn read_counters(interface_dir: &Path) -> Result<InterfaceCounters, std::io::Error> {
    Ok(InterfaceCounters {
        rx_bytes: read_stat(interface_dir, "rx_bytes")?,
        tx_bytes: read_stat(interface_dir, "tx_bytes")?,
    })
}

fn read_stat(interface_dir: &Path, name: &str) -> Result<u64, std::io::Error> {
    let path = interface_dir.join("statistics").join(name);
    let raw = fs::read_to_string(&path)?;
    raw.trim().parse::<u64>().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("invalid {name} in {}: {e}", path.display()),
        )
    })
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
    use netwatch_core::retention::bucket_15m;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    struct SysfsFixture {
        root: std::path::PathBuf,
    }

    impl SysfsFixture {
        fn new() -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "netwatch-sysfs-test-{}-{nonce}",
                std::process::id()
            ));
            for name in ["eth0", "lo"] {
                fs::create_dir_all(root.join(name).join("statistics")).unwrap();
            }
            write_counters(&root, "eth0", 100, 50);
            write_counters(&root, "lo", 9_999, 9_999);
            Self { root }
        }
    }

    impl Drop for SysfsFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn write_counters(root: &std::path::Path, name: &str, rx: u64, tx: u64) {
        let dir = root.join(name).join("statistics");
        fs::write(dir.join("rx_bytes"), rx.to_string()).unwrap();
        fs::write(dir.join("tx_bytes"), tx.to_string()).unwrap();
    }

    async fn test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(SqliteConnectOptions::new().in_memory(true))
            .await
            .unwrap();
        db::migrate(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn sampler_writes_deltas_and_skips_loopback() {
        let pool = test_db().await;
        let fixture = SysfsFixture::new();
        let mut state = InterfaceSampler::new();

        assert_eq!(
            sample_once(&pool, &fixture.root, &mut state, 0)
                .await
                .unwrap(),
            0,
            "first observation must be a baseline"
        );

        write_counters(&fixture.root, "eth0", 1_000, 500);
        assert_eq!(
            sample_once(&pool, &fixture.root, &mut state, 901)
                .await
                .unwrap(),
            1
        );

        let rows: Vec<(String, i64, i64, i64)> = sqlx::query_as(
            "SELECT i.name, u.bucket_start, u.rx_bytes, u.tx_bytes
               FROM usage_interface_15m u JOIN interfaces i ON i.id = u.interface_id
              ORDER BY i.name",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(rows, vec![("eth0".to_string(), bucket_15m(901), 900, 450)]);
        let (kind,): (String,) = sqlx::query_as("SELECT kind FROM interfaces WHERE name = 'eth0'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(kind, "ethernet");

        // Unchanged counters produce no new traffic.
        assert_eq!(
            sample_once(&pool, &fixture.root, &mut state, 902)
                .await
                .unwrap(),
            0
        );
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM usage_interface_15m")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }
}

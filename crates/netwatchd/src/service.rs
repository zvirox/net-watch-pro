//! Systemd startup enablement sync (P7.7).
//!
//! The root daemon owns the service unit, so it can apply `start_on_boot`
//! without extra admin prompts (D1a). The setting is polled; only actual
//! changes invoke systemctl, and failures are degraded events, not crashes.

use std::process::Command;
use std::time::Duration;

use sqlx::SqlitePool;
use tokio::task::JoinHandle;

/// Poll cadence for the startup setting.
const INTERVAL: Duration = Duration::from_secs(60);

/// Spawn the settings-sync loop. The handle is aborted on daemon shutdown.
#[must_use]
pub fn spawn(pool: SqlitePool) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut last: Option<bool> = None;
        loop {
            match crate::db::setting(&pool, "start_on_boot").await {
                Ok(raw) => {
                    let enabled = raw.and_then(|value| value.parse().ok()).unwrap_or(true);
                    if Some(enabled) != last {
                        match apply(enabled) {
                            Ok(()) => {
                                tracing::info!(enabled, "start_on_boot synced to systemd");
                                last = Some(enabled);
                            }
                            Err(error) => {
                                let message = format!("start_on_boot sync failed: {error}");
                                tracing::warn!("{message}");
                                crate::record_degraded_event(
                                    &pool,
                                    crate::epoch_now(),
                                    "START_ON_BOOT_SYNC_FAILED",
                                    message,
                                )
                                .await;
                            }
                        }
                    }
                }
                Err(error) => {
                    tracing::warn!("cannot read start_on_boot setting: {error}");
                }
            }
            tokio::time::sleep(INTERVAL).await;
        }
    })
}

/// Run the matching systemctl action for the desired enablement.
fn apply(enabled: bool) -> Result<(), String> {
    let action = netwatch_core::settings::start_on_boot_action(enabled);
    let status = Command::new("systemctl")
        .arg(action)
        .arg(netwatch_core::settings::SERVICE_UNIT)
        .status()
        .map_err(|error| format!("systemctl unavailable: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("systemctl {action} exited with {status}"))
    }
}

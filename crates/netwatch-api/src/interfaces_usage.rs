//! `GET /usage/interfaces` — U3 per-interface dashboard totals (API.md §2.5).
//!
//! Excluded interfaces (`lo` by seed) never appear: they are filtered at the
//! query boundary, not hidden in the UI. Bytes come from the per-interface
//! counter sampler (`P4.2`), with `tx_bytes` as up and `rx_bytes` as down.

use std::path::Path;

use serde::Serialize;
use sqlx::{Row, SqlitePool};

use crate::calendar::resolve_timezone;
use crate::error::ApiError;

/// Local sysfs root for Wi-Fi link information (owner L-OQ4). The UI never
/// reads sysfs; U3 derives these values at request time and serializes them.
const SYSFS_NET_ROOT: &str = "/sys/class/net";

/// Up/down split for one interface (API.md U3).
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct InterfaceBytes {
    /// Transmitted bytes in the period.
    pub up: i64,
    /// Received bytes in the period.
    pub down: i64,
}

/// One U3 response row.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct InterfaceUsage {
    /// `interfaces.id`.
    pub id: i64,
    /// sysfs interface name.
    pub name: String,
    /// Stored link kind (`ethernet`, `wifi`, `tun`, …).
    pub kind: String,
    /// Metered label is informational in the MVP.
    pub is_metered: bool,
    /// Counter totals for the requested period.
    pub bytes: InterfaceBytes,
    /// Wi-Fi signal strength in dBm from local sysfs; `null` when unavailable.
    pub signal_dbm: Option<i32>,
    /// Negotiated link speed in Mbps from local sysfs; `null` when unavailable.
    pub link_mbps: Option<u64>,
}

/// Build the U3 payload for one request.
///
/// # Errors
///
/// `QUERY_INVALID` for malformed/contradictory parameters, `DB_ERROR` for
/// SQLite failures.
pub async fn interfaces_usage(
    pool: &SqlitePool,
    now: i64,
    raw_query: Option<&str>,
) -> Result<Vec<InterfaceUsage>, ApiError> {
    build(pool, now, raw_query, Path::new(SYSFS_NET_ROOT)).await
}

/// Build the payload with an injectable sysfs root so Wi-Fi reads can be
/// tested against a synthetic tree (owner L-OQ4).
async fn build(
    pool: &SqlitePool,
    now: i64,
    raw_query: Option<&str>,
    sysfs_root: &Path,
) -> Result<Vec<InterfaceUsage>, ApiError> {
    let (since, until) = parse_range(pool, now, raw_query).await?;
    let rows = sqlx::query(
        "SELECT i.id AS id, i.name AS name, i.kind AS kind, i.is_metered AS is_metered,
                COALESCE(SUM(u.tx_bytes), 0) AS up,
                COALESCE(SUM(u.rx_bytes), 0) AS down
           FROM interfaces i
           LEFT JOIN usage_interface_15m u
             ON u.interface_id = i.id
            AND u.bucket_start >= ?1 AND u.bucket_start < ?2
          WHERE i.is_excluded = 0
          GROUP BY i.id, i.name, i.kind, i.is_metered
          ORDER BY i.id",
    )
    .bind(since)
    .bind(until)
    .fetch_all(pool)
    .await
    .map_err(crate::history::db_error)?;
    rows.iter()
        .map(|row| {
            let name: String = row.try_get("name").map_err(crate::history::db_error)?;
            let kind: String = row.try_get("kind").map_err(crate::history::db_error)?;
            let (signal_dbm, link_mbps) = wifi_link_fields(sysfs_root, &name, &kind);
            Ok(InterfaceUsage {
                id: row.try_get("id").map_err(crate::history::db_error)?,
                name,
                kind,
                is_metered: row
                    .try_get::<i64, _>("is_metered")
                    .map_err(crate::history::db_error)?
                    != 0,
                bytes: InterfaceBytes {
                    up: row.try_get("up").map_err(crate::history::db_error)?,
                    down: row.try_get("down").map_err(crate::history::db_error)?,
                },
                signal_dbm,
                link_mbps,
            })
        })
        .collect()
}

/// Read signal/link for wireless interfaces only. Any missing or unparsable
/// value becomes `None` (serialized `null`, rendered "—" by the UI). Values
/// are read verbatim from sysfs; no network call is made (owner L-OQ4).
fn wifi_link_fields(root: &Path, name: &str, kind: &str) -> (Option<i32>, Option<u64>) {
    if kind != "wifi" {
        return (None, None);
    }
    let iface = root.join(name);
    (
        read_i32(&iface.join("wireless").join("link")),
        read_nonnegative_u64(&iface.join("speed")),
    )
}

fn read_i32(path: &Path) -> Option<i32> {
    std::fs::read_to_string(path).ok()?.trim().parse().ok()
}

/// sysfs `speed` is `-1` when the link speed is unknown; treat it as missing.
fn read_nonnegative_u64(path: &Path) -> Option<u64> {
    let value: i64 = std::fs::read_to_string(path).ok()?.trim().parse().ok()?;
    u64::try_from(value).ok()
}

pub(crate) async fn parse_range(
    pool: &SqlitePool,
    now: i64,
    raw_query: Option<&str>,
) -> Result<(i64, i64), ApiError> {
    let mut since = None;
    let mut until = None;
    for (key, value) in crate::history::parse_query_params(raw_query.unwrap_or(""))? {
        match key.as_str() {
            "since" => since = Some(parse_int("since", &value)?),
            "until" => until = Some(parse_int("until", &value)?),
            "range" => {
                // Validated by crate::range::resolve below.
            }
            other => {
                return Err(ApiError::query_invalid(format!(
                    "invalid query parameter '{other}': unknown"
                )));
            }
        }
    }
    if let (Some(s), Some(u)) = (since, until) {
        if s >= u {
            return Err(ApiError::query_invalid("since must be earlier than until"));
        }
        return Ok((s, u));
    }
    let timezone = crate::usage::setting(pool, "timezone", "local").await?;
    let billing_day = crate::usage::setting(pool, "billing_cycle_day", "1")
        .await?
        .parse::<i64>()
        .unwrap_or(1);
    let tz = resolve_timezone(&timezone);
    let preset = crate::range::resolve(pool, raw_query).await?;
    let period = preset.period(now, tz, billing_day);
    // Keep the historical U3 end (next local midnight): a rollup bucket
    // stamped inside the current local day but ahead of `now` still belongs
    // to that day (netwatchd's publisher relies on this).
    Ok((since.unwrap_or(period.start), until.unwrap_or(period.end)))
}

fn parse_int(key: &str, value: &str) -> Result<i64, ApiError> {
    value
        .parse::<i64>()
        .map_err(|_| ApiError::query_invalid(format!("invalid query parameter '{key}': {value}")))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)] // tests

    use super::*;

    fn temp_root() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "nwp-wifi-sysfs-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn write(root: &std::path::Path, iface: &str, relative: &str, value: &str) {
        let path = root.join(iface).join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, value).unwrap();
    }

    #[test]
    fn wifi_fields_read_local_sysfs_only() {
        let root = temp_root();
        write(&root, "wlan0", "wireless/link", "-57\n");
        write(&root, "wlan0", "speed", "866\n");
        assert_eq!(
            wifi_link_fields(&root, "wlan0", "wifi"),
            (Some(-57), Some(866))
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn missing_or_invalid_sysfs_values_are_none() {
        let root = temp_root();
        std::fs::create_dir_all(root.join("wlan0")).unwrap();
        assert_eq!(wifi_link_fields(&root, "wlan0", "wifi"), (None, None));
        write(&root, "wlan0", "speed", "-1\n");
        write(&root, "wlan0", "wireless/link", "not-a-number\n");
        assert_eq!(wifi_link_fields(&root, "wlan0", "wifi"), (None, None));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn non_wifi_interfaces_are_not_enriched() {
        let root = temp_root();
        write(&root, "eth0", "wireless/link", "-40");
        write(&root, "eth0", "speed", "1000");
        assert_eq!(wifi_link_fields(&root, "eth0", "ethernet"), (None, None));
        std::fs::remove_dir_all(&root).unwrap();
    }
}

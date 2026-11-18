//! `GET /usage/interfaces` — U3 per-interface dashboard totals (API.md §2.5).
//!
//! Excluded interfaces (`lo` by seed) never appear: they are filtered at the
//! query boundary, not hidden in the UI. Bytes come from the per-interface
//! counter sampler (`P4.2`), with `tx_bytes` as up and `rx_bytes` as down.

use serde::Serialize;
use sqlx::{Row, SqlitePool};

use crate::calendar::{day_period, resolve_timezone};
use crate::error::ApiError;

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
            Ok(InterfaceUsage {
                id: row.try_get("id").map_err(crate::history::db_error)?,
                name: row.try_get("name").map_err(crate::history::db_error)?,
                kind: row.try_get("kind").map_err(crate::history::db_error)?,
                is_metered: row
                    .try_get::<i64, _>("is_metered")
                    .map_err(crate::history::db_error)?
                    != 0,
                bytes: InterfaceBytes {
                    up: row.try_get("up").map_err(crate::history::db_error)?,
                    down: row.try_get("down").map_err(crate::history::db_error)?,
                },
            })
        })
        .collect()
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
    let today = day_period(now, resolve_timezone(&timezone));
    Ok((since.unwrap_or(today.start), until.unwrap_or(today.end)))
}

fn parse_int(key: &str, value: &str) -> Result<i64, ApiError> {
    value
        .parse::<i64>()
        .map_err(|_| ApiError::query_invalid(format!("invalid query parameter '{key}': {value}")))
}

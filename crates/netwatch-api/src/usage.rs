//! `GET /usage/summary` — U1 dashboard headline totals (API.md §2.5).
//!
//! Interface counters are the source of truth for `total`; per-app rollups
//! provide `attributed` and the top-talker list. Their difference is the
//! honest `unattributed` gap (headers, ACKs, kernel/VPN traffic — D3a).

use serde::Serialize;
use sqlx::{Row, SqlitePool};

use crate::calendar::{billing_period, day_period, resolve_timezone};
use crate::error::ApiError;

const TOP_LIMIT_DEFAULT: i64 = 5;
const TOP_LIMIT_MAX: i64 = 10;

/// Headline byte totals for one period (API.md U1).
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Totals {
    /// ISP-billed bytes from interface counters.
    pub total: i64,
    /// Sum of per-app rollups.
    pub attributed: i64,
    /// `total - attributed`, clamped at zero.
    pub unattributed: i64,
}

/// One top-talker row (API.md U1).
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct TopApp {
    /// App registry id.
    pub id: i64,
    /// Display name from the app registry.
    pub name: String,
    /// Attributed bytes in the billing period.
    pub bytes_total: i64,
}

/// Full U1 response payload.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct UsageSummary {
    /// Current local calendar day.
    pub today: Totals,
    /// Current billing cycle.
    pub month: Totals,
    /// Ranked talkers within the billing cycle.
    pub top_apps: Vec<TopApp>,
    /// `"collecting"` until the first counter/rollup sample exists (D3a).
    pub state: &'static str,
}

/// Build the U1 payload for one request.
///
/// # Errors
///
/// `QUERY_INVALID` for a bad `top_limit`, `DB_ERROR` for SQLite failures.
pub async fn summary(
    pool: &SqlitePool,
    now: i64,
    raw_query: Option<&str>,
) -> Result<UsageSummary, ApiError> {
    let top_limit = parse_top_limit(raw_query)?;
    let timezone = setting(pool, "timezone", "local").await?;
    let billing_day = setting(pool, "billing_cycle_day", "1")
        .await?
        .parse::<i64>()
        .unwrap_or(1);
    let tz = resolve_timezone(&timezone);
    let today = day_period(now, tz);
    let month = billing_period(now, tz, billing_day);

    let (today_total, today_attributed) = period_totals(pool, today).await?;
    let (month_total, month_attributed) = period_totals(pool, month).await?;
    let top_apps = top_apps(pool, month, top_limit).await?;
    let has_data = today_total > 0
        || today_attributed > 0
        || month_total > 0
        || month_attributed > 0
        || !top_apps.is_empty();

    Ok(UsageSummary {
        today: totals(today_total, today_attributed),
        month: totals(month_total, month_attributed),
        top_apps,
        state: if has_data { "ok" } else { "collecting" },
    })
}

fn totals(total: i64, attributed: i64) -> Totals {
    Totals {
        total,
        attributed,
        // A non-zero gap is expected; a negative one means an upstream
        // accounting bug, not "negative traffic" (D3a).
        unattributed: total.saturating_sub(attributed),
    }
}

async fn period_totals(
    pool: &SqlitePool,
    period: crate::calendar::Period,
) -> Result<(i64, i64), ApiError> {
    let total: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(rx_bytes + tx_bytes), 0)
           FROM usage_interface_15m
          WHERE bucket_start >= ?1 AND bucket_start < ?2",
    )
    .bind(period.start)
    .bind(period.end)
    .fetch_one(pool)
    .await
    .map_err(crate::history::db_error)?;
    let attributed: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(bytes_up + bytes_down), 0)
           FROM usage_rollup_15m
          WHERE bucket_start >= ?1 AND bucket_start < ?2",
    )
    .bind(period.start)
    .bind(period.end)
    .fetch_one(pool)
    .await
    .map_err(crate::history::db_error)?;
    Ok((total, attributed))
}

async fn top_apps(
    pool: &SqlitePool,
    period: crate::calendar::Period,
    limit: i64,
) -> Result<Vec<TopApp>, ApiError> {
    let rows = sqlx::query(
        "SELECT a.id AS id, a.name AS name,
                SUM(r.bytes_up + r.bytes_down) AS bytes_total
           FROM usage_rollup_15m r JOIN apps a ON a.id = r.app_id
          WHERE r.bucket_start >= ?1 AND r.bucket_start < ?2
          GROUP BY a.id, a.name
          ORDER BY bytes_total DESC, a.id ASC
          LIMIT ?3",
    )
    .bind(period.start)
    .bind(period.end)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(crate::history::db_error)?;
    rows.iter()
        .map(|row| {
            Ok(TopApp {
                id: row.try_get("id").map_err(crate::history::db_error)?,
                name: row.try_get("name").map_err(crate::history::db_error)?,
                bytes_total: row
                    .try_get("bytes_total")
                    .map_err(crate::history::db_error)?,
            })
        })
        .collect()
}

pub(crate) async fn setting(
    pool: &SqlitePool,
    key: &str,
    default: &str,
) -> Result<String, ApiError> {
    let value: Option<String> = sqlx::query_scalar("SELECT value FROM settings WHERE key = ?1")
        .bind(key)
        .fetch_optional(pool)
        .await
        .map_err(crate::history::db_error)?;
    Ok(value.unwrap_or_else(|| default.to_string()))
}

fn parse_top_limit(raw: Option<&str>) -> Result<i64, ApiError> {
    let mut limit = TOP_LIMIT_DEFAULT;
    for (key, value) in crate::history::parse_query_params(raw.unwrap_or(""))? {
        match key.as_str() {
            "top_limit" => {
                limit = value
                    .parse::<i64>()
                    .map_err(|_| ApiError::query_invalid(format!("invalid top_limit: {value}")))?;
            }
            other => {
                return Err(ApiError::query_invalid(format!(
                    "invalid query parameter '{other}': unknown"
                )));
            }
        }
    }
    if !(1..=TOP_LIMIT_MAX).contains(&limit) {
        return Err(ApiError::query_invalid(format!(
            "top_limit must be 1..={TOP_LIMIT_MAX}"
        )));
    }
    Ok(limit)
}

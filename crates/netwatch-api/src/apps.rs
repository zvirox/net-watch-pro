//! `GET /apps` — A1 ranked app list (API.md §2.4).
//!
//! Usage values reuse U1's local-day and billing-period rollup ranges so the
//! ranking agrees with `/usage/summary`. Soft-deleted apps stay in history but
//! are hidden here unless explicitly requested.

use std::collections::HashMap;

use netwatch_common::DirectionTotals;
use serde::Serialize;
use sqlx::{Row, SqlitePool};

use crate::calendar::{billing_period, day_period, resolve_timezone};
use crate::error::ApiError;

const LIMIT_DEFAULT: i64 = 50;
const LIMIT_MAX: i64 = 200;

/// Up/down split for one period (API.md A1).
#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
pub struct AppUsage {
    /// Attributed uplink bytes.
    pub up: u64,
    /// Attributed downlink bytes.
    pub down: u64,
}

/// One A1 response row.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct AppRow {
    /// `apps.id`.
    pub id: i64,
    /// Display name.
    pub name: String,
    /// Stable icon handle; `null` → UI fallback (DESIGN.md §6.2).
    pub icon: Option<String>,
    /// Attribution confidence (`exact`/`heuristic`/`unknown`).
    pub attribution_confidence: String,
    /// Epoch seconds of first sighting.
    pub first_seen_at: i64,
    /// Epoch seconds of last activity.
    pub last_seen_at: i64,
    /// Current-day and billing-cycle usage.
    pub usage: AppUsagePair,
}

/// The two usage periods in an A1 row.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct AppUsagePair {
    /// Current local day.
    pub today: AppUsage,
    /// Current billing cycle.
    pub month: AppUsage,
}

#[derive(Clone, Copy)]
enum Sort {
    UsageMonth,
    UsageToday,
    LastSeenAt,
}

#[derive(Clone, Copy)]
enum Order {
    Asc,
    Desc,
}

/// Build the A1 payload for one request.
///
/// # Errors
///
/// `QUERY_INVALID` for malformed parameters, `DB_ERROR` for SQLite failures.
pub async fn apps_page(
    pool: &SqlitePool,
    now: i64,
    raw_query: Option<&str>,
) -> Result<Vec<AppRow>, ApiError> {
    let query = parse_query(raw_query)?;
    let timezone = crate::usage::setting(pool, "timezone", "local").await?;
    let billing_day = crate::usage::setting(pool, "billing_cycle_day", "1")
        .await?
        .parse::<i64>()
        .unwrap_or(1);
    let tz = resolve_timezone(&timezone);
    let today = day_period(now, tz);
    let month = billing_period(now, tz, billing_day);

    let today_usage = usage_by_app(pool, today).await?;
    let month_usage = usage_by_app(pool, month).await?;
    let mut rows = load_apps(pool, query.include_deleted).await?;
    for row in &mut rows {
        let empty = DirectionTotals::default();
        let today = today_usage.get(&row.id).unwrap_or(&empty);
        let month = month_usage.get(&row.id).unwrap_or(&empty);
        row.usage = AppUsagePair {
            today: AppUsage {
                up: today.up,
                down: today.down,
            },
            month: AppUsage {
                up: month.up,
                down: month.down,
            },
        };
    }
    sort_rows(&mut rows, query.sort, query.order);
    rows.truncate(usize::try_from(query.limit).unwrap_or(usize::MAX));
    Ok(rows)
}

struct Query {
    sort: Sort,
    order: Order,
    limit: i64,
    include_deleted: bool,
}

fn parse_query(raw: Option<&str>) -> Result<Query, ApiError> {
    let mut query = Query {
        sort: Sort::UsageMonth,
        order: Order::Desc,
        limit: LIMIT_DEFAULT,
        include_deleted: false,
    };
    for (key, value) in crate::history::parse_query_params(raw.unwrap_or(""))? {
        match key.as_str() {
            "sort" => {
                query.sort = match value.as_str() {
                    "usage_month" => Sort::UsageMonth,
                    "usage_today" => Sort::UsageToday,
                    "last_seen_at" => Sort::LastSeenAt,
                    other => {
                        return Err(ApiError::query_invalid(format!("invalid sort: {other}")));
                    }
                };
            }
            "order" => {
                query.order = match value.as_str() {
                    "asc" => Order::Asc,
                    "desc" => Order::Desc,
                    other => {
                        return Err(ApiError::query_invalid(format!("invalid order: {other}")));
                    }
                };
            }
            "limit" => {
                query.limit = value.parse::<i64>().map_err(|_| {
                    ApiError::query_invalid(format!("invalid query parameter 'limit': {value}"))
                })?;
            }
            "include_deleted" => {
                query.include_deleted = match value.as_str() {
                    "true" | "1" => true,
                    "false" | "0" => false,
                    other => {
                        return Err(ApiError::query_invalid(format!(
                            "invalid include_deleted: {other}"
                        )));
                    }
                };
            }
            other => {
                return Err(ApiError::query_invalid(format!(
                    "invalid query parameter '{other}': unknown"
                )));
            }
        }
    }
    if !(1..=LIMIT_MAX).contains(&query.limit) {
        return Err(ApiError::query_invalid(format!(
            "limit must be 1..={LIMIT_MAX}"
        )));
    }
    Ok(query)
}

async fn load_apps(pool: &SqlitePool, include_deleted: bool) -> Result<Vec<AppRow>, ApiError> {
    let sql = if include_deleted {
        "SELECT id, name, icon_path, attribution_confidence, first_seen_at, last_seen_at
           FROM apps"
    } else {
        "SELECT id, name, icon_path, attribution_confidence, first_seen_at, last_seen_at
           FROM apps WHERE deleted_at IS NULL"
    };
    let rows = sqlx::query(sql)
        .fetch_all(pool)
        .await
        .map_err(crate::history::db_error)?;
    rows.iter()
        .map(|row| {
            let id: i64 = row.try_get("id").map_err(crate::history::db_error)?;
            let icon_path: Option<String> =
                row.try_get("icon_path").map_err(crate::history::db_error)?;
            Ok(AppRow {
                id,
                name: row.try_get("name").map_err(crate::history::db_error)?,
                // Never expose a filesystem path; the handle resolves to an
                // icon resource served by the same API (A2, DESIGN.md §6.2).
                icon: icon_path.map(|_| format!("ic:{id}")),
                attribution_confidence: row
                    .try_get("attribution_confidence")
                    .map_err(crate::history::db_error)?,
                first_seen_at: row
                    .try_get("first_seen_at")
                    .map_err(crate::history::db_error)?,
                last_seen_at: row
                    .try_get("last_seen_at")
                    .map_err(crate::history::db_error)?,
                usage: AppUsagePair {
                    today: AppUsage { up: 0, down: 0 },
                    month: AppUsage { up: 0, down: 0 },
                },
            })
        })
        .collect()
}

async fn usage_by_app(
    pool: &SqlitePool,
    period: crate::calendar::Period,
) -> Result<HashMap<i64, DirectionTotals>, ApiError> {
    let rows = sqlx::query(
        "SELECT app_id, SUM(bytes_up) AS up, SUM(bytes_down) AS down
           FROM usage_rollup_15m
          WHERE bucket_start >= ?1 AND bucket_start < ?2
          GROUP BY app_id",
    )
    .bind(period.start)
    .bind(period.end)
    .fetch_all(pool)
    .await
    .map_err(crate::history::db_error)?;
    let mut totals = HashMap::new();
    for row in rows {
        let app_id: i64 = row.try_get("app_id").map_err(crate::history::db_error)?;
        let up: i64 = row.try_get("up").map_err(crate::history::db_error)?;
        let down: i64 = row.try_get("down").map_err(crate::history::db_error)?;
        totals.insert(
            app_id,
            DirectionTotals {
                up: u64::try_from(up.max(0)).unwrap_or(u64::MAX),
                down: u64::try_from(down.max(0)).unwrap_or(u64::MAX),
            },
        );
    }
    Ok(totals)
}

fn sort_rows(rows: &mut [AppRow], sort: Sort, order: Order) {
    let key = |row: &AppRow| -> u128 {
        match sort {
            Sort::UsageMonth => u128::from(row.usage.month.up) + u128::from(row.usage.month.down),
            Sort::UsageToday => u128::from(row.usage.today.up) + u128::from(row.usage.today.down),
            Sort::LastSeenAt => {
                u128::from(u64::try_from(row.last_seen_at.max(0)).unwrap_or(u64::MAX))
            }
        }
    };
    rows.sort_by(|a, b| {
        let comparison = key(a).cmp(&key(b)).then_with(|| a.id.cmp(&b.id));
        match order {
            Order::Asc => comparison,
            Order::Desc => comparison.reverse(),
        }
    });
}

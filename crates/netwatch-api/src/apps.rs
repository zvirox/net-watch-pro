//! `GET /apps` — A1 ranked app list (API.md §2.4).
//!
//! Usage values reuse U1's local-day and billing-period rollup ranges so the
//! ranking agrees with `/usage/summary`. Soft-deleted apps stay in history but
//! are hidden here unless explicitly requested.

use std::collections::HashMap;
use std::path::Path;

use netwatch_common::DirectionTotals;
use serde::Serialize;
use sqlx::sqlite::SqliteRow;
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

/// One A4 endpoint-ranking row.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct EndpointRow {
    /// Remote IP address.
    pub remote_ip: String,
    /// Resolved hostname when known.
    pub remote_host: Option<String>,
    /// Remote port.
    pub port: u16,
    /// Total bytes to/from this endpoint.
    pub bytes_total: u64,
    /// Number of contributing episodes.
    pub connections: u64,
    /// Last activity epoch seconds.
    pub last_seen_at: i64,
}

/// One A2 detail row: the A1 shape plus provenance and lifecycle fields.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct AppDetail {
    /// Shared A1 fields.
    #[serde(flatten)]
    pub app: AppRow,
    /// Executable path (provenance only; never exposed as an icon path).
    pub exe_path: String,
    /// Soft-delete timestamp; `null` while the app is active.
    pub deleted_at: Option<i64>,
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
    let (today, month) = periods(pool, now).await?;

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

async fn periods(
    pool: &SqlitePool,
    now: i64,
) -> Result<(crate::calendar::Period, crate::calendar::Period), ApiError> {
    let timezone = crate::usage::setting(pool, "timezone", "local").await?;
    let billing_day = crate::usage::setting(pool, "billing_cycle_day", "1")
        .await?
        .parse::<i64>()
        .unwrap_or(1);
    let tz = resolve_timezone(&timezone);
    Ok((day_period(now, tz), billing_period(now, tz, billing_day)))
}

/// Build the A2 payload for one app, including soft-deleted rows so history
/// detail pages keep working (DESIGN.md §8.5).
///
/// # Errors
///
/// `NOT_FOUND` when the app does not exist; `DB_ERROR` for SQLite failures.
pub async fn app_detail(pool: &SqlitePool, now: i64, id: i64) -> Result<AppDetail, ApiError> {
    let row = sqlx::query(
        "SELECT id, name, icon_path, attribution_confidence, first_seen_at,
                last_seen_at, exe_path, deleted_at
           FROM apps WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(crate::history::db_error)?
    .ok_or_else(ApiError::not_found)?;

    let mut app = map_app_row(&row)?;
    let (today, month) = periods(pool, now).await?;
    let today_usage = usage_by_app(pool, today).await?;
    let month_usage = usage_by_app(pool, month).await?;
    let empty = DirectionTotals::default();
    let today = today_usage.get(&app.id).unwrap_or(&empty);
    let month = month_usage.get(&app.id).unwrap_or(&empty);
    app.usage = AppUsagePair {
        today: AppUsage {
            up: today.up,
            down: today.down,
        },
        month: AppUsage {
            up: month.up,
            down: month.down,
        },
    };
    Ok(AppDetail {
        app,
        exe_path: row.try_get("exe_path").map_err(crate::history::db_error)?,
        deleted_at: row
            .try_get("deleted_at")
            .map_err(crate::history::db_error)?,
    })
}

/// Resolve the registered icon for an app ID. The request never carries a
/// filesystem path: only a DB-registered path with an image extension is read.
///
/// # Errors
///
/// `NOT_FOUND` for unknown app, no icon, unsupported type, or unreadable file.
pub async fn app_icon(pool: &SqlitePool, id: i64) -> Result<(&'static str, Vec<u8>), ApiError> {
    let icon_path: Option<String> = sqlx::query_scalar("SELECT icon_path FROM apps WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(crate::history::db_error)?
        .flatten();
    let icon_path = icon_path.ok_or_else(ApiError::not_found)?;
    let content_type = content_type(&icon_path).ok_or_else(ApiError::not_found)?;
    let bytes = tokio::fs::read(&icon_path)
        .await
        .map_err(|_| ApiError::not_found())?;
    Ok((content_type, bytes))
}

fn content_type(path: &str) -> Option<&'static str> {
    let extension = Path::new(path).extension()?.to_str()?.to_ascii_lowercase();
    match extension.as_str() {
        "png" => Some("image/png"),
        "ico" => Some("image/x-icon"),
        "svg" => Some("image/svg+xml"),
        "webp" => Some("image/webp"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        _ => None,
    }
}

/// Whether an app exists (used by A3/A4 404 handling).
///
/// # Errors
///
/// Propagates SQLite failures (`DB_ERROR`).
pub async fn app_exists(pool: &SqlitePool, id: i64) -> Result<bool, ApiError> {
    let found: Option<i64> = sqlx::query_scalar("SELECT 1 FROM apps WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(crate::history::db_error)?;
    Ok(found.is_some())
}

/// Build the A4 ranked endpoint list for one app.
///
/// Raw-window episodes are grouped live; older buckets come from
/// `usage_endpoint_day` at the same seam as C2. Both are merged by endpoint.
///
/// # Errors
///
/// `NOT_FOUND` for unknown apps; `QUERY_INVALID` for bad parameters;
/// `DB_ERROR` for SQLite failures.
pub async fn app_endpoints(
    pool: &SqlitePool,
    now: i64,
    id: i64,
    raw_query: Option<&str>,
) -> Result<Vec<EndpointRow>, ApiError> {
    if !app_exists(pool, id).await? {
        return Err(ApiError::not_found());
    }
    let limit = parse_endpoint_query(raw_query)?;
    let cutoff = crate::timeline::raw_cutoff(pool, now).await?;
    let mut endpoints: HashMap<(String, u16), EndpointRow> = HashMap::new();

    let raw = sqlx::query(
        "SELECT remote_ip, MAX(remote_host) AS remote_host, remote_port,
                SUM(bytes_up + bytes_down) AS bytes_total,
                COUNT(*) AS connections, MAX(last_seen_at) AS last_seen_at
           FROM connections
          WHERE app_id = ?1 AND last_seen_at >= ?2
          GROUP BY remote_ip, remote_port",
    )
    .bind(id)
    .bind(cutoff)
    .fetch_all(pool)
    .await
    .map_err(crate::history::db_error)?;
    for row in raw {
        add_endpoint(&mut endpoints, endpoint_from_row(&row, true)?);
    }

    let aged = sqlx::query(
        "SELECT remote_ip, remote_port,
                SUM(bytes_up + bytes_down) AS bytes_total,
                SUM(connection_count) AS connections, MAX(last_seen_at) AS last_seen_at
           FROM usage_endpoint_day
          WHERE app_id = ?1 AND bucket_start + ?2 <= ?3
          GROUP BY remote_ip, remote_port",
    )
    .bind(id)
    .bind(crate::timeline::DAY_S)
    .bind(cutoff)
    .fetch_all(pool)
    .await
    .map_err(crate::history::db_error)?;
    for row in aged {
        add_endpoint(&mut endpoints, endpoint_from_row(&row, false)?);
    }

    let mut rows: Vec<EndpointRow> = endpoints.into_values().collect();
    rows.sort_by(|a, b| {
        b.bytes_total
            .cmp(&a.bytes_total)
            .then_with(|| a.remote_ip.cmp(&b.remote_ip))
            .then_with(|| a.port.cmp(&b.port))
    });
    rows.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
    Ok(rows)
}

fn endpoint_from_row(row: &SqliteRow, raw: bool) -> Result<EndpointRow, ApiError> {
    let port: i64 = row
        .try_get("remote_port")
        .map_err(crate::history::db_error)?;
    let bytes_total: i64 = row
        .try_get("bytes_total")
        .map_err(crate::history::db_error)?;
    let connections: i64 = row
        .try_get("connections")
        .map_err(crate::history::db_error)?;
    Ok(EndpointRow {
        remote_ip: row.try_get("remote_ip").map_err(crate::history::db_error)?,
        remote_host: if raw {
            row.try_get("remote_host")
                .map_err(crate::history::db_error)?
        } else {
            None
        },
        port: u16::try_from(port).unwrap_or_default(),
        bytes_total: u64::try_from(bytes_total.max(0)).unwrap_or(u64::MAX),
        connections: u64::try_from(connections.max(0)).unwrap_or(u64::MAX),
        last_seen_at: row
            .try_get("last_seen_at")
            .map_err(crate::history::db_error)?,
    })
}

fn add_endpoint(endpoints: &mut HashMap<(String, u16), EndpointRow>, row: EndpointRow) {
    endpoints
        .entry((row.remote_ip.clone(), row.port))
        .and_modify(|existing| {
            existing.bytes_total = existing.bytes_total.saturating_add(row.bytes_total);
            existing.connections = existing.connections.saturating_add(row.connections);
            existing.last_seen_at = existing.last_seen_at.max(row.last_seen_at);
            if existing.remote_host.is_none() {
                existing.remote_host.clone_from(&row.remote_host);
            }
        })
        .or_insert(row);
}

fn parse_endpoint_query(raw: Option<&str>) -> Result<i64, ApiError> {
    let mut limit = 100_i64;
    for (key, value) in crate::history::parse_query_params(raw.unwrap_or(""))? {
        match key.as_str() {
            "limit" => {
                limit = value.parse::<i64>().map_err(|_| {
                    ApiError::query_invalid(format!("invalid query parameter 'limit': {value}"))
                })?;
            }
            "order" if value == "bytes" => {}
            "order" => {
                return Err(ApiError::query_invalid(format!("invalid order: {value}")));
            }
            other => {
                return Err(ApiError::query_invalid(format!(
                    "invalid query parameter '{other}': unknown"
                )));
            }
        }
    }
    if !(1..=100).contains(&limit) {
        return Err(ApiError::query_invalid("limit must be 1..=100"));
    }
    Ok(limit)
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
    rows.iter().map(map_app_row).collect()
}

fn map_app_row(row: &SqliteRow) -> Result<AppRow, ApiError> {
    let id: i64 = row.try_get("id").map_err(crate::history::db_error)?;
    let icon_path: Option<String> = row.try_get("icon_path").map_err(crate::history::db_error)?;
    Ok(AppRow {
        id,
        name: row.try_get("name").map_err(crate::history::db_error)?,
        // Never expose a filesystem path; the handle resolves to an icon
        // resource served by the same API (A2, DESIGN.md §6.2).
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

//! Budget state for U1 (API.md §2.5 additive `budgets` field, PRD §6.1 #8).
//!
//! Totals are metered-aware (D6a): when at least one interface is marked
//! metered, only those counters count against a budget. With no metered
//! interface configured, the full interface total is used (honest fallback).
//! The threshold math itself lives in `netwatch_core::budgets`.

use serde::Serialize;
use sqlx::{Row, SqlitePool};

use netwatch_core::budgets::{BudgetLevel, evaluate};

use crate::calendar::Period;
use crate::error::ApiError;

const MB: i64 = 1024 * 1024;
/// Bound the per-app arrays so U1 stays a small envelope.
const MAX_PER_APP: i64 = 10;

/// One evaluated budget (DESIGN.md §7.14 usage bar).
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct BudgetState {
    /// Metered-only bytes consumed in the period.
    pub used_bytes: i64,
    /// Configured ceiling in bytes (0 when disabled).
    pub limit_bytes: i64,
    /// `ok | warning | breach`.
    pub level: &'static str,
}

/// Per-app variant of [`BudgetState`].
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct AppBudgetState {
    /// App registry id.
    pub app_id: i64,
    /// Display name.
    pub name: String,
    /// Bytes consumed in the period.
    pub used_bytes: i64,
    /// Configured ceiling in bytes (0 when disabled).
    pub limit_bytes: i64,
    /// `ok | warning | breach`.
    pub level: &'static str,
}

/// All budget states attached to U1.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Budgets {
    /// Current-day total against its budget.
    pub total_daily: BudgetState,
    /// Current billing-cycle total against its budget.
    pub total_monthly: BudgetState,
    /// Per-app daily budgets (top talkers).
    pub per_app_daily: Vec<AppBudgetState>,
    /// Per-app monthly budgets (top talkers).
    pub per_app_monthly: Vec<AppBudgetState>,
}

/// Compute all budget states for the two current periods.
///
/// # Errors
///
/// Propagates SQLite failures (`DB_ERROR`).
pub async fn compute(
    pool: &SqlitePool,
    today: Period,
    month: Period,
    today_total: i64,
    month_total: i64,
) -> Result<Budgets, ApiError> {
    let warn = read_mb(pool, "budget_warn_pct", 80).await?;
    let warn_pct = u8::try_from(warn.clamp(1, 100)).unwrap_or(80);
    let metered = has_metered_interface(pool).await?;
    let today_used = if metered {
        metered_total(pool, today).await?
    } else {
        today_total
    };
    let month_used = if metered {
        metered_total(pool, month).await?
    } else {
        month_total
    };

    let daily_limit = read_mb(pool, "budget_total_daily_mb", 0).await? * MB;
    let monthly_limit = read_mb(pool, "budget_total_monthly_mb", 0).await? * MB;
    let app_daily_limit = read_mb(pool, "budget_per_app_daily_mb", 0).await? * MB;
    let app_monthly_limit = read_mb(pool, "budget_per_app_monthly_mb", 0).await? * MB;

    Ok(Budgets {
        total_daily: state(today_used, daily_limit, warn_pct),
        total_monthly: state(month_used, monthly_limit, warn_pct),
        per_app_daily: app_states(pool, today, app_daily_limit, warn_pct).await?,
        per_app_monthly: app_states(pool, month, app_monthly_limit, warn_pct).await?,
    })
}

fn state(used: i64, limit: i64, warn_pct: u8) -> BudgetState {
    let level = evaluate(
        u64::try_from(used.max(0)).unwrap_or(0),
        u64::try_from(limit.max(0)).unwrap_or(0),
        warn_pct,
    );
    BudgetState {
        used_bytes: used,
        limit_bytes: limit,
        level: level_str(level),
    }
}

fn level_str(level: BudgetLevel) -> &'static str {
    match level {
        BudgetLevel::Ok => "ok",
        BudgetLevel::Warning => "warning",
        BudgetLevel::Breach => "breach",
    }
}

async fn read_mb(pool: &SqlitePool, key: &str, default: i64) -> Result<i64, ApiError> {
    let raw: Option<String> = sqlx::query_scalar("SELECT value FROM settings WHERE key = ?1")
        .bind(key)
        .fetch_optional(pool)
        .await
        .map_err(crate::history::db_error)?;
    Ok(raw.and_then(|value| value.parse().ok()).unwrap_or(default))
}

async fn has_metered_interface(pool: &SqlitePool) -> Result<bool, ApiError> {
    let found: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM interfaces WHERE is_metered = 1)")
            .fetch_one(pool)
            .await
            .map_err(crate::history::db_error)?;
    Ok(found)
}

async fn metered_total(pool: &SqlitePool, period: Period) -> Result<i64, ApiError> {
    let total: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(rx_bytes + tx_bytes), 0)
           FROM usage_interface_15m c
           JOIN interfaces i ON i.id = c.interface_id
          WHERE c.bucket_start >= ?1 AND c.bucket_start < ?2
            AND i.is_metered = 1 AND i.is_excluded = 0",
    )
    .bind(period.start)
    .bind(period.end)
    .fetch_one(pool)
    .await
    .map_err(crate::history::db_error)?;
    Ok(total)
}

async fn app_states(
    pool: &SqlitePool,
    period: Period,
    limit: i64,
    warn_pct: u8,
) -> Result<Vec<AppBudgetState>, ApiError> {
    let rows = sqlx::query(
        "SELECT a.id, a.name, COALESCE(SUM(r.bytes_up + r.bytes_down), 0) AS used
           FROM usage_rollup_15m r
           JOIN apps a ON a.id = r.app_id
          WHERE r.bucket_start >= ?1 AND r.bucket_start < ?2 AND a.deleted_at IS NULL
          GROUP BY a.id, a.name
          ORDER BY used DESC
          LIMIT ?3",
    )
    .bind(period.start)
    .bind(period.end)
    .bind(MAX_PER_APP)
    .fetch_all(pool)
    .await
    .map_err(crate::history::db_error)?;
    Ok(rows
        .iter()
        .map(|row| {
            let used: i64 = row.get("used");
            let level = evaluate(
                u64::try_from(used.max(0)).unwrap_or(0),
                u64::try_from(limit.max(0)).unwrap_or(0),
                warn_pct,
            );
            AppBudgetState {
                app_id: row.get("id"),
                name: row.get("name"),
                used_bytes: used,
                limit_bytes: limit,
                level: level_str(level),
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)] // tests

    use super::*;

    #[test]
    fn budget_state_maps_levels() {
        assert_eq!(state(10, 0, 80).level, "ok");
        assert_eq!(state(80, 100, 80).level, "warning");
        assert_eq!(state(100, 100, 80).level, "breach");
    }
}

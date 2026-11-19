//! Settings endpoints T1/T2 (API.md §2.7) and the server-side retention
//! ceiling. Values are stored as TEXT per DATABASE.md §2.6 and typed here.

use axum::http::StatusCode;
use netwatch_common::ErrorCode;
use serde_json::{Value, json};
use sqlx::SqlitePool;

use crate::error::ApiError;

const RETENTION_FREE_MAX: i64 = 90;
const RETENTION_PRO_MAX: i64 = 365;

/// All settings with types, bounds, and tier-aware ceilings (T1).
///
/// # Errors
///
/// Propagates SQLite failures (`DB_ERROR`).
pub async fn list(pool: &SqlitePool) -> Result<Vec<Value>, ApiError> {
    let pro = crate::license::is_pro(pool).await?;
    let mut rows = Vec::with_capacity(SPECIFICATIONS.len());
    for spec in SPECIFICATIONS {
        let stored = read_setting(pool, spec.key).await?;
        rows.push(spec.to_json(stored.as_deref(), pro));
    }
    Ok(rows)
}

/// Set one closed-list setting with type/range validation (T2).
///
/// Free-tier retention above 90 days returns `403 EXPORT_PRO_REQUIRED`;
/// rejected writes never touch the database.
///
/// # Errors
///
/// `QUERY_INVALID` for unknown keys/types/ranges, `EXPORT_PRO_REQUIRED` for
/// the free retention ceiling, and `DB_ERROR` for SQLite failures.
pub async fn update(pool: &SqlitePool, key: &str, value: &Value) -> Result<Value, ApiError> {
    let spec = SPECIFICATIONS
        .iter()
        .find(|spec| spec.key == key)
        .ok_or_else(|| ApiError::query_invalid(format!("unknown setting key: {key}")))?;
    let stored = spec.validate(value)?;
    if key == "retention_days" {
        let days = value
            .as_i64()
            .ok_or_else(|| ApiError::query_invalid("retention_days must be an integer"))?;
        if days > RETENTION_FREE_MAX && !crate::license::is_pro(pool).await? {
            return Err(ApiError::new(
                ErrorCode::ExportProRequired,
                StatusCode::FORBIDDEN,
                format!("Retention above {RETENTION_FREE_MAX} days requires Pro"),
            ));
        }
    }
    sqlx::query("UPDATE settings SET value = ?2, updated_at = strftime('%s','now') WHERE key = ?1")
        .bind(key)
        .bind(&stored)
        .execute(pool)
        .await
        .map_err(crate::history::db_error)?;
    let pro = crate::license::is_pro(pool).await?;
    Ok(spec.to_json(Some(&stored), pro))
}

async fn read_setting(pool: &SqlitePool, key: &str) -> Result<Option<String>, ApiError> {
    sqlx::query_scalar("SELECT value FROM settings WHERE key = ?1")
        .bind(key)
        .fetch_optional(pool)
        .await
        .map_err(crate::history::db_error)
}

struct Spec {
    key: &'static str,
    kind: &'static str,
    default: &'static str,
    min: Option<i64>,
    max: Option<i64>,
    pro_max: Option<i64>,
    values: Option<&'static [&'static str]>,
    help: &'static str,
}

const SPECIFICATIONS: &[Spec] = &[
    Spec {
        key: "retention_days",
        kind: "int",
        default: "90",
        min: Some(7),
        max: Some(RETENTION_FREE_MAX),
        pro_max: Some(RETENTION_PRO_MAX),
        values: None,
        help: "History kept before automatic pruning",
    },
    Spec {
        key: "start_on_boot",
        kind: "bool",
        default: "1",
        min: None,
        max: None,
        pro_max: None,
        values: None,
        help: "Enable the systemd service at boot (synced by the daemon)",
    },
    Spec {
        key: "theme_default",
        kind: "enum",
        default: "system",
        min: None,
        max: None,
        pro_max: None,
        values: Some(&["system", "light", "dark"]),
        help: "Initial UI theme",
    },
    Spec {
        key: "speed_unit",
        kind: "enum",
        default: "mbps",
        min: None,
        max: None,
        pro_max: None,
        values: Some(&["mbps", "mbytes"]),
        help: "Live speed unit: Mbps (bits/s) or MB/s (bytes/s)",
    },
    Spec {
        key: "metered_policy",
        kind: "enum",
        default: "ask",
        min: None,
        max: None,
        pro_max: None,
        values: Some(&["ask", "label", "auto"]),
        help: "New interface labeling: ask (manual), label (metered), auto (falls back to ask in MVP)",
    },
    Spec {
        key: "raw_window_days",
        kind: "int",
        default: "7",
        min: Some(1),
        max: Some(365),
        pro_max: None,
        values: None,
        help: "Days of connection-level history before aggregation",
    },
    Spec {
        key: "timezone",
        kind: "enum",
        default: "local",
        min: None,
        max: None,
        pro_max: None,
        values: None,
        help: "IANA timezone for local day and billing boundaries",
    },
    Spec {
        key: "billing_cycle_day",
        kind: "int",
        default: "1",
        min: Some(1),
        max: Some(31),
        pro_max: None,
        values: None,
        help: "Day of month that starts the billing cycle",
    },
    // D6a: budgets are a free-tier feature. Values are whole MB; 0 = off.
    Spec {
        key: "budget_total_daily_mb",
        kind: "int",
        default: "0",
        min: Some(0),
        max: Some(1_000_000),
        pro_max: None,
        values: None,
        help: "Daily total budget in MB (0 = off)",
    },
    Spec {
        key: "budget_total_monthly_mb",
        kind: "int",
        default: "0",
        min: Some(0),
        max: Some(1_000_000),
        pro_max: None,
        values: None,
        help: "Monthly total budget in MB (0 = off)",
    },
    Spec {
        key: "budget_per_app_daily_mb",
        kind: "int",
        default: "0",
        min: Some(0),
        max: Some(1_000_000),
        pro_max: None,
        values: None,
        help: "Daily per-app budget in MB (0 = off)",
    },
    Spec {
        key: "budget_per_app_monthly_mb",
        kind: "int",
        default: "0",
        min: Some(0),
        max: Some(1_000_000),
        pro_max: None,
        values: None,
        help: "Monthly per-app budget in MB (0 = off)",
    },
    Spec {
        key: "budget_warn_pct",
        kind: "int",
        default: "80",
        min: Some(50),
        max: Some(100),
        pro_max: None,
        values: None,
        help: "Warn at this percentage of a budget",
    },
];

impl Spec {
    /// Produce the T1 entry for this key.
    fn to_json(&self, stored: Option<&str>, pro: bool) -> Value {
        let mut object = serde_json::Map::new();
        object.insert("key".to_string(), json!(self.key));
        object.insert(
            "value".to_string(),
            typed_value(self.kind, stored.unwrap_or(self.default)),
        );
        object.insert("type".to_string(), json!(self.kind));
        if let Some(min) = self.min {
            object.insert("min".to_string(), json!(min));
        }
        if let Some(max) = self.max {
            object.insert(
                "max".to_string(),
                json!(if pro {
                    self.pro_max.unwrap_or(max)
                } else {
                    max
                }),
            );
        }
        if let Some(pro_max) = self.pro_max {
            object.insert("pro_max".to_string(), json!(pro_max));
        }
        if let Some(values) = self.values {
            object.insert(
                "values".to_string(),
                json!(
                    values
                        .iter()
                        .map(|value| (*value).to_string())
                        .collect::<Vec<_>>()
                ),
            );
        }
        object.insert("help".to_string(), json!(self.help));
        Value::Object(object)
    }

    /// Validate and normalise a T2 value into its stored string form.
    fn validate(&self, value: &Value) -> Result<String, ApiError> {
        match self.kind {
            "bool" => value
                .as_bool()
                .map(|flag| if flag { "1" } else { "0" }.to_string())
                .ok_or_else(|| ApiError::query_invalid(format!("{} must be a boolean", self.key))),
            "int" => {
                let number = value.as_i64().ok_or_else(|| {
                    ApiError::query_invalid(format!("{} must be an integer", self.key))
                })?;
                let min = self.min.unwrap_or(i64::MIN);
                let max = self.pro_max.unwrap_or(self.max.unwrap_or(i64::MAX));
                if number < min || number > max {
                    return Err(ApiError::query_invalid(format!(
                        "{} must be {min}..={max}",
                        self.key
                    )));
                }
                Ok(number.to_string())
            }
            "enum" => {
                let text = value.as_str().ok_or_else(|| {
                    ApiError::query_invalid(format!("{} must be a string", self.key))
                })?;
                if self.key == "timezone" {
                    text.parse::<chrono_tz::Tz>().map_err(|_| {
                        ApiError::query_invalid(format!("unknown timezone: {text}"))
                    })?;
                    return Ok(text.to_string());
                }
                if self.values.is_some_and(|values| values.contains(&text)) {
                    Ok(text.to_string())
                } else {
                    Err(ApiError::query_invalid(format!(
                        "invalid {}: {text}",
                        self.key
                    )))
                }
            }
            other => Err(ApiError::query_invalid(format!(
                "unknown setting type: {other}"
            ))),
        }
    }
}

fn typed_value(kind: &str, stored: &str) -> Value {
    match kind {
        "bool" => json!(stored == "1"),
        "int" => json!(stored.parse::<i64>().unwrap_or_default()),
        _ => json!(stored),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)] // tests, not daemon paths

    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    async fn test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(SqliteConnectOptions::new().in_memory(true))
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE settings (
                key TEXT PRIMARY KEY, value TEXT NOT NULL,
                created_at INTEGER NOT NULL DEFAULT 0, updated_at INTEGER NOT NULL DEFAULT 0)",
        )
        .execute(&pool)
        .await
        .unwrap();
        for spec in SPECIFICATIONS {
            sqlx::query("INSERT INTO settings (key, value) VALUES (?1, ?2)")
                .bind(spec.key)
                .bind(spec.default)
                .execute(&pool)
                .await
                .unwrap();
        }
        sqlx::query(
            "CREATE TABLE license (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                tier TEXT NOT NULL DEFAULT 'free')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO license (id) VALUES (1)")
            .execute(&pool)
            .await
            .unwrap();
        pool
    }

    #[tokio::test]
    async fn free_retention_ceiling_is_enforced() {
        let pool = test_db().await;
        let entry = update(&pool, "retention_days", &json!(90)).await.unwrap();
        assert_eq!(entry["value"], 90);
        let err = update(&pool, "retention_days", &json!(120))
            .await
            .unwrap_err();
        assert_eq!(err.code(), ErrorCode::ExportProRequired);
        let stored: String =
            sqlx::query_scalar("SELECT value FROM settings WHERE key = 'retention_days'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(stored, "90");

        sqlx::query("UPDATE license SET tier = 'pro'")
            .execute(&pool)
            .await
            .unwrap();
        let entry = update(&pool, "retention_days", &json!(120)).await.unwrap();
        assert_eq!(entry["value"], 120);
    }

    #[tokio::test]
    async fn invalid_settings_are_rejected() {
        let pool = test_db().await;
        for (key, value) in [
            ("bogus", json!(1)),
            ("retention_days", json!("nope")),
            ("start_on_boot", json!("yes")),
            ("theme_default", json!("neon")),
            ("billing_cycle_day", json!(32)),
            ("speed_unit", json!("kph")),
            ("budget_warn_pct", json!(120)),
            ("budget_total_daily_mb", json!(-1)),
            ("timezone", json!("Mars/Olympus")),
        ] {
            assert_eq!(
                update(&pool, key, &value).await.unwrap_err().code(),
                ErrorCode::QueryInvalid,
                "key {key} with {value}"
            );
        }
    }
}

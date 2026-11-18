//! Pure budget evaluation for PRD §6.1 #8 / D6a (usage budgets are free tier).
//!
//! No I/O lives here (ARCHITECTURE.md §4): callers supply interface-filtered
//! byte totals and the configured limits. A limit of zero means "not set", so
//! an unconfigured budget never produces a warning or a breach.

/// How close usage is to a configured budget (DESIGN.md §7.14).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetLevel {
    /// No budget configured, or usage is below the warning threshold.
    Ok,
    /// Usage reached the configured warning percentage (default 80).
    Warning,
    /// Usage reached or exceeded the budget.
    Breach,
}

/// Evaluate one budget. `used` and `limit` are bytes; `warn_pct` is 1..=100.
///
/// A zero limit disables the budget. The warning threshold is inclusive so a
/// 100% warning setting fires at the same point as a breach (callers should
/// prefer `Breach` in that case — handled below).
#[must_use]
pub fn evaluate(used: u64, limit: u64, warn_pct: u8) -> BudgetLevel {
    if limit == 0 {
        return BudgetLevel::Ok;
    }
    if used >= limit {
        return BudgetLevel::Breach;
    }
    let warn_pct = warn_pct.clamp(1, 100);
    if used.saturating_mul(100) >= limit.saturating_mul(u64::from(warn_pct)) {
        return BudgetLevel::Warning;
    }
    BudgetLevel::Ok
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)] // tests

    use super::*;

    #[test]
    fn zero_limit_is_never_a_breach() {
        assert_eq!(evaluate(1_000, 0, 80), BudgetLevel::Ok);
    }

    #[test]
    fn warning_and_breach_boundaries() {
        assert_eq!(evaluate(79, 100, 80), BudgetLevel::Ok);
        assert_eq!(evaluate(80, 100, 80), BudgetLevel::Warning);
        assert_eq!(evaluate(99, 100, 80), BudgetLevel::Warning);
        assert_eq!(evaluate(100, 100, 80), BudgetLevel::Breach);
        assert_eq!(evaluate(120, 100, 80), BudgetLevel::Breach);
    }

    #[test]
    fn hundred_percent_warning_still_reports_breach_first() {
        assert_eq!(evaluate(100, 100, 100), BudgetLevel::Breach);
    }
}

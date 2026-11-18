//! Pure helpers for the two daemon-consumed settings (DATABASE.md §2.6).

/// systemd unit managed by the root daemon (packaging/systemd/netwatchd.service).
pub const SERVICE_UNIT: &str = "netwatchd.service";

/// Default `is_metered` for a newly discovered interface (OQ5 manual MVP).
///
/// `label` auto-labels new interfaces as metered. `ask` waits for the user to
/// label them through I2. `auto` would require `NetworkManager` integration,
/// which is not in MVP — it falls back to `ask` honestly.
#[must_use]
pub fn metered_default(policy: &str) -> bool {
    // `ask`, `auto` (no detector in MVP) and unknown values share the fallback.
    matches!(policy, "label")
}

/// systemctl action matching a `start_on_boot` value.
#[must_use]
pub fn start_on_boot_action(enabled: bool) -> &'static str {
    if enabled { "enable" } else { "disable" }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn metered_policy_defaults() {
        assert!(metered_default("label"));
        assert!(!metered_default("ask"));
        assert!(!metered_default("auto"));
        assert!(!metered_default("bogus"));
    }

    #[test]
    fn start_on_boot_maps_to_systemctl_action() {
        assert_eq!(start_on_boot_action(true), "enable");
        assert_eq!(start_on_boot_action(false), "disable");
        assert_eq!(SERVICE_UNIT, "netwatchd.service");
    }
}

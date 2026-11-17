//! Daemon domain logic: accounting, attribution, retention, rollups.
//!
//! ARCHITECTURE.md §4: no I/O dependencies here — this crate is the pure
//! correctness core (TESTING.md §6 targets ≥ 90% coverage).
//! Modules land per-feature: accounting.rs, attribution.rs, retention.rs.

pub mod accounting;
pub mod attribution;
pub mod episode;
pub mod interfaces;
pub mod retention;

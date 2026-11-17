//! Process-level integration tests for Net Watch Pro (TESTING.md §1).
//!
//! The crate intentionally has no library code — only `tests/` targets,
//! which drive the real `netwatchd` binary against temp SQLite databases
//! and the Unix-socket API. Run them with:
//!
//! ```sh
//! cargo build -p netwatchd
//! cargo test -p netwatch-integration-tests
//! ```
//!
//! Note: none of these tests need the eBPF collector (P2.1–P2.3, blocked
//! pending the Ubuntu/Fedora VMs). Fixture data is seeded through SQLite
//! directly, so everything here runs on the developer host.

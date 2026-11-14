//! Net Watch Pro UI shell — deliberately thin (ARCHITECTURE.md §4):
//! window lifecycle, tray (P7.5), daemon token link (P1.5) land later.
//! No product logic lives in this crate.

// Prevents an extra console window on Windows; harmless on Linux.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![deny(clippy::unwrap_used, clippy::expect_used)]
#![deny(missing_docs)]

fn main() {
    let builder = tauri::Builder::default();
    if let Err(err) = builder.run(tauri::generate_context!()) {
        // Startup failure: fail fast with a distinct message; the one place
        // a crash is acceptable (ARCHITECTURE.md §7.1 fail-fast startup).
        eprintln!("netwatch-app: failed to start: {err}");
        std::process::exit(1);
    }
}

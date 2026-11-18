//! Tray icon and menu (P7.5, PRD Flow 6).
//!
//! The tray belongs to the unprivileged UI: "Quit" exits the app only, never
//! the root daemon (which is a separate systemd service). The tooltip shows
//! today's usage, pushed from the UI's `usage.totals` feed.

use tauri::menu::{CheckMenuItem, MenuBuilder, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

/// Stable id used by the tooltip command to look up the tray icon.
pub const TRAY_ID: &str = "nwp-main";
/// Event carrying the "pause UI notifications" checkbox state to the UI.
pub const PAUSE_EVENT: &str = "tray://pause-notifications";

/// Build the tray icon and its menu. Failure is reported to the caller, which
/// logs it as degraded state instead of aborting startup.
///
/// # Errors
///
/// Propagates Tauri menu/icon construction failures.
pub fn setup(app: &AppHandle) -> Result<(), String> {
    let open = MenuItem::with_id(app, "open", "Open Net Watch Pro", true, None::<&str>)
        .map_err(|error| format!("tray open item: {error}"))?;
    let pause = CheckMenuItem::with_id(
        app,
        "pause_notifications",
        "Pause UI notifications",
        true,
        false,
        None::<&str>,
    )
    .map_err(|error| format!("tray pause item: {error}"))?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)
        .map_err(|error| format!("tray quit item: {error}"))?;
    let separator =
        PredefinedMenuItem::separator(app).map_err(|error| format!("tray separator: {error}"))?;
    let separator_two =
        PredefinedMenuItem::separator(app).map_err(|error| format!("tray separator: {error}"))?;
    let menu = MenuBuilder::new(app)
        .items(&[&open, &separator, &pause, &separator_two, &quit])
        .build()
        .map_err(|error| format!("tray menu: {error}"))?;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| "no default window icon for tray".to_string())?;

    let pause_for_event = pause.clone();
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("Net Watch Pro — monitoring")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "open" => show_main(app),
            "pause_notifications" => {
                let checked = pause_for_event.is_checked().unwrap_or(false);
                let _ = app.emit(PAUSE_EVENT, checked);
            }
            "quit" => app.exit(0), // UI only — the root daemon keeps running.
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { .. } = event {
                show_main(tray.app_handle());
            }
        })
        .build(app)
        .map_err(|error| format!("tray icon: {error}"))?;
    Ok(())
}

fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

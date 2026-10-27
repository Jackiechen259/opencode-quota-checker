//! Main window helpers.
//!
//! The main window itself is declared in `tauri.conf.json`; this module only
//! holds the reopen path used by the tray when no main window exists.

use tauri::webview::WebviewWindowBuilder;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow};

/// Installs the close-request policy on a main window.
///
/// Every main window instance must carry this handler — the one declared in
/// `tauri.conf.json` and any window recreated by [`open`] after the original
/// was destroyed. A window without it still *looks* fine: the caption × closes
/// the webview, the process keeps running in the tray with no visible window,
/// and the application becomes impossible to quit. That failure mode is
/// indistinguishable from "the close button does nothing", so the handler is
/// installed from one place for every creation path.
pub fn install_close_handler(window: &WebviewWindow, app: &AppHandle) {
    let handle = app.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { .. } = event {
            tracing::info!("main window close request; exiting");
            match crate::actions::main_close_action() {
                crate::actions::MainCloseAction::Exit => crate::actions::quit(&handle),
                // Unreachable by design: closing never hides. The arm exists
                // so the close policy stays a single decision.
                crate::actions::MainCloseAction::HideToTray => {}
            }
        }
    });
}

/// Recreates the main window (used by tray recovery as a safety net).
pub fn open(app: &AppHandle) -> Result<(), String> {
    if app.get_webview_window("main").is_some() {
        return Ok(());
    }
    let builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("OpenCode Quota Checker")
        .inner_size(1_440.0, 900.0)
        .min_inner_size(680.0, 620.0)
        .decorations(false)
        .resizable(true)
        .center();
    let builder = builder
        .icon(crate::icons::window())
        .map_err(|error| error.to_string())?;
    let window = builder.build().map_err(|error| error.to_string())?;
    // A recreated window is a new native window: the close policy does not
    // survive the original being destroyed, so it has to be re-attached here.
    install_close_handler(&window, app);
    Ok(())
}

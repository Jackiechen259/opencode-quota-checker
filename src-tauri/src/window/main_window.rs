//! Main window helpers.
//!
//! The main window itself is declared in `tauri.conf.json`; this module only
//! holds the reopen path used by the tray when no main window exists.

use tauri::webview::WebviewWindowBuilder;
use tauri::{AppHandle, Manager, WebviewUrl};

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
    builder
        .build()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

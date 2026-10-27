//! Floating window commands.
//!
//! Every command is a thin wrapper around `float::controller`, which owns the
//! native window, the runtime state, event emission and persistence. Opening a
//! window must run on the main thread, so the mutating commands stay
//! synchronous; only the read-only snapshot is async so its mutex acquisition
//! never blocks the event loop.

use crate::config::FloatLayout;
use crate::float::controller;
use crate::state::FloatStateDto;
use tauri::AppHandle;

/// Returns the current floating-window snapshot.
#[tauri::command]
pub async fn get_float_state(app: AppHandle) -> FloatStateDto {
    controller::snapshot(&app)
}

/// Opens (or focuses) the floating window.
///
/// Async on purpose: on Windows, creating a webview from a synchronous command
/// deadlocks (WebView2 must be created outside the command/event handler
/// thread). Tauri runs async commands on its runtime, which is the supported
/// context for `WebviewWindowBuilder::build`.
#[tauri::command]
pub async fn open_float_window(app: AppHandle) -> Result<(), String> {
    controller::open(&app)
}

/// Closes the floating window through the normal close path.
#[tauri::command]
pub fn close_float_window(app: AppHandle) -> Result<(), String> {
    controller::close(&app);
    Ok(())
}

/// Switches the persisted floating-window layout (Full / Compact).
#[tauri::command]
pub fn set_float_mode(app: AppHandle, mode: FloatLayout) -> Result<(), String> {
    controller::set_layout(&app, mode);
    Ok(())
}

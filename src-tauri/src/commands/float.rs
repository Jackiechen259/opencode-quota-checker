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

/// Applies the content-fitted strip length measured by the docked view.
///
/// The length runs along the strip axis: the width of the top strip, the
/// height of a side rail. The backend still owns the native size: this only
/// records the requested length and resizes while docked (clamped in
/// `float::layout`).
#[tauri::command]
pub fn set_float_docked_length(app: AppHandle, length: f64) -> Result<(), String> {
    controller::set_docked_length(&app, length);
    Ok(())
}

/// Expands or collapses the hover card of the docked strip.
///
/// The webview reports the hover intent only; the controller owns the native
/// size, the anchored position and the corner region, and reports the result
/// back through `float://state` (`dockHover`). A request is ignored while the
/// window is not docked, so a stale hover cannot resize an un-docked card.
#[tauri::command]
pub fn set_float_dock_hover(app: AppHandle, hover: bool) -> Result<(), String> {
    controller::set_dock_hover(&app, hover);
    Ok(())
}

//! Floating window commands.
//!
//! These are synchronous commands: they run on the main thread, which is
//! required for window creation.

use crate::actions;
use crate::config::FloatMode;
use crate::state::{AppState, FloatStateDto};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

/// Returns the current floating-window snapshot.
///
/// Async so the mutex acquisition never runs on the Tauri main thread.
#[tauri::command]
pub async fn get_float_state(app: AppHandle) -> FloatStateDto {
    let state = app.state::<Arc<AppState>>().inner().clone();
    state.float_dto()
}

/// Opens (or focuses) the floating window.
#[tauri::command]
pub fn open_float_window(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let state = state.inner().clone();
    if app.get_webview_window("float").is_some() {
        if let Some(window) = app.get_webview_window("float") {
            let _ = window.show();
            let _ = window.set_focus();
        }
        return Ok(());
    }
    crate::window::float_window::open(&app, state.clone())?;
    state.floating.lock().expect("float mutex").open = true;
    crate::tray::emit_float_state(&app);
    crate::persistence::persist_config(&app);
    Ok(())
}

/// Closes the floating window through the normal close path.
#[tauri::command]
pub fn close_float_window(app: AppHandle) -> Result<(), String> {
    actions::close_float(&app);
    Ok(())
}

/// Switches the floating window mode (Full / Compact / Docked).
#[tauri::command]
pub fn set_float_mode(app: AppHandle, mode: FloatMode) -> Result<(), String> {
    actions::change_float_mode(&app, mode);
    Ok(())
}

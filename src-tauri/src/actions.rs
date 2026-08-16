//! Shared window actions used by the tray, IPC commands, and window events.

use crate::config::FloatMode;
use crate::persistence::persist_config;
use crate::state::AppState;
use crate::window::float_window;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

/// Response to a main-window close request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainCloseAction {
    /// Terminate the process.
    Exit,
    /// Hide to the tray, keeping the daemon alive.
    ///
    /// Never constructed: closing must not hide. The variant documents the
    /// behavior that was deliberately removed so a future change cannot
    /// silently reintroduce close-to-tray through this decision point.
    #[allow(dead_code)]
    HideToTray,
}

/// Decides what a main-window close request (caption ×, Alt+F4) does.
///
/// Closing always exits. Hiding to the tray is an explicit action (header
/// menu "隐藏主窗口" / tray menu); closing and hiding never share a path.
/// The `close_behavior` config field is retained for schema compatibility
/// but deliberately no longer consulted here.
pub fn main_close_action() -> MainCloseAction {
    MainCloseAction::Exit
}

/// Shows (or recreates) and focuses the main window.
pub fn show_main(app: &AppHandle) {
    match app.get_webview_window("main") {
        Some(window) => {
            let _ = window.show();
            let _ = window.unminimize();
            let _ = window.set_focus();
        }
        None => {
            if let Err(error) = crate::window::main_window::open(app) {
                tracing::error!(%error, "failed to reopen the main window");
            }
        }
    }
}

/// Hides the main window while keeping the daemon (and monitoring) alive.
///
/// This is the only path that hides to the tray; it is always explicit.
pub fn hide_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        tracing::info!("main window hidden to tray");
        let _ = window.hide();
    }
}

/// Opens (or focuses) the floating window.
pub fn open_float(app: &AppHandle) {
    let state = app.state::<Arc<AppState>>().inner().clone();
    if let Some(window) = app.get_webview_window("float") {
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    if let Err(error) = float_window::open(app, &state) {
        tracing::error!(%error, "failed to open the floating window");
        return;
    }
    state.floating.lock().expect("float mutex").open = true;
    crate::tray::emit_float_state(app);
    persist_config(app);
}

/// Closes the floating window through the normal close path.
pub fn close_float(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("float") {
        let _ = window.close();
    }
    // The close-requested handler also runs this bookkeeping; running it here
    // covers every caller (tray, commands) even when no window exists.
    finish_close_float(app);
}

/// Bookkeeping shared by the float close-request handler and `close_float`.
pub fn finish_close_float(app: &AppHandle) {
    let state = app.state::<Arc<AppState>>().inner().clone();
    state.floating.lock().expect("float mutex").open = false;
    crate::tray::emit_float_state(app);
    persist_config(app);
}

/// Toggles the floating window.
pub fn toggle_float(app: &AppHandle) {
    if app.get_webview_window("float").is_some() {
        close_float(app);
    } else {
        open_float(app);
    }
}

/// Switches the floating window mode and resizes it to the mode's size.
pub fn change_float_mode(app: &AppHandle, mode: FloatMode) {
    let state = app.state::<Arc<AppState>>().inner().clone();
    {
        let mut floating = state.floating.lock().expect("float mutex");
        floating.top_docked = mode == FloatMode::Docked;
        if mode != FloatMode::Docked {
            state.config.lock().expect("config mutex").float_mode = mode;
        }
    }
    if let Some(window) = app.get_webview_window("float") {
        let effective = float_window::effective_mode(&state);
        let (width, height) = effective.size();
        let _ = window.set_size(tauri::LogicalSize::new(width, height));
        if effective == FloatMode::Docked {
            float_window::snap_to_monitor_top(&window);
        }
    }
    crate::tray::emit_float_state(app);
    persist_config(app);
}

/// Terminates the whole application: monitor task, windows, webviews, process.
pub fn quit(app: &AppHandle) {
    app.exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_close_always_exits() {
        // The caption × and Alt+F4 terminate the process; hiding to the tray
        // is an explicit action only and never reachable through close.
        assert_eq!(main_close_action(), MainCloseAction::Exit);
    }
}

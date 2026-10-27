//! Shared window actions used by the tray, IPC commands, and window events.
//!
//! Floating-window actions live in `crate::float::controller`; this module
//! only owns the main-window lifecycle and process exit.

use crate::state::AppState;
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

/// Terminates the whole application: monitor task, windows, webviews, process.
///
/// The shutdown flag is set first so floating-window close bookkeeping cannot
/// overwrite the persisted `float_open` intent while the process tears down.
pub fn quit(app: &AppHandle) {
    let state = app.state::<Arc<AppState>>().inner().clone();
    state.begin_shutdown();
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

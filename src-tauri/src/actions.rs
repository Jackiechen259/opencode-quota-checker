//! Shared window actions used by the tray, IPC commands, and window events.
//!
//! Floating-window actions live in `crate::float::controller`; this module
//! only owns the main-window lifecycle and process exit.

use crate::config::CloseBehavior;
use crate::state::AppState;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

/// Response to a main-window close request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainCloseAction {
    /// Terminate the process.
    Exit,
    /// Keep the daemon alive and minimize the window to the taskbar.
    MinimizeToTaskbar,
    /// Keep the daemon alive but hide the window; the tray icon recovers it.
    HideToTray,
}

/// Resolves the persisted close preference into a concrete action.
///
/// Hiding to the tray is only possible while a tray icon exists: with no icon
/// the window would become unreachable and the process impossible to quit, so
/// the tray preference degrades to [`MainCloseAction::Exit`] (the same
/// fallback documented for tray initialization failures). Minimizing to the
/// taskbar needs no tray at all.
pub fn resolve_close_action(behavior: CloseBehavior, tray_available: bool) -> MainCloseAction {
    match behavior {
        CloseBehavior::MinimizeToTaskbar => MainCloseAction::MinimizeToTaskbar,
        CloseBehavior::MinimizeToTray if tray_available => MainCloseAction::HideToTray,
        CloseBehavior::MinimizeToTray | CloseBehavior::Exit => MainCloseAction::Exit,
    }
}

/// Applies the configured close behavior to the live application state.
///
/// Locks are acquired one at a time in the documented order (config before
/// tray) and released before returning, so the caller can act on the result.
pub fn main_close_action(app: &AppHandle) -> MainCloseAction {
    let state = app.state::<Arc<AppState>>().inner().clone();
    let behavior = {
        let config = state.config.lock().expect("config mutex");
        config.close_behavior
    };
    let tray_available = state.tray.lock().expect("tray mutex").is_some();
    resolve_close_action(behavior, tray_available)
}

/// Whether a live tray icon can recover a hidden main window.
pub fn tray_available(app: &AppHandle) -> bool {
    let state = app.state::<Arc<AppState>>().inner().clone();
    let tray = state.tray.lock().expect("tray mutex");
    tray.is_some()
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
/// This is the explicit "隐藏主窗口" action (header menu / tray menu). Without
/// a tray icon the hidden window could not be recovered, so the request
/// degrades to a taskbar minimize instead of stranding the process.
pub fn hide_main(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    if !tray_available(app) {
        tracing::warn!("tray unavailable; minimizing instead of hiding the main window");
        let _ = window.minimize();
        return;
    }
    tracing::info!("main window hidden to tray");
    let _ = window.hide();
}

/// Minimizes the main window to the taskbar.
pub fn minimize_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        tracing::info!("main window minimized to taskbar");
        let _ = window.minimize();
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
    fn minimize_to_taskbar_never_hides_or_exits() {
        // The taskbar option works with or without a tray icon: the window
        // stays reachable through its taskbar button.
        for tray_available in [true, false] {
            assert_eq!(
                resolve_close_action(CloseBehavior::MinimizeToTaskbar, tray_available),
                MainCloseAction::MinimizeToTaskbar
            );
        }
    }

    #[test]
    fn tray_preference_hides_only_when_a_tray_exists() {
        assert_eq!(
            resolve_close_action(CloseBehavior::MinimizeToTray, true),
            MainCloseAction::HideToTray
        );
        // A hidden window with no tray icon would be unrecoverable.
        assert_eq!(
            resolve_close_action(CloseBehavior::MinimizeToTray, false),
            MainCloseAction::Exit
        );
    }

    #[test]
    fn exit_always_exits() {
        for tray_available in [true, false] {
            assert_eq!(
                resolve_close_action(CloseBehavior::Exit, tray_available),
                MainCloseAction::Exit
            );
        }
    }
}

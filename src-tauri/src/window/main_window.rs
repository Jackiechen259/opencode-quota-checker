//! Main window helpers: close policy, native rounded shape, and the reopen
//! path used by the tray when no main window exists.
//!
//! The window itself is declared in `tauri.conf.json`; every runtime concern
//! that must survive a window recreation is installed here.

use tauri::webview::WebviewWindowBuilder;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow};

/// Logical corner radius of the main window.
///
/// Must match `--radius-frame` in `src/styles/tokens.css`: the CSS rounded
/// clip and the native window region then share one arc, so the titlebar
/// hover cannot leave a white wedge outside the window corner.
#[cfg(target_os = "windows")]
const MAIN_WINDOW_CORNER_RADIUS: f32 = 12.0;

/// Applies the native rounded-window shape and keeps it in sync with
/// resizes, DPI changes and the maximized/fullscreen state.
///
/// The webview is opaque, so the square native window corner would otherwise
/// show outside the CSS radius — most visibly as a white wedge under the red
/// close hover. A window region is the only way to cut the actual HWND; it
/// lives in window coordinates, so it has to be reapplied whenever the
/// window geometry changes.
pub fn install_window_shape(window: &WebviewWindow, app: &AppHandle) {
    #[cfg(target_os = "windows")]
    {
        refresh_window_shape(window);
        let handle = app.clone();
        window.on_window_event(move |event| match event {
            tauri::WindowEvent::Resized(_) | tauri::WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(window) = handle.get_webview_window("main") {
                    refresh_window_shape(&window);
                }
            }
            _ => {}
        });
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (window, app);
    }
}

/// Maximized/fullscreen windows must be square (the screen corners belong to
/// the OS); everything else follows the CSS frame radius.
#[cfg(target_os = "windows")]
fn refresh_window_shape(window: &WebviewWindow) {
    if window.is_maximized().unwrap_or(false) || window.is_fullscreen().unwrap_or(false) {
        crate::window::win::clear_round_corners(window);
    } else if !window.is_minimized().unwrap_or(false) {
        crate::window::win::round_corners(window, MAIN_WINDOW_CORNER_RADIUS);
    }
}

/// Installs the close-request policy on a main window.
///
/// Every main window instance must carry this handler — the one declared in
/// `tauri.conf.json` and any window recreated by [`open`] after the original
/// was destroyed. A window without it still *looks* fine: the caption × closes
/// the webview, the process keeps running in the tray with no visible window,
/// and the application becomes impossible to quit. That failure mode is
/// indistinguishable from "the close button does nothing", so the handler is
/// installed from one place for every creation path.
///
/// The policy itself is the persisted `close_behavior` (see
/// [`crate::actions::resolve_close_action`]): minimize to taskbar, hide to
/// tray, or exit. Every non-exit path must prevent the native close — letting
/// the window close would destroy the webview the rest of the app talks to.
pub fn install_close_handler(window: &WebviewWindow, app: &AppHandle) {
    let handle = app.clone();
    window.on_window_event(move |event| {
        let tauri::WindowEvent::CloseRequested { api, .. } = event else {
            return;
        };
        match crate::actions::main_close_action(&handle) {
            crate::actions::MainCloseAction::Exit => {
                tracing::info!("main window close request; exiting");
                crate::actions::quit(&handle);
            }
            crate::actions::MainCloseAction::MinimizeToTaskbar => {
                tracing::info!("main window close request; minimizing to taskbar");
                api.prevent_close();
                crate::actions::minimize_main(&handle);
            }
            crate::actions::MainCloseAction::HideToTray => {
                tracing::info!("main window close request; hiding to tray");
                api.prevent_close();
                crate::actions::hide_main(&handle);
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
    // A recreated window is a new native window: the close policy and the
    // native rounded shape do not survive the original being destroyed, so
    // both have to be re-attached here.
    install_close_handler(&window, app);
    install_window_shape(&window, app);
    Ok(())
}

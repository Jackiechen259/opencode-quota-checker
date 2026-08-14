//! System tray: menu, left-click behavior, and lifecycle.
//!
//! Ported from the archived Iced `platform/tray.rs`. The tray keeps the
//! daemon reachable while the main window is hidden (close-to-tray).

use crate::state::AppState;
use std::sync::Arc;
use tauri::menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

/// Owns the tray icon and the floating-window check item for their lifetime.
pub struct TrayHandle {
    _tray: TrayIcon,
    float_item: CheckMenuItem<tauri::Wry>,
}

/// Creates the native tray menu and wires its events.
pub fn init(app: &AppHandle) -> Result<TrayHandle, String> {
    let show = MenuItem::with_id(app, "show-main", "打开主窗口", true, None::<&str>)
        .map_err(|error| error.to_string())?;
    let hide = MenuItem::with_id(app, "hide-main", "隐藏主窗口", true, None::<&str>)
        .map_err(|error| error.to_string())?;
    let float_item =
        CheckMenuItem::with_id(app, "toggle-float", "显示悬浮窗", true, false, None::<&str>)
            .map_err(|error| error.to_string())?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)
        .map_err(|error| error.to_string())?;
    let menu = Menu::with_items(
        app,
        &[
            &show as &dyn IsMenuItem<tauri::Wry>,
            &hide,
            &float_item,
            &PredefinedMenuItem::separator(app).map_err(|error| error.to_string())?,
            &quit,
        ],
    )
    .map_err(|error| error.to_string())?;

    let tray_icon = TrayIconBuilder::with_id("main-tray")
        .icon(crate::icons::tray().map_err(|error| error.to_string())?)
        .tooltip("OpenCode Quota Checker")
        .menu(&menu)
        // Windows: left click shows the main window, right click opens the
        // menu (macOS keeps the standard click-to-open-menu behavior).
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show-main" => crate::actions::show_main(app),
            "hide-main" => crate::actions::hide_main(app),
            "toggle-float" => crate::actions::toggle_float(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                crate::actions::show_main(tray.app_handle());
            }
        })
        .build(app)
        .map_err(|error| error.to_string())?;

    Ok(TrayHandle {
        _tray: tray_icon,
        float_item,
    })
}

/// Synchronizes the floating-window check mark with the current state.
pub fn sync_float_checked(app: &AppHandle, checked: bool) {
    let state = app.state::<Arc<AppState>>().inner().clone();
    let tray_guard = state.tray.lock().expect("tray mutex");
    if let Some(tray) = tray_guard.as_ref() {
        let _ = tray.float_item.set_checked(checked);
    }
}

/// Emits the current float state after any float mutation.
pub fn emit_float_state(app: &AppHandle) {
    let state = app.state::<Arc<AppState>>().inner().clone();
    let _ = app.emit(crate::events::FLOAT_STATE, state.float_dto());
    sync_float_checked(app, state.float_dto().open);
}

/// Emits the current monitor state after any monitor mutation.
pub fn emit_monitor_state(app: &AppHandle) {
    let state = app.state::<Arc<AppState>>().inner().clone();
    let _ = app.emit(crate::events::MONITOR_STATUS, state.monitor_dto());
}

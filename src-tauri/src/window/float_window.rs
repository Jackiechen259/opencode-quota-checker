//! Floating window: creation, mode sizing, position restore, and top docking.
//!
//! Behavior ported from the archived Iced `window/float_window.rs`:
//!
//! - always-on-top, borderless, non-resizable;
//! - Full / Compact / Docked sizes in logical pixels;
//! - position persisted in physical pixels (stable across mixed-DPI
//!   monitors) and clamped to the nearest monitor work area on restore;
//! - top docking with hysteresis: snapping at 18 px (DPI-scaled) while
//!   undocked, releasing at 24 px while docked;
//! - Windows rounds the opaque surface with a window region and snaps with
//!   `SetWindowPos` in native virtual-desktop coordinates.

use crate::config::{FloatMode, FloatPosition};
use crate::state::AppState;
use std::sync::Arc;
use tauri::webview::WebviewWindowBuilder;
use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindow};

/// Distance from the monitor top that activates the compact dock.
pub const TOP_SNAP_DISTANCE: f64 = 18.0;
/// Larger release distance prevents repeated mode changes near the boundary.
pub const TOP_RELEASE_DISTANCE: f64 = 24.0;

/// Returns the presentation mode currently in effect: Docked while the
/// window is snapped to the monitor top, otherwise the configured mode.
pub fn effective_mode(state: &AppState) -> FloatMode {
    // Bind the guard explicitly: a guard temporary inside an `if` condition
    // lives until the end of the whole statement and would silently nest
    // with the config lock below (lock-ordering rules on `AppState`).
    let top_docked = state.floating.lock().expect("float mutex").top_docked;
    if top_docked {
        FloatMode::Docked
    } else {
        state.config.lock().expect("config mutex").float_mode
    }
}

/// Docking decision with hysteresis.
///
/// `distance` is `window_top - monitor_top` in physical pixels and
/// `scale_factor` scales both thresholds, matching the archived client.
pub fn is_top_docked(currently_docked: bool, distance: f64, scale_factor: f64) -> bool {
    let scale = scale_factor.max(0.01);
    if distance < -TOP_RELEASE_DISTANCE * scale {
        false
    } else if currently_docked {
        distance <= TOP_RELEASE_DISTANCE * scale
    } else {
        distance <= TOP_SNAP_DISTANCE * scale
    }
}

/// Creates the unique floating window with all its event handlers.
pub fn open(app: &AppHandle, state: &AppState) -> Result<(), String> {
    if app.get_webview_window("float").is_some() {
        return Ok(());
    }
    let mode = effective_mode(state);
    let (width, height) = mode.size();

    let builder = WebviewWindowBuilder::new(app, "float", WebviewUrl::App("index.html".into()))
        .title("OpenCode Quota Checker · 悬浮窗")
        .inner_size(width, height)
        .decorations(false)
        .resizable(false)
        .always_on_top(true)
        .visible(false);
    let builder = builder
        .icon(crate::icons::window())
        .map_err(|error| error.to_string())?;

    // Restore the saved position, clamped to a visible monitor work area.
    let saved_position = state.config.lock().expect("config mutex").float_position;
    let window = builder.build().map_err(|error| error.to_string())?;

    if let Some(position) = saved_position {
        restore_saved_position(&window, position);
    }

    // The initial size from the builder is logical; re-assert it so Docked
    // restoration after a previous Docked session is exact.
    let _ = window.set_size(LogicalSize::new(width, height));

    let handle = app.clone();
    window.on_window_event(move |event| match event {
        tauri::WindowEvent::Moved(position) => {
            handle_moved(&handle, position);
        }
        tauri::WindowEvent::Resized(_) =>
        {
            #[cfg(target_os = "windows")]
            if let Some(window) = handle.get_webview_window("float") {
                crate::window::win::round_corners(&window);
            }
        }
        tauri::WindowEvent::CloseRequested { .. } => {
            crate::actions::finish_close_float(&handle);
        }
        _ => {}
    });

    #[cfg(target_os = "windows")]
    crate::window::win::round_corners(&window);

    window.show().map_err(|error| error.to_string())?;
    let _ = window.set_focus();
    Ok(())
}

/// Handles a float-window move: persists the position (debounced), re-runs
/// the docking check in physical coordinates, and snaps/resizes on changes.
fn handle_moved(app: &AppHandle, position: &PhysicalPosition<i32>) {
    let state = app.state::<Arc<AppState>>().inner().clone();

    let position = FloatPosition {
        x: position.x,
        y: position.y,
    };
    state.mark_float_moved(app, position);

    let Some(window) = app.get_webview_window("float") else {
        return;
    };
    // Docking check in physical coordinates against the monitor work area.
    let Some(monitor_top) = crate::window::float_window::monitor_top(&window) else {
        return;
    };
    let scale_factor = window
        .current_monitor()
        .ok()
        .flatten()
        .map_or(1.0, |monitor| monitor.scale_factor());
    let currently_docked = state.floating.lock().expect("float mutex").top_docked;
    let distance = position.y as f64 - monitor_top;
    let top_docked = is_top_docked(currently_docked, distance, scale_factor);

    if top_docked == currently_docked {
        return;
    }
    state.floating.lock().expect("float mutex").top_docked = top_docked;
    let mode = effective_mode(&state);
    let (width, height) = mode.size();
    let _ = window.set_size(LogicalSize::new(width, height));
    if top_docked {
        snap_to_monitor_top(&window);
    }
    crate::tray::emit_float_state(app);
}

/// Monitor work-area top under the window, in physical pixels.
///
/// Windows uses the native work area (taskbar-aware); other platforms use the
/// monitor's top edge, matching the archived client's non-Windows behavior.
pub fn monitor_top(window: &WebviewWindow) -> Option<f64> {
    #[cfg(target_os = "windows")]
    {
        crate::window::win::work_area_top(window).map(|top| top as f64)
    }
    #[cfg(not(target_os = "windows"))]
    {
        window
            .current_monitor()
            .ok()
            .flatten()
            .map(|monitor| monitor.position().y as f64)
    }
}

/// Restores a saved physical position, clamped into the nearest monitor.
///
/// Windows goes through the native adapter (work-area aware, virtual-desktop
/// coordinates); other platforms clamp into the current monitor bounds.
fn restore_saved_position(window: &WebviewWindow, position: FloatPosition) {
    #[cfg(target_os = "windows")]
    {
        crate::window::win::restore_position(window, position);
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = window.set_position(PhysicalPosition::new(position.x, position.y));
        if let (Ok(Some(monitor)), Ok(size)) = (window.current_monitor(), window.outer_size()) {
            if let Ok(position) = window.outer_position() {
                let clamped = clamp_position(position, &monitor, (size.width, size.height));
                let _ = window.set_position(clamped);
            }
        }
    }
}

/// Snaps the floating window to the top of its current monitor work area.
pub fn snap_to_monitor_top(window: &WebviewWindow) {
    #[cfg(target_os = "windows")]
    crate::window::win::snap_to_monitor_top(window);
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(monitor) = window.current_monitor().ok().flatten() {
            let x = window.outer_position().ok().map_or(0, |pos| pos.x);
            let _ = window.set_position(PhysicalPosition::new(x, monitor.position().y));
        }
    }
}

/// Clamps a physical position to the visible bounds of one monitor.
#[cfg(not(target_os = "windows"))]
pub fn clamp_position(
    position: PhysicalPosition<i32>,
    monitor: &tauri::Monitor,
    window_size: (u32, u32),
) -> PhysicalPosition<i32> {
    let maximum_x = (monitor.size().width as i32 - window_size.0 as i32).max(monitor.position().x);
    let maximum_y = (monitor.size().height as i32 - window_size.1 as i32).max(monitor.position().y);
    PhysicalPosition::new(
        position.x.clamp(monitor.position().x, maximum_x),
        position.y.clamp(monitor.position().y, maximum_y),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_snap_uses_hysteresis() {
        assert!(is_top_docked(false, 18.0, 1.0));
        assert!(!is_top_docked(false, 19.0, 1.0));
        assert!(is_top_docked(true, 24.0, 1.0));
        assert!(!is_top_docked(true, 25.0, 1.0));
        assert!(!is_top_docked(false, -25.0, 1.0));
    }

    #[test]
    fn top_snap_accepts_non_zero_monitor_origins() {
        assert!(is_top_docked(false, 18.0, 1.0));
        assert!(!is_top_docked(false, 19.0, 1.0));
    }

    #[test]
    fn top_snap_threshold_scales_with_monitor_dpi() {
        assert!(is_top_docked(false, 27.0, 1.5));
        assert!(!is_top_docked(false, 28.0, 1.5));
        assert!(is_top_docked(true, 36.0, 1.5));
        assert!(!is_top_docked(true, 37.0, 1.5));
    }
}

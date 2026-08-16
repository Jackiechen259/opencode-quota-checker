//! Floating window: creation, mode sizing, position restore, and top docking.
//!
//! Behavior ported from the archived Iced `window/float_window.rs`:
//!
//! - always-on-top, borderless, non-resizable;
//! - Full / Compact / Docked sizes in logical pixels (single source of the
//!   layout constants lives here, matching `float-window.css`);
//! - Full height is dynamic: `FULL_CHROME_HEIGHT + quota_count ×
//!   FULL_CARD_STEP`, clamped to `FULL_MIN_HEIGHT..=FULL_MAX_HEIGHT`;
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

/// Width of the floating widget in logical pixels, shared by every mode.
pub const FLOAT_WIDTH: f64 = 392.0;
/// Compact card height (the hero row flexes to fill the extra space).
pub const COMPACT_HEIGHT: f64 = 168.0;
/// Docked strip height.
pub const DOCKED_HEIGHT: f64 = 48.0;
/// Full mode lower bound: never shrink below the skeleton/empty layout.
pub const FULL_MIN_HEIGHT: f64 = 280.0;
/// Full mode upper bound: beyond this the quota list scrolls.
pub const FULL_MAX_HEIGHT: f64 = 560.0;
/// One quota card plus its gap, in logical pixels.
pub const FULL_CARD_STEP: f64 = 96.0;
/// Fixed Full chrome (padding + header + gaps + meta + footer), logical px.
pub const FULL_CHROME_HEIGHT: f64 = 134.0;

/// Corner radius per presentation mode, in logical pixels.
///
/// Must stay in sync with the CSS `border-radius` in `float-window.css`
/// (`--radius-float` / `--radius-float-docked`); the native region and the
/// rendered card would otherwise show a jagged ring between the two shapes.
pub fn corner_radius(mode: FloatMode) -> f32 {
    match mode {
        FloatMode::Docked => 12.0,
        FloatMode::Full | FloatMode::Compact => 16.0,
    }
}

/// Returns the presentation mode currently in effect: Docked while the
/// window is snapped to the monitor top, otherwise the configured mode.
///
/// Reads `AppConfig.float_mode` — the single persisted source — instead of a
/// duplicated transient copy, so the DTO, the native window size and the
/// frontend render can never disagree. The config guard is a statement
/// temporary (dropped before the floating lock is taken), honoring the
/// lock-ordering rules on `AppState`.
pub fn effective_mode(state: &AppState) -> FloatMode {
    let configured = state.config.lock().expect("config mutex").float_mode;
    let top_docked = state.floating.lock().expect("float mutex").top_docked;
    if top_docked {
        FloatMode::Docked
    } else {
        configured
    }
}

/// Ideal Full height for `quota_count` cards, clamped to the window bounds.
pub fn full_height(quota_count: usize) -> f64 {
    (FULL_CHROME_HEIGHT + FULL_CARD_STEP * quota_count as f64)
        .clamp(FULL_MIN_HEIGHT, FULL_MAX_HEIGHT)
}

/// Ideal Full height for the current usage state.
///
/// While no report exists the skeleton is sized for two cards, so the first
/// load does not resize the window out from under the loading UI.
pub fn full_height_for(state: &AppState) -> f64 {
    let count = state
        .usage
        .lock()
        .expect("usage mutex")
        .report
        .as_ref()
        .map_or(2, |report| report.windows.len().max(1));
    full_height(count)
}

/// Logical size for a presentation mode under the current usage state.
pub fn size_for(mode: FloatMode, state: &AppState) -> (f64, f64) {
    let height = match mode {
        FloatMode::Full => full_height_for(state),
        FloatMode::Compact => COMPACT_HEIGHT,
        FloatMode::Docked => DOCKED_HEIGHT,
    };
    (FLOAT_WIDTH, height)
}

/// Resizes the float window (when it exists) to the ideal size for its
/// current presentation mode and usage state.
///
/// Idempotent and safe to call from any thread; the size always derives from
/// the canonical state, so the native window can never drift from the DTO
/// the frontend renders. Called after every mode/dock change and after every
/// quota refresh.
pub fn sync_float_size(app: &AppHandle) {
    let state = app.state::<Arc<AppState>>().inner().clone();
    let Some(window) = app.get_webview_window("float") else {
        return;
    };
    let mode = effective_mode(&state);
    let (width, height) = size_for(mode, &state);
    let _ = window.set_size(LogicalSize::new(width, height));
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
///
/// The window is created hidden and sized to the effective presentation
/// mode before it is ever shown, so the webview's first frames (a
/// lightweight boot shell until `get_float_state` resolves) already match
/// the native size — no Full-in-a-Compact-window first frame.
pub fn open(app: &AppHandle, state: Arc<AppState>) -> Result<(), String> {
    if app.get_webview_window("float").is_some() {
        return Ok(());
    }
    let mode = effective_mode(&state);
    let (width, height) = size_for(mode, &state);

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
    let state_for_events = state.clone();
    window.on_window_event(move |event| match event {
        tauri::WindowEvent::Moved(position) => {
            handle_moved(&handle, position);
        }
        tauri::WindowEvent::Resized(_) =>
        {
            #[cfg(target_os = "windows")]
            if let Some(window) = handle.get_webview_window("float") {
                let mode = effective_mode(&state_for_events);
                crate::window::win::round_corners_for(&window, corner_radius(mode));
            }
        }
        tauri::WindowEvent::CloseRequested { .. } => {
            crate::actions::finish_close_float(&handle);
        }
        _ => {}
    });

    #[cfg(target_os = "windows")]
    crate::window::win::round_corners_for(&window, corner_radius(mode));

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
    let (width, height) = size_for(mode, &state);
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

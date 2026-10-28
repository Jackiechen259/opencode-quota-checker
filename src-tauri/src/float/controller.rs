//! Floating-window lifecycle.
//!
//! This module owns the unique `float` window and every transition of the
//! floating-window state. Callers (IPC commands, tray, monitor, startup) only
//! delegate here, so opening, closing, docking, resizing, event emission and
//! persistence cannot drift apart.
//!
//! # Lock ordering
//!
//! `AppState` documents the global order `config → tray → monitor → floating →
//! updater → usage`. This module never holds two guards at the same time:
//! every acquisition is a statement temporary or lives in its own block, and
//! `persist_config` (which takes the config lock) is always called after all
//! guards have been dropped.

use crate::config::{FloatLayout, FloatPosition};
use crate::float::layout;
use crate::float::state::{FloatPresentation, FloatStateDto};
use crate::persistence::persist_config;
use crate::state::AppState;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::webview::WebviewWindowBuilder;
use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindow,
};

/// Window label shared by every float-window lookup.
const WINDOW_LABEL: &str = "float";

/// Delay before a drag position is written to disk.
const POSITION_DEBOUNCE_MS: u64 = 750;

/// Serializes window creation.
///
/// `open` can now run on the async runtime (WebView2 creation deadlocks in a
/// synchronous command), so two rapid clicks could otherwise race and try to
/// build the same label twice. This is not part of `AppState`'s lock order: it
/// is never held while acquiring an `AppState` guard in another order.
static OPEN_LOCK: Mutex<()> = Mutex::new(());

fn state(app: &AppHandle) -> Arc<AppState> {
    app.state::<Arc<AppState>>().inner().clone()
}

fn layout_of(state: &AppState) -> FloatLayout {
    state.config.lock().expect("config mutex").float_mode
}

/// Number of quota cards driving the Full height.
fn quota_count_of(state: &AppState) -> usize {
    let usage = state.usage.lock().expect("usage mutex");
    layout::quota_count(usage.report.as_ref())
}

/// Native size for a presentation, applying the last measured Docked width.
///
/// The webview measures the Docked content and reports the total window
/// width; until it does (and again after an undock/redock) the stored width
/// or the default `FLOAT_WIDTH` is used.
fn size_for(state: &AppState, presentation: FloatPresentation) -> (f64, f64) {
    if presentation == FloatPresentation::Docked {
        let width = state
            .floating
            .lock()
            .expect("float mutex")
            .docked_width()
            .unwrap_or(layout::FLOAT_WIDTH);
        return layout::docked_size(width);
    }
    layout::size(presentation, quota_count_of(state))
}

/// Canonical floating-window snapshot.
pub fn snapshot(app: &AppHandle) -> FloatStateDto {
    state(app).float_dto()
}

/// Emits the canonical snapshot and synchronizes the tray checkmark.
///
/// This is the only place that emits `float://state`; every mutation ends
/// here, so the DTO and the tray item cannot lag behind the native window.
pub fn emit_state(app: &AppHandle) {
    let dto = snapshot(app);
    let _ = app.emit(crate::events::FLOAT_STATE, &dto);
    crate::tray::sync_float_checked(app, dto.open);
}

/// Whether a native floating window currently exists.
pub fn is_open(app: &AppHandle) -> bool {
    app.get_webview_window(WINDOW_LABEL).is_some()
}

/// Opens the floating window, or shows and focuses the existing one.
///
/// The window is built hidden and sized before it is shown, so the webview's
/// first frames (the lightweight boot shell until `get_float_state` resolves)
/// already match the native size.
pub fn open(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        window.show().map_err(|error| error.to_string())?;
        let _ = window.set_focus();
        set_open(app, true);
        return Ok(());
    }

    // Serialize creation; another async open may have just built the window
    // between the check above and this point.
    let _open_guard = OPEN_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        window.show().map_err(|error| error.to_string())?;
        let _ = window.set_focus();
        set_open(app, true);
        return Ok(());
    }

    let state = state(app);
    let (layout, saved_position) = {
        let config = state.config.lock().expect("config mutex");
        (config.float_mode, config.float_position)
    };
    let presentation = state
        .floating
        .lock()
        .expect("float mutex")
        .presentation(layout);
    let (width, height) = size_for(&state, presentation);

    let builder =
        WebviewWindowBuilder::new(app, WINDOW_LABEL, WebviewUrl::App("index.html".into()))
            .title("OpenCode Quota Checker · 悬浮窗")
            .inner_size(width, height)
            .decorations(false)
            .resizable(false)
            // Disable the native undecorated shadow: tao would enlarge the
            // outer window by the shadow insets, leaving a dark border around
            // the webview. The surface already has its own rounded outline.
            .shadow(false)
            .always_on_top(true)
            .visible(false);
    let builder = builder
        .icon(crate::icons::window())
        .map_err(|error| error.to_string())?;
    let window = builder.build().map_err(|error| error.to_string())?;

    if let Some(position) = saved_position {
        restore_position(&window, position);
    }
    // The builder size is logical; re-assert it so a Docked restoration after
    // a previous Docked session is exact.
    if let Err(error) = window.set_size(LogicalSize::new(width, height)) {
        tracing::warn!(%error, "failed to apply the floating window's initial size");
    }

    attach_window_handlers(&window, app);

    #[cfg(target_os = "windows")]
    crate::float::win::round_corners_for(&window, layout::corner_radius(presentation));

    if let Err(error) = window.show() {
        // Do not leave an invisible "open" window behind: a failed show must
        // be retryable through the normal open path.
        let _ = window.close();
        return Err(error.to_string());
    }
    let _ = window.set_focus();
    set_open(app, true);
    Ok(())
}

/// Closes the floating window through the normal close path.
pub fn close(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        if let Err(error) = window.close() {
            tracing::warn!(%error, "failed to close the floating window");
            return;
        }
    }
    // The close-requested handler also runs this bookkeeping; running it here
    // covers every caller even when no window exists. The function is
    // idempotent.
    finalize_close(app);
}

/// Toggles the floating window.
pub fn toggle(app: &AppHandle) {
    if is_open(app) {
        close(app);
    } else if let Err(error) = open(app) {
        tracing::error!(%error, "failed to open the floating window");
    }
}

/// Switches the persisted layout and resizes the window to it.
///
/// Setting a layout always leaves the transient Docked presentation, matching
/// the old `apply_float_mode` behavior.
pub fn set_layout(app: &AppHandle, layout: FloatLayout) {
    let state = state(app);
    state.apply_float_layout(layout);
    sync_size(app);
    emit_state(app);
    persist_config(app);
}

/// Applies the Docked width measured by the webview.
///
/// The webview measures its own content (text widths depend on the platform
/// fonts) and reports the total window width. The controller clamps that value
/// and re-applies the native size; nothing is persisted and the DTO is
/// unchanged, so no event is needed.
pub fn set_docked_width(app: &AppHandle, width: f64) {
    let state = state(app);
    let changed = state
        .floating
        .lock()
        .expect("float mutex")
        .set_docked_width(width);
    if changed {
        sync_size(app);
    }
}

/// Re-applies the native size for the current presentation and usage state.
///
/// Idempotent; called after every quota refresh, after config changes and
/// after mode changes. The size always derives from the canonical state, so
/// the native window can never drift from the DTO the frontend renders.
pub fn sync_size(app: &AppHandle) {
    let Some(window) = app.get_webview_window(WINDOW_LABEL) else {
        return;
    };
    let state = state(app);
    let layout = layout_of(&state);
    let presentation = state
        .floating
        .lock()
        .expect("float mutex")
        .presentation(layout);
    let (width, height) = size_for(&state, presentation);
    if let Err(error) = window.set_size(LogicalSize::new(width, height)) {
        tracing::warn!(%error, "failed to sync the floating window size");
    }
}

/// Restores the floating window on launch when `float_open` is set.
pub fn restore_on_startup(app: &AppHandle) {
    let should_restore = state(app).config.lock().expect("config mutex").float_open;
    if should_restore {
        if let Err(error) = open(app) {
            tracing::error!(%error, "failed to restore the floating window");
        }
    }
}

fn attach_window_handlers(window: &WebviewWindow, app: &AppHandle) {
    let handle = app.clone();
    window.on_window_event(move |event| match event {
        tauri::WindowEvent::Moved(position) => on_moved(&handle, position),
        #[cfg(target_os = "windows")]
        tauri::WindowEvent::Resized(_) => refresh_region(&handle),
        #[cfg(target_os = "windows")]
        tauri::WindowEvent::ScaleFactorChanged { .. } => refresh_region(&handle),
        tauri::WindowEvent::CloseRequested { .. } => finalize_close(&handle),
        _ => {}
    });
}

/// Persists the open intent into both the runtime and the config.
///
/// `float_open` is the persisted intent read on launch; `FloatRuntime::open`
/// is the live state used by the DTO. The controller owns both; this helper
/// updates them together, and `finalize_close` follows the same rule.
fn set_open(app: &AppHandle, open: bool) {
    let state = state(app);
    let config_changed = {
        let mut config = state.config.lock().expect("config mutex");
        if config.float_open == open {
            false
        } else {
            config.float_open = open;
            true
        }
    };
    let state_changed = state.floating.lock().expect("float mutex").set_open(open);

    if config_changed {
        persist_config(app);
    }
    if state_changed {
        emit_state(app);
    }
}

/// Bookkeeping shared by the close-request handler and the close command.
///
/// Idempotent: a second invocation (event plus command) only finds
/// `was_open == false` and no pending position, so it writes nothing.
fn finalize_close(app: &AppHandle) {
    let state = state(app);

    if state.is_shutting_down() {
        // The process is exiting: keep `float_open` so the next launch can
        // restore the window, but stop any queued position write.
        let mut runtime = state.floating.lock().expect("float mutex");
        let _ = runtime.take_pending_position();
        runtime.set_open(false);
        return;
    }

    let (was_open, pending_position) = {
        let mut runtime = state.floating.lock().expect("float mutex");
        let was_open = runtime.is_open();
        let pending_position = runtime.take_pending_position();
        if was_open {
            runtime.set_open(false);
        }
        (was_open, pending_position)
    };

    if was_open {
        {
            let mut config = state.config.lock().expect("config mutex");
            config.float_open = false;
        }
        persist_config(app);
        emit_state(app);
    } else if pending_position {
        // A move arrived after the window was already gone; flush the last
        // position without touching the open intent.
        persist_config(app);
    }
}

/// Handles a float-window move: persists the position, runs the docking
/// decision in physical coordinates, and snaps/resizes on changes.
fn on_moved(app: &AppHandle, position: &PhysicalPosition<i32>) {
    let Some(window) = app.get_webview_window(WINDOW_LABEL) else {
        return;
    };
    let state = state(app);

    {
        let mut config = state.config.lock().expect("config mutex");
        config.float_position = Some(FloatPosition {
            x: position.x,
            y: position.y,
        });
    }
    let generation = state.floating.lock().expect("float mutex").begin_move();

    let Some(monitor_top) = monitor_top(&window) else {
        schedule_position_persist(app, generation);
        return;
    };
    let scale_factor = window
        .current_monitor()
        .ok()
        .flatten()
        .map_or(1.0, |monitor| monitor.scale_factor());
    let layout = layout_of(&state);
    let currently_docked = state.floating.lock().expect("float mutex").is_docked();
    let distance = position.y as f64 - monitor_top;
    let top_docked = layout::is_top_docked(currently_docked, distance, scale_factor);

    if top_docked != currently_docked {
        state
            .floating
            .lock()
            .expect("float mutex")
            .set_docked(top_docked);
        let presentation = if top_docked {
            FloatPresentation::Docked
        } else {
            FloatPresentation::from(layout)
        };
        let (width, height) = size_for(&state, presentation);
        if let Err(error) = window.set_size(LogicalSize::new(width, height)) {
            tracing::warn!(%error, "failed to resize the floating window for docking");
        }
        if top_docked {
            snap_to_monitor_top(&window);
        }
        emit_state(app);
    }

    schedule_position_persist(app, generation);
}

fn schedule_position_persist(app: &AppHandle, generation: u64) {
    let state = state(app);
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(POSITION_DEBOUNCE_MS)).await;
        let should_persist = state
            .floating
            .lock()
            .expect("float mutex")
            .settle_move(generation);
        if should_persist {
            persist_config(&handle);
        }
    });
}

/// Monitor work-area top under the window, in physical pixels.
///
/// Windows uses the native work area (taskbar-aware); other platforms use the
/// monitor's top edge.
fn monitor_top(window: &WebviewWindow) -> Option<f64> {
    #[cfg(target_os = "windows")]
    {
        crate::float::win::work_area_top(window).map(|top| top as f64)
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

/// Snaps the floating window to the top of its current monitor work area.
fn snap_to_monitor_top(window: &WebviewWindow) {
    #[cfg(target_os = "windows")]
    {
        crate::float::win::snap_to_monitor_top(window);
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(monitor) = window.current_monitor().ok().flatten() {
            let x = window
                .outer_position()
                .ok()
                .map_or(0, |position| position.x);
            let _ = window.set_position(PhysicalPosition::new(x, monitor.position().y));
        }
    }
}

/// Restores a saved physical position, clamped into the nearest monitor.
fn restore_position(window: &WebviewWindow, position: FloatPosition) {
    #[cfg(target_os = "windows")]
    {
        crate::float::win::restore_position(window, position);
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = window.set_position(PhysicalPosition::new(position.x, position.y));
        if let (Ok(Some(monitor)), Ok(size)) = (window.current_monitor(), window.outer_size()) {
            if let Ok(current) = window.outer_position() {
                let (x, y) = layout::clamp_position(
                    (current.x, current.y),
                    (monitor.position().x, monitor.position().y),
                    (monitor.size().width, monitor.size().height),
                    (size.width, size.height),
                );
                let _ = window.set_position(PhysicalPosition::new(x, y));
            }
        }
    }
}

/// Re-applies the native corner region after a resize or DPI change.
#[cfg(target_os = "windows")]
fn refresh_region(app: &AppHandle) {
    let Some(window) = app.get_webview_window(WINDOW_LABEL) else {
        return;
    };
    let state = state(app);
    let layout = layout_of(&state);
    let presentation = state
        .floating
        .lock()
        .expect("float mutex")
        .presentation(layout);
    crate::float::win::round_corners_for(&window, layout::corner_radius(presentation));
}

//! Floating-window lifecycle.
//!
//! This module owns the unique `float` window and every transition of the
//! floating-window state. Callers (IPC commands, tray, monitor, startup) only
//! delegate here, so opening, closing, docking, resizing, event emission and
//! persistence cannot drift apart.
//!
//! # Docking and dragging
//!
//! Docking is decided on every `Moved` event from the window's distances to the
//! monitor work-area edges (see `float::layout::plan_dock`):
//!
//! * the top edge produces the one-line horizontal strip and un-docks
//!   immediately — growing back into the configured layout keeps the top-left
//!   corner, so the card stays on screen;
//! * the left/right edges produce a vertical rail. Expanding a rail mid-drag
//!   would put most of a right-edge card off-screen: while the mouse button is
//!   down, the native move loop owns the window position and undoes any
//!   repositioning. A side un-dock is therefore *pending* until the drag
//!   settles, then the rail expands (or docks to another edge it was dropped
//!   against) and is clamped into the work area.
//! * after an un-dock, docking stays guarded until the window leaves every snap
//!   band, so a card dropped flush against an edge cannot immediately collapse
//!   back into a rail.
//!
//! # Docked hover card
//!
//! Resting the pointer on a docked strip asks the controller to grow the same
//! window into a slightly larger card (`layout::dock_hover_size`). The
//! expansion is anchored to the docked edge (`layout::docked_position`) and its
//! along-edge coordinate is remembered, so collapsing restores the strip where
//! it was even when the card had to clamp into the work area. The flag is
//! transient: the docking decision clears it when the window leaves (or starts
//! to leave) the edge, a close/rebuild clears it, and the expanded card never
//! writes `AppConfig.float_position` — that position is derived from the strip
//! anchor, not chosen by the user.
//!
//! # Lock ordering
//!
//! `AppState` documents the global order `config → tray → monitor → floating →
//! updater → usage`. This module never holds two guards at the same time:
//! every acquisition is a statement temporary or lives in its own block, and
//! `persist_config` (which takes the config lock) is always called after all
//! guards have been dropped.

use crate::config::{FloatLayout, FloatPosition};
use crate::float::layout::{self, EdgeDistances};
use crate::float::state::{DockPlan, FloatEdge, FloatPresentation, FloatStateDto};
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

/// Quiet period after the last move before a docked window is settled.
///
/// The settle pass re-snaps docked windows flush to their edge and expands a
/// pending side rail; it must only run once the native move loop has ended.
const DOCK_SETTLE_MS: u64 = 220;

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

/// Native size for a presentation, applying the last measured strip length.
///
/// The webview measures the strip content and reports its length; until it does
/// (and again after an undock/redock) the stored length or the presentation
/// default is used. The top strip uses it as its width, the side rails as their
/// height.
///
/// `dock_hover` expands a docked window into the fixed hover card instead;
/// callers pass the runtime flag explicitly so a transition can size the window
/// it is *becoming* rather than the one it still is.
fn size_for(state: &AppState, presentation: FloatPresentation, dock_hover: bool) -> (f64, f64) {
    if dock_hover && presentation.docked_edge().is_some() {
        return layout::dock_hover_size();
    }
    let length = state.floating.lock().expect("float mutex").docked_length();
    match presentation {
        FloatPresentation::DockedTop => layout::docked_size(length.unwrap_or(layout::FLOAT_WIDTH)),
        FloatPresentation::DockedLeft | FloatPresentation::DockedRight => {
            layout::side_docked_size(length.unwrap_or(layout::SIDE_DOCKED_DEFAULT_LENGTH))
        }
        FloatPresentation::Full | FloatPresentation::Compact => {
            layout::size(presentation, quota_count_of(state))
        }
    }
}

/// Native size the current presentation requires.
fn desired_size(state: &AppState) -> (f64, f64) {
    let layout = layout_of(state);
    let (presentation, dock_hover) = {
        let runtime = state.floating.lock().expect("float mutex");
        (runtime.presentation(layout), runtime.is_dock_hover())
    };
    size_for(state, presentation, dock_hover)
}

/// Applies [`desired_size`] to the native window and returns it.
///
/// Returning the target size lets callers position the window against the size
/// it is *becoming*: Tauri may apply a resize asynchronously (especially from
/// the async settle task), so reading `outer_size()` back immediately is not
/// reliable.
fn apply_presentation_size(app: &AppHandle, window: &WebviewWindow) -> (f64, f64) {
    let size = desired_size(&state(app));
    if let Err(error) = window.set_size(LogicalSize::new(size.0, size.1)) {
        tracing::warn!(%error, "failed to sync the floating window size");
    }
    size
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
    let presentation = {
        let mut runtime = state.floating.lock().expect("float mutex");
        // A rebuild always starts from the collapsed strip: the hover card is
        // a hover-only state and the pointer is not over the new window yet.
        runtime.clear_dock_hover();
        runtime.presentation(layout)
    };
    let (width, height) = size_for(&state, presentation, false);

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
    crate::window::win::round_corners(&window, layout::corner_radius(presentation, false));

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
/// Setting a layout always leaves the transient docked presentation, matching
/// the old `apply_float_mode` behavior. A side rail would otherwise expand to
/// the right of the screen edge, so its position is clamped back into the work
/// area.
pub fn set_layout(app: &AppHandle, layout: FloatLayout) {
    let state = state(app);
    let was_side_rail = state
        .floating
        .lock()
        .expect("float mutex")
        .dock_edge()
        .is_some_and(FloatEdge::is_side);
    state.apply_float_layout(layout);
    sync_size(app);
    if was_side_rail {
        if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
            if let Some(work) = work_area(&window) {
                let size = physical_size(desired_size(&state), work.scale_factor);
                clamp_into_work_area(&window, work, size);
            }
        }
    }
    emit_state(app);
    persist_config(app);
}

/// Applies the strip length measured by the webview.
///
/// The webview measures its own content (text widths depend on the platform
/// fonts) and reports the total length along the strip axis: the width of the
/// top strip, the height of a side rail. The controller clamps that value and
/// re-applies the native size; nothing is persisted and the DTO is unchanged,
/// so no event is needed.
pub fn set_docked_length(app: &AppHandle, length: f64) {
    let state = state(app);
    let changed = state
        .floating
        .lock()
        .expect("float mutex")
        .set_docked_length(length);
    if changed {
        sync_size(app);
    }
}

/// Expands or collapses the hover card of a docked window.
///
/// Hovering a docked strip asks for a slightly larger card: the window is
/// anchored to its edge (the coordinate along the edge is preserved), grows
/// away from it and is clamped into the work area. Collapsing restores the
/// strip on that same anchor, so a clamped card does not slide the strip.
///
/// Hover is a transient presentation, not a layout: requests are ignored while
/// the window is not docked, while a side rail waits for its un-dock to settle
/// (the native move loop owns the window), and while the user is dragging. The
/// controller owns the native size; the webview only asks for the state and
/// renders whatever `float://state` reports.
pub fn set_dock_hover(app: &AppHandle, hover: bool) {
    let Some(window) = app.get_webview_window(WINDOW_LABEL) else {
        return;
    };
    let state = state(app);

    let edge = {
        let runtime = state.floating.lock().expect("float mutex");
        match runtime.dock_edge() {
            Some(edge) if !runtime.is_undock_pending() => edge,
            _ => return,
        }
    };
    if drag_in_progress() {
        return;
    }
    if state.floating.lock().expect("float mutex").is_dock_hover() == hover {
        return;
    }
    let Some(work) = work_area(&window) else {
        return;
    };
    let Ok(position) = window.outer_position() else {
        return;
    };

    // The coordinate along the edge must survive the expansion: only the axis
    // that grows away from the edge moves.
    let anchor = match edge {
        FloatEdge::Top => position.x,
        FloatEdge::Left | FloatEdge::Right => position.y,
    };
    let anchor = if hover {
        anchor
    } else {
        state
            .floating
            .lock()
            .expect("float mutex")
            .dock_hover_origin()
            .unwrap_or(anchor)
    };
    let presentation = edge.into();
    let (width, height) = size_for(&state, presentation, hover);

    // Record the new state first: the resize below re-applies the native
    // window region, which reads the same flag for its corner radius.
    {
        let mut runtime = state.floating.lock().expect("float mutex");
        if !runtime.set_dock_hover(hover, Some(anchor)) {
            return;
        }
    }

    if let Err(error) = window.set_size(LogicalSize::new(width, height)) {
        tracing::warn!(%error, "failed to resize the floating window's hover card");
    }
    let target = layout::docked_position(
        edge,
        anchor,
        work.rect,
        physical_size((width, height), work.scale_factor),
    );
    if (target.0, target.1) != (position.x, position.y) {
        let _ = window.set_position(PhysicalPosition::new(target.0, target.1));
    }
    #[cfg(target_os = "windows")]
    crate::window::win::round_corners(&window, layout::corner_radius(presentation, hover));
    emit_state(app);
}

/// Re-applies the native size for the current presentation and usage state.
///
/// Idempotent; called after every quota refresh, after config changes and
/// after mode changes. The size always derives from the canonical state, so
/// the native window can never drift from the DTO the frontend renders.
pub fn sync_size(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        apply_presentation_size(app, &window);
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
        // restore the window, but stop any queued position write or settle.
        let mut runtime = state.floating.lock().expect("float mutex");
        let _ = runtime.take_pending_position();
        runtime.cancel_dock_settle();
        runtime.clear_dock_hover();
        runtime.set_open(false);
        return;
    }

    let (was_open, pending_position) = {
        let mut runtime = state.floating.lock().expect("float mutex");
        let was_open = runtime.is_open();
        let pending_position = runtime.take_pending_position();
        runtime.cancel_dock_settle();
        // The window is going away; a reopen must start from the strip.
        runtime.clear_dock_hover();
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
/// decision in physical coordinates, and applies size/snap on changes.
fn on_moved(app: &AppHandle, position: &PhysicalPosition<i32>) {
    let Some(window) = app.get_webview_window(WINDOW_LABEL) else {
        return;
    };
    let state = state(app);

    // The hover card's position is derived from the strip anchor and the work
    // area; it must never overwrite the persisted (collapsed) position.
    let hover = state.floating.lock().expect("float mutex").is_dock_hover();
    if !hover {
        let mut config = state.config.lock().expect("config mutex");
        config.float_position = Some(FloatPosition {
            x: position.x,
            y: position.y,
        });
    }
    let generation = state.floating.lock().expect("float mutex").begin_move();

    let Some(plan) = dock_plan_at(&state, &window, *position) else {
        schedule_position_persist(app, generation);
        return;
    };

    let (presentation_changed, hover_changed) = {
        let mut runtime = state.floating.lock().expect("float mutex");
        let was_hover = runtime.is_dock_hover();
        let presentation_changed = runtime.apply_dock_plan(plan);
        // Dragging the expanded card along its edge moves the strip with it:
        // remember the new anchor so the collapse lands where the card is.
        if was_hover && runtime.is_dock_hover() && drag_in_progress() {
            if let Some(edge) = plan.edge {
                let anchor = match edge {
                    FloatEdge::Top => position.x,
                    FloatEdge::Left | FloatEdge::Right => position.y,
                };
                runtime.set_dock_hover_origin(anchor);
            }
        }
        (presentation_changed, was_hover != runtime.is_dock_hover())
    };
    if presentation_changed {
        let size = apply_presentation_size(app, &window);
        if let Some(edge) = plan.edge {
            snap_to_edge(&window, edge, size);
        }
        emit_state(app);
    } else if hover_changed {
        // The plan dropped the card (a side rail started to un-dock). Resize
        // back to the strip, but do not snap: the move loop owns the position
        // until the drag settles.
        apply_presentation_size(app, &window);
        emit_state(app);
    }

    if plan.edge.is_some() {
        // Docked windows are re-snapped flush to their edge, and a pending
        // side rail is expanded, only once the drag has settled.
        schedule_dock_settle(app, generation);
    }
    schedule_position_persist(app, generation);
}

/// Resolves the docking decision for the window's current geometry.
///
/// `None` means the monitor work area or the window size could not be read;
/// callers then only persist the position and leave the dock state untouched.
fn dock_plan_at(
    state: &AppState,
    window: &WebviewWindow,
    position: PhysicalPosition<i32>,
) -> Option<DockPlan> {
    let work = work_area(window)?;
    let size = window.outer_size().ok()?;
    let distances = EdgeDistances::new(
        (position.x, position.y),
        (size.width, size.height),
        work.rect,
    );
    let (current, guarded) = {
        let runtime = state.floating.lock().expect("float mutex");
        (runtime.dock_edge(), runtime.is_dock_guarded())
    };
    Some(layout::plan_dock(
        current,
        guarded,
        distances,
        work.scale_factor,
    ))
}

/// Schedules the deferred dock pass for a move.
///
/// The task re-checks the move generation, so only the newest drag can settle.
/// While the mouse button is still down the pass reschedules itself: the native
/// move loop owns the window position until the drag ends.
fn schedule_dock_settle(app: &AppHandle, generation: u64) {
    let state = state(app);
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(DOCK_SETTLE_MS)).await;
            if !state
                .floating
                .lock()
                .expect("float mutex")
                .is_latest_move(generation)
            {
                return;
            }
            if drag_in_progress() {
                continue;
            }
            settle_docked_window(&handle);
            return;
        }
    });
}

/// Re-snaps a docked window, or expands a pending side rail.
fn settle_docked_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window(WINDOW_LABEL) else {
        return;
    };
    let state = state(app);
    let (edge, pending) = {
        let runtime = state.floating.lock().expect("float mutex");
        (runtime.dock_edge(), runtime.is_undock_pending())
    };
    match edge {
        Some(edge) if pending => commit_undock(app, &window, edge),
        Some(edge) => snap_to_edge(&window, edge, desired_size(&state)),
        None => {}
    }
}

/// Expands a side rail that was dragged out of its release band.
///
/// The rail is dropped as-is when the drag ends. If it ended against another
/// edge, that edge docks instead of expanding into the middle of the screen;
/// otherwise the configured layout is restored and clamped into the work area.
/// A fresh un-dock arms the re-dock guard while the card still touches an edge.
fn commit_undock(app: &AppHandle, window: &WebviewWindow, edge: FloatEdge) {
    let state = state(app);
    {
        let runtime = state.floating.lock().expect("float mutex");
        if runtime.dock_edge() != Some(edge) || !runtime.is_undock_pending() {
            // The window re-docked (or a newer move arrived) while this settle
            // task was waking up; the newer decision wins.
            return;
        }
    }

    let (Ok(position), Ok(size)) = (window.outer_position(), window.outer_size()) else {
        return;
    };
    let Some(work) = work_area(window) else {
        return;
    };
    let distances = EdgeDistances::new(
        (position.x, position.y),
        (size.width, size.height),
        work.rect,
    );
    let target_edge = layout::snap_edge(distances, work.scale_factor);
    let layout = layout_of(&state);
    let presentation: FloatPresentation = match target_edge {
        Some(edge) => edge.into(),
        None => layout.into(),
    };
    let (width, height) = size_for(&state, presentation, false);
    // Position against the size the window is *becoming*: the resize below may
    // be applied asynchronously, so `outer_size()` cannot be read back yet.
    let target_size = physical_size((width, height), work.scale_factor);
    let current = (position.x, position.y);
    let target_position = match target_edge {
        Some(edge) => snapped_position(edge, current, work, target_size),
        None => clamped_position(current, work, target_size),
    };
    // A freshly un-docked card that still touches an edge keeps docking
    // disarmed until the user drags it clear (a right-edge rail always expands
    // flush to the edge, which is inside its own snap band).
    let guarded = target_edge.is_none()
        && !EdgeDistances::new(target_position, target_size, work.rect)
            .all_beyond(work.scale_factor);

    // Commit the docking decision first so the native size, the position and
    // the DTO all come from this one decision.
    let plan = match target_edge {
        Some(edge) => DockPlan::docked(edge),
        None => DockPlan::undocked(false),
    };
    state
        .floating
        .lock()
        .expect("float mutex")
        .apply_dock_plan(plan);

    if let Err(error) = window.set_size(LogicalSize::new(width, height)) {
        tracing::warn!(%error, "failed to resize the floating window after un-docking");
    }
    if target_position != current {
        let _ = window.set_position(PhysicalPosition::new(target_position.0, target_position.1));
    }
    state
        .floating
        .lock()
        .expect("float mutex")
        .set_dock_guarded(guarded);
    emit_state(app);
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

/// Monitor work area in physical pixels plus the DPI scale.
#[derive(Debug, Clone, Copy)]
struct WorkArea {
    /// `(left, top, right, bottom)`, taskbar-aware on Windows.
    rect: (i32, i32, i32, i32),
    scale_factor: f64,
}

/// Work area under the window, in physical pixels.
///
/// Windows uses the native work area (taskbar-aware); other platforms use the
/// monitor's bounds, matching the previous top-docking behavior.
fn work_area(window: &WebviewWindow) -> Option<WorkArea> {
    #[cfg(target_os = "windows")]
    let rect = crate::float::win::work_area(window)?;
    #[cfg(not(target_os = "windows"))]
    let rect = {
        let monitor = window.current_monitor().ok().flatten()?;
        let position = monitor.position();
        let size = monitor.size();
        (
            position.x,
            position.y,
            position.x.saturating_add(size.width as i32),
            position.y.saturating_add(size.height as i32),
        )
    };
    Some(WorkArea {
        rect,
        scale_factor: window.scale_factor().unwrap_or(1.0),
    })
}

/// Converts a logical window size into the physical pixels the native window
/// will have on this monitor.
fn physical_size(size: (f64, f64), scale_factor: f64) -> (u32, u32) {
    let scale = scale_factor.max(0.01);
    (
        (size.0 * scale).round().max(1.0) as u32,
        (size.1 * scale).round().max(1.0) as u32,
    )
}

/// Work-area size in physical pixels.
fn work_size(work: WorkArea) -> (u32, u32) {
    (
        (work.rect.2 - work.rect.0).max(1) as u32,
        (work.rect.3 - work.rect.1).max(1) as u32,
    )
}

/// Physical position that snaps a `size` window flush to `edge`.
///
/// Only the axis that owns the edge moves: a top strip keeps its horizontal
/// position, a side rail keeps its vertical position.
fn snapped_position(
    edge: FloatEdge,
    current: (i32, i32),
    work: WorkArea,
    size: (u32, u32),
) -> (i32, i32) {
    let x = match edge {
        FloatEdge::Top => current.0,
        FloatEdge::Left => work.rect.0,
        FloatEdge::Right => work.rect.2 - size.0 as i32,
    };
    let y = match edge {
        FloatEdge::Top => work.rect.1,
        FloatEdge::Left | FloatEdge::Right => current.1,
    };
    (x, y)
}

/// Physical position clamped into the work area for a `size` window.
fn clamped_position(position: (i32, i32), work: WorkArea, size: (u32, u32)) -> (i32, i32) {
    layout::clamp_position(position, (work.rect.0, work.rect.1), work_size(work), size)
}

/// Snaps the floating window flush to a work-area edge.
///
/// `size` is the logical size the window has (or is becoming). A no-op when
/// already flush, so the settle pass cannot start a move feedback loop.
fn snap_to_edge(window: &WebviewWindow, edge: FloatEdge, size: (f64, f64)) {
    let Some(work) = work_area(window) else {
        return;
    };
    let Ok(position) = window.outer_position() else {
        return;
    };
    let target = snapped_position(
        edge,
        (position.x, position.y),
        work,
        physical_size(size, work.scale_factor),
    );
    if target != (position.x, position.y) {
        let _ = window.set_position(PhysicalPosition::new(target.0, target.1));
    }
}

/// Clamps the window back into the work area for a target physical size.
fn clamp_into_work_area(window: &WebviewWindow, work: WorkArea, size: (u32, u32)) {
    let Ok(position) = window.outer_position() else {
        return;
    };
    let target = clamped_position((position.x, position.y), work, size);
    if target != (position.x, position.y) {
        let _ = window.set_position(PhysicalPosition::new(target.0, target.1));
    }
}

/// Whether the user is still dragging the window.
///
/// On Windows the native move loop owns the window until the mouse button is
/// released; the deferred side un-dock must not run before that. Other
/// platforms fall back to the settle delay alone.
fn drag_in_progress() -> bool {
    #[cfg(target_os = "windows")]
    {
        crate::float::win::primary_button_down()
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
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
    let (presentation, dock_hover) = {
        let runtime = state.floating.lock().expect("float mutex");
        (runtime.presentation(layout), runtime.is_dock_hover())
    };
    crate::window::win::round_corners(&window, layout::corner_radius(presentation, dock_hover));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn work() -> WorkArea {
        WorkArea {
            rect: (0, 0, 1920, 1040),
            scale_factor: 1.0,
        }
    }

    #[test]
    fn physical_size_rounds_with_the_monitor_scale() {
        assert_eq!(physical_size((392.0, 280.0), 1.0), (392, 280));
        assert_eq!(physical_size((392.0, 280.0), 1.5), (588, 420));
        assert_eq!(physical_size((24.0, 257.4), 1.0), (24, 257));
        assert_eq!(
            physical_size((0.0, 0.0), 1.0),
            (1, 1),
            "a zero size must never reach the native window"
        );
    }

    #[test]
    fn snapped_position_moves_only_the_owning_axis() {
        let work = work();
        assert_eq!(
            snapped_position(FloatEdge::Top, (300, 12), work, (392, 24)),
            (300, 0)
        );
        assert_eq!(
            snapped_position(FloatEdge::Left, (12, 300), work, (24, 257)),
            (0, 300)
        );
        assert_eq!(
            snapped_position(FloatEdge::Right, (879, 300), work, (24, 257)),
            (1896, 300)
        );
        assert_eq!(
            snapped_position(FloatEdge::Right, (500, 300), work, (392, 280)),
            (1528, 300),
            "the target size decides where a right-edge card starts"
        );
    }

    #[test]
    fn clamped_position_keeps_an_expanded_rail_visible() {
        let work = work();
        // A right rail dropped 30 px from the edge expands flush right.
        assert_eq!(clamped_position((1890, 300), work, (392, 280)), (1528, 300));
        // A rail dropped in the middle keeps its position.
        assert_eq!(clamped_position((800, 300), work, (392, 280)), (800, 300));
        // A rail dragged past the top is pushed back into the work area.
        assert_eq!(clamped_position((800, -40), work, (392, 280)), (800, 0));
    }
}

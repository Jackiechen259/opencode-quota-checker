//! Application-level status snapshot and lifecycle.
//!
//! Both status commands are `async`: Tauri runs synchronous commands on the
//! main thread, and a contended or poisoned mutex in a synchronous snapshot
//! would freeze the webview event loop — exactly the failure mode that made
//! the title bar buttons unresponsive. Async commands run on the Tauri async
//! runtime, so a slow snapshot can only stall its own IPC response, never
//! window dragging, minimize/maximize/close, or other commands.

use crate::state::{AppState, AppStatusDto, BootStatusDto};
use std::sync::Arc;
use tauri::{AppHandle, Manager};

/// Returns the boot-critical subset of the application status.
///
/// Deliberately reads only config-load state and the credential phase (see
/// [`BootStatusDto`]): the main window must be able to boot even when the
/// tray, monitor, floating window or updater state is wedged.
#[tauri::command]
pub async fn get_boot_status(app: AppHandle) -> BootStatusDto {
    let state = app.state::<Arc<AppState>>().inner().clone();
    let started = std::time::Instant::now();
    tracing::debug!("command entered: get_boot_status");
    let dto = state.boot_dto();
    tracing::debug!(
        elapsed_ms = started.elapsed().as_millis() as u64,
        "boot status snapshot complete"
    );
    dto
}

/// Returns the full application status for the initial render.
#[tauri::command]
pub async fn get_app_status(app: AppHandle) -> AppStatusDto {
    let state = app.state::<Arc<AppState>>().inner().clone();
    let started = std::time::Instant::now();
    tracing::debug!("command entered: get_app_status");
    let dto = state.status_dto();
    tracing::debug!(
        elapsed_ms = started.elapsed().as_millis() as u64,
        "status snapshot complete"
    );
    dto
}

/// Terminates the whole application: monitor task, windows, webviews, process.
#[tauri::command]
pub fn quit_app(app: AppHandle) {
    crate::actions::quit(&app);
}

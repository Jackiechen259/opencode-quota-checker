//! Application-level status snapshot and lifecycle.

use crate::state::{AppState, AppStatusDto};
use std::sync::Arc;
use tauri::{AppHandle, State};

/// Returns the full application status for the initial render.
#[tauri::command]
pub fn get_app_status(state: State<'_, Arc<AppState>>) -> AppStatusDto {
    state.inner().status_dto()
}

/// Terminates the whole application: monitor task, windows, webviews, process.
#[tauri::command]
pub fn quit_app(app: AppHandle) {
    crate::actions::quit(&app);
}

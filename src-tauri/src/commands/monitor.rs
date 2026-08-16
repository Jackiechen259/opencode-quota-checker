//! Monitor state commands.

use crate::state::{AppState, MonitorStatusDto};
use std::sync::Arc;
use tauri::{AppHandle, Manager};

/// Returns the current monitoring snapshot.
///
/// Async so the mutex acquisition never runs on the Tauri main thread.
#[tauri::command]
pub async fn get_monitor_status(app: AppHandle) -> MonitorStatusDto {
    let state = app.state::<Arc<AppState>>().inner().clone();
    state.monitor_dto()
}

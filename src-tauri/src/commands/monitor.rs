//! Monitor state commands.

use crate::state::{AppState, MonitorStatusDto};
use std::sync::Arc;
use tauri::State;

/// Returns the current monitoring snapshot.
#[tauri::command]
pub fn get_monitor_status(state: State<'_, Arc<AppState>>) -> MonitorStatusDto {
    state.inner().monitor_dto()
}

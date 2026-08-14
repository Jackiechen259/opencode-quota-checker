//! Updater commands. The state machine lives in Rust; these commands only
//! trigger transitions and return snapshots.

use crate::error::AppError;
use crate::state::{AppState, UpdateStateDto};
use crate::updater;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

/// Returns the current updater snapshot.
#[tauri::command]
pub fn get_update_state(state: State<'_, Arc<AppState>>) -> UpdateStateDto {
    state.inner().update_dto()
}

/// Runs one manifest check (background; events carry the result).
#[tauri::command]
pub fn check_for_update(app: AppHandle) -> Result<(), AppError> {
    tauri::async_runtime::spawn(async move {
        updater::check(&app).await;
    });
    Ok(())
}

/// Starts downloading the available update (background).
#[tauri::command]
pub fn download_update(app: AppHandle) -> Result<(), AppError> {
    tauri::async_runtime::spawn(async move {
        updater::download(&app).await;
    });
    Ok(())
}

/// Installs the verified update. Requires explicit user confirmation.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), AppError> {
    updater::install(&app).await
}

/// Hides the dashboard update banner for this run.
#[tauri::command]
pub fn dismiss_update(app: AppHandle) {
    updater::dismiss(&app);
}

/// Opens the release notes of the available update in the browser.
#[tauri::command]
pub fn open_release_notes(app: AppHandle) -> Result<(), AppError> {
    let state = app.state::<Arc<AppState>>().inner().clone();
    let dto = state.update_dto();
    let Some(info) = dto.available else {
        return Ok(());
    };
    crate::launcher::open_url(&info.release_notes_url).map_err(|detail| {
        AppError::new("browser_launch_failed", "无法打开更新说明。", detail)
    })
}

//! Go usage page commands.
//!
//! The overview is fetched on demand: opening the page calls
//! `get_go_overview`, and the header/page refresh calls `refresh_go_overview`.
//! Both commands return the current snapshot immediately; the result arrives
//! through the `go://overview` event so the IPC call never blocks on three
//! network requests.

use crate::error::AppError;
use crate::go_overview;
use crate::launcher;
use crate::state::{AppState, GoOverviewDto};
use std::sync::Arc;
use tauri::{AppHandle, Manager};

/// Returns the current Go overview and starts a background fetch if stale.
#[tauri::command]
pub async fn get_go_overview(app: AppHandle) -> GoOverviewDto {
    let state = app.state::<Arc<AppState>>().inner().clone();
    go_overview::request(&app, &state, false);
    state.go_overview_dto()
}

/// Forces a fresh Go overview fetch (manual refresh).
#[tauri::command]
pub async fn refresh_go_overview(app: AppHandle) -> GoOverviewDto {
    let state = app.state::<Arc<AppState>>().inner().clone();
    go_overview::request(&app, &state, true);
    state.go_overview_dto()
}

/// Opens the OpenCode Console Go page in the system browser.
#[tauri::command]
pub fn open_go_page() -> Result<(), AppError> {
    launcher::open_url(launcher::GO_URL).map_err(|detail| {
        AppError::new(
            "browser_launch_failed",
            "无法打开浏览器，请手动访问 opencode.ai/console/go。",
            detail,
        )
    })
}

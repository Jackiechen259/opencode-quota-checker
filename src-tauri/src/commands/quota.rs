//! Quota usage commands.
//!
//! The `auth` cookie never crosses the IPC boundary: it is read from the
//! keyring inside the Rust backend (through the blocking credential layer,
//! so a wedged Credential Manager can never stall a refresh) and only ever
//! transmitted as a request header by `opencode-core`.

use crate::credential_task;
use crate::error::AppError;
use crate::monitor;
use crate::state::{AppState, UsageDto};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

/// Returns the latest parsed usage report (never the raw response).
#[tauri::command]
pub fn get_usage(state: State<'_, Arc<AppState>>) -> UsageDto {
    let usage = state.inner().usage.lock().expect("usage mutex");
    UsageDto {
        report: usage.report.clone(),
        loading: usage.loading,
        error: usage.error.clone(),
        last_success_ms: usage.last_success_ms,
    }
}

/// Triggers one fetch/parse/alert cycle through the shared monitor path.
///
/// Returns immediately; results arrive via `quota://updated` / `quota://error`
/// and `monitor://status` events.
#[tauri::command]
pub fn refresh_usage(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<(), AppError> {
    let state = state.inner().clone();
    if !state.configured() {
        return Ok(());
    }
    tauri::async_runtime::spawn(async move {
        monitor::run_once(&app).await;
    });
    Ok(())
}

/// Fetches the raw dashboard HTML for the debug overlay.
///
/// The response may contain server-side data; the frontend shows a warning.
/// The `auth` cookie itself is never part of the returned value.
#[tauri::command]
pub async fn get_raw_dashboard(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<String, AppError> {
    let state = state.inner().clone();
    {
        let mut usage = state.usage.lock().expect("usage mutex");
        if usage.raw_loading {
            return Err(AppError::new(
                "raw_request_in_flight",
                "原始响应正在加载中。",
                "a raw response request is already in flight",
            ));
        }
        usage.raw_loading = true;
    }
    let config = state.config.lock().expect("config mutex").clone();
    let workspace = config.opencode_workspace_id.clone().unwrap_or_default();
    if workspace.trim().is_empty() {
        let mut usage = state.usage.lock().expect("usage mutex");
        usage.raw_loading = false;
        return Err(AppError::from(
            opencode_core::OpenCodeError::CredentialsMissing,
        ));
    }

    let service = state.service.clone();
    let result = async {
        let cookie = credential_task::load_cookie().await?;
        service
            .fetch_raw_dashboard(&workspace, &cookie)
            .await
            .map_err(AppError::from)
    }
    .await;

    let mut usage = state.usage.lock().expect("usage mutex");
    usage.raw_loading = false;
    match result {
        Ok(raw) => {
            usage.raw = Some(raw.clone());
            usage.error = None;
            Ok(raw)
        }
        Err(error) => {
            usage.error = Some(error.clone());
            let _ = app.emit(crate::events::QUOTA_ERROR, &error);
            Err(error)
        }
    }
}

//! Quota usage commands.
//!
//! The API key never crosses the IPC boundary: it is read from the keyring
//! inside the Rust backend (through the blocking credential layer, so a wedged
//! Credential Manager can never stall a refresh) and only ever transmitted as
//! a bearer token by `opencode-core`.

use crate::credential_task;
use crate::error::AppError;
use crate::monitor;
use crate::state::{AppState, UsageDto};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};

/// Returns the latest parsed usage report (never the raw response).
///
/// Async so the mutex acquisition never runs on the Tauri main thread.
#[tauri::command]
pub async fn get_usage(app: AppHandle) -> UsageDto {
    let state = app.state::<Arc<AppState>>().inner().clone();
    state.usage_dto()
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

/// Fetches the raw usage JSON for the debug overlay.
///
/// The response may contain workspace identifiers; the frontend shows a
/// warning. The API key itself is never part of the returned value.
#[tauri::command]
pub async fn get_raw_usage(
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
    let workspace = state
        .config
        .lock()
        .expect("config mutex")
        .opencode_workspace_id
        .clone();
    let service = state.service.clone();

    let result = async {
        let api_key = credential_task::load_api_key().await?;
        service
            .fetch_raw_usage(&api_key, workspace.as_deref())
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
            drop(usage);
            let _ = app.emit(crate::events::QUOTA_ERROR, &error);
            Err(error)
        }
    }
}

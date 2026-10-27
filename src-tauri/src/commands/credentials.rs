//! Credential commands.
//!
//! The OpenCode Console API key is stored in the system keyring by
//! `opencode-core` (`service = opencode-quota-checker`,
//! `account = opencode-api-key`). It is never returned to the frontend and is
//! cleared from React state right after a successful save.
//!
//! Every keyring call goes through `crate::credential_task` (blocking pool +
//! soft timeout): a wedged Credential Manager can never freeze the main
//! thread or the webview event loop.

use crate::credential_task::{self, CredentialCheckResult};
use crate::error::AppError;
use crate::launcher;
use crate::persistence::persist_config;
use crate::state::{AppState, CredentialPhase};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};

/// Reports whether an OpenCode API key is stored in the keyring.
///
/// Runs on the blocking pool with the same soft timeout as every other
/// keyring operation.
#[tauri::command]
pub async fn has_credentials() -> Result<bool, AppError> {
    Ok(credential_task::check_credentials().await == CredentialCheckResult::Available)
}

/// Re-runs the keyring availability check after an error or timeout.
///
/// The check runs in the background; the result arrives via the `app://status`
/// event. The window shell stays fully operable while it is in flight.
#[tauri::command]
pub async fn recheck_credentials(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<(), AppError> {
    let state = state.inner().clone();
    {
        let mut credentials = state.credentials.lock().expect("credential mutex");
        credentials.phase = CredentialPhase::Checking;
        credentials.error = None;
    }
    let _ = app.emit(crate::events::APP_STATUS, state.status_dto());
    tauri::async_runtime::spawn(async move {
        let result = credential_task::check_credentials().await;
        state.apply_credential_check(result);
        let _ = app.emit(crate::events::APP_STATUS, state.status_dto());
    });
    Ok(())
}

/// Saves the optional workspace ID (plain config) and the API key (keyring only).
///
/// The key travels over IPC once, from the password input to this command; it
/// is never stored in the frontend afterwards and never written to logs. The
/// workspace ID is only an override for the `x-opencode-org-id` header — a
/// personal API key already targets the workspace it was created in.
///
/// The key is stored first and then exercised with one real request, so the
/// caller learns immediately whether it actually works: an invalid key or a
/// workspace without Go comes back as an error while the key stays stored, so
/// the user can retry without retyping it.
#[tauri::command]
pub async fn save_credentials(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    workspace_id: Option<String>,
    api_key: String,
) -> Result<(), AppError> {
    let state = state.inner().clone();
    let api_key = api_key.trim().to_owned();
    if api_key.is_empty() {
        return Err(AppError::new(
            "credentials_invalid",
            "API Key 不能为空。",
            "empty OpenCode API key",
        ));
    }
    // The keyring write runs on the blocking pool with a soft timeout; the key
    // is moved into the worker and never logged.
    credential_task::save_api_key(api_key).await?;
    let workspace_id = workspace_id
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    {
        let mut config = state.config.lock().expect("config mutex");
        config.opencode_workspace_id = workspace_id;
    }
    {
        let mut credentials = state.credentials.lock().expect("credential mutex");
        credentials.phase = CredentialPhase::Available;
        credentials.available = true;
        credentials.error = None;
    }
    persist_config(&app);
    state.push_monitor_config();
    let _ = app.emit(crate::events::APP_STATUS, state.status_dto());

    // One probe so a wrong key, a missing Go subscription or a blocked network
    // is reported while the user is still on the credentials page.
    crate::monitor::run_once(&app).await;
    let probe_error = state.usage.lock().expect("usage mutex").error.clone();
    let _ = app.emit(crate::events::APP_STATUS, state.status_dto());
    match probe_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// Re-tests the stored API key with one request and returns any failure.
#[tauri::command]
pub async fn test_connection(app: AppHandle) -> Result<(), AppError> {
    let state = app.state::<Arc<AppState>>().inner().clone();
    if !state.configured() {
        return Err(AppError::from(
            opencode_core::OpenCodeError::CredentialsMissing,
        ));
    }
    crate::monitor::run_once(&app).await;
    let error = state.usage.lock().expect("usage mutex").error.clone();
    match error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// Clears the keyring API key and the persisted workspace ID.
#[tauri::command]
pub async fn clear_credentials(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<(), AppError> {
    let state = state.inner().clone();
    credential_task::clear_api_key().await?;
    {
        let mut config = state.config.lock().expect("config mutex");
        config.opencode_workspace_id = None;
        config.monitor_enabled = false;
    }
    {
        let mut credentials = state.credentials.lock().expect("credential mutex");
        credentials.phase = CredentialPhase::Missing;
        credentials.available = false;
        credentials.error = None;
    }
    {
        let mut usage = state.usage.lock().expect("usage mutex");
        usage.report = None;
        usage.raw = None;
        usage.error = None;
    }
    {
        let mut monitor = state.monitor.lock().expect("monitor mutex");
        monitor.last_alerted.clear();
        monitor.last_fetch_ms = None;
        monitor.error = None;
    }
    state.push_monitor_config();
    persist_config(&app);
    let _ = app.emit(crate::events::APP_STATUS, state.status_dto());
    crate::tray::emit_monitor_state(&app);
    Ok(())
}

/// Opens the OpenCode Console where API keys are created.
#[tauri::command]
pub fn open_console_page() -> Result<(), AppError> {
    launcher::open_url(launcher::CONSOLE_URL).map_err(|detail| {
        AppError::new(
            "browser_launch_failed",
            "无法打开浏览器，请手动访问 opencode.ai/console 创建 API Key。",
            detail,
        )
    })
}

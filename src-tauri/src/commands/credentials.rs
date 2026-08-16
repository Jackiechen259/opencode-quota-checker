//! Credential commands.
//!
//! The auth cookie is stored in the system keyring by `opencode-core`
//! (`service = opencode-quota-checker`, `account = opencode-auth`) — the same
//! entries the archived Iced client wrote, so existing credentials survive
//! the upgrade. The cookie is never returned to the frontend and is cleared
//! from React state right after a successful save.
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
use tauri::{AppHandle, Emitter, State};

/// Reports whether an OpenCode auth cookie is stored in the keyring.
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

/// Saves the workspace ID (plain config) and the auth cookie (keyring only).
///
/// The cookie travels over IPC once, from the password input to this command;
/// it is never stored in the frontend afterwards and never written to logs.
#[tauri::command]
pub async fn save_credentials(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    workspace_id: String,
    auth_cookie: String,
) -> Result<(), AppError> {
    let state = state.inner().clone();
    let workspace_id = workspace_id.trim().to_owned();
    if workspace_id.is_empty() {
        return Err(AppError::new(
            "credentials_invalid",
            "Workspace ID 不能为空。",
            "empty OpenCode workspace id",
        ));
    }
    // The keyring write runs on the blocking pool with a soft timeout; the
    // cookie is moved into the worker and never logged.
    credential_task::save_cookie(auth_cookie).await?;
    {
        let mut config = state.config.lock().expect("config mutex");
        config.opencode_workspace_id = Some(workspace_id);
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
    Ok(())
}

/// Clears the keyring cookie and the persisted workspace ID.
#[tauri::command]
pub async fn clear_credentials(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<(), AppError> {
    let state = state.inner().clone();
    credential_task::clear_cookie().await?;
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

/// Opens the OpenCode login page in the system browser.
#[tauri::command]
pub fn open_login_page() -> Result<(), AppError> {
    launcher::open_url(launcher::LOGIN_URL).map_err(|detail| {
        AppError::new(
            "browser_launch_failed",
            "无法打开浏览器，请手动访问 opencode.ai/auth 完成登录。",
            detail,
        )
    })
}

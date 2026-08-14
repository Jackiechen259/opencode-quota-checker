//! Configuration read/write commands.
//!
//! Rust is the final validation gate: `AppConfig::validate` runs on every
//! write (and every load). Frontend validation is only UX.

use crate::config::{AppConfig, ConfigStore};
use crate::error::AppError;
use crate::persistence::persist_config;
use crate::state::AppState;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

/// Returns the current validated configuration.
#[tauri::command]
pub fn get_config(state: State<'_, Arc<AppState>>) -> AppConfig {
    state.inner().config.lock().expect("config mutex").clone()
}

/// Validates, persists, and applies a configuration update.
///
/// The returned config is the canonical saved value; callers should replace
/// their local copy with it.
#[tauri::command]
pub async fn save_config(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    config: AppConfig,
) -> Result<AppConfig, AppError> {
    let state = state.inner().clone();
    let validated = config.validate()?;

    let store = ConfigStore::discover()?;
    let canonical = validated.clone();
    let write = tauri::async_runtime::spawn_blocking(move || store.save(&canonical))
        .await
        .map_err(|error| {
            AppError::new(
                "config_write_failed",
                "无法保存配置。",
                error.to_string(),
            )
        })?;
    write.map_err(AppError::from)?;

    state.apply_config(validated.clone());
    let _ = app.emit(crate::events::APP_STATUS, state.status_dto());
    crate::tray::emit_monitor_state(&app);
    Ok(validated)
}

/// Toggles background monitoring on/off (persisted).
#[tauri::command]
pub fn set_monitor(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    enabled: bool,
) -> Result<(), AppError> {
    let state = state.inner().clone();
    {
        let mut config = state.config.lock().expect("config mutex");
        config.monitor_enabled = enabled;
    }
    state.push_monitor_config();
    persist_config(&app);
    crate::tray::emit_monitor_state(&app);
    Ok(())
}

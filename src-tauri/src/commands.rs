use crate::client;
use crate::credential;
use crate::monitor::{Monitor, MonitorStatus, Thresholds};
use std::sync::Mutex;
use tauri::State;

pub struct AppState {
    pub monitor: Monitor,
}

pub struct SafeState(pub Mutex<AppState>);

#[tauri::command]
pub fn set_credentials(ak: String, sk: String) -> Result<(), String> {
    credential::save(&ak, &sk).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn has_credentials() -> bool {
    credential::has()
}

#[tauri::command]
pub fn clear_credentials() -> Result<(), String> {
    credential::clear().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn fetch_usage() -> Result<crate::models::UsageReport, String> {
    let (ak, sk) = credential::load().map_err(|e| e.to_string())?;
    client::fetch_report(&ak, &sk)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn fetch_usage_raw() -> Result<String, String> {
    let (ak, sk) = credential::load().map_err(|e| e.to_string())?;
    client::fetch_afp_usage(&ak, &sk)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_monitor(
    app: tauri::AppHandle,
    state: State<'_, SafeState>,
    interval_sec: u64,
    thresholds: Thresholds,
) -> Result<(), String> {
    let (ak, sk) = credential::load().map_err(|e| e.to_string())?;
    let monitor = {
        let inner = state.0.lock().map_err(|e| e.to_string())?;
        inner.monitor.clone()
    };
    monitor.start(app, ak, sk, interval_sec, thresholds).await
}

#[tauri::command]
pub async fn stop_monitor(state: State<'_, SafeState>) -> Result<(), String> {
    let monitor = {
        let inner = state.0.lock().map_err(|e| e.to_string())?;
        inner.monitor.clone()
    };
    monitor.stop().await
}

#[tauri::command]
pub async fn get_monitor_status(state: State<'_, SafeState>) -> Result<MonitorStatus, String> {
    let monitor = {
        let inner = state.0.lock().map_err(|e| e.to_string())?;
        inner.monitor.clone()
    };
    let status = monitor.status().await;
    Ok(status)
}

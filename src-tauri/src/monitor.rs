//! Background quota monitor.
//!
//! The monitor is a long-lived tokio task owned by the Rust backend, not by
//! any window or webview. It keeps polling while the main window is hidden to
//! the tray, evaluates thresholds with `opencode-core`, deduplicates alerts
//! per subscription cycle, delivers notifications, and emits `quota://*` /
//! `monitor://status` events. The React frontend only listens.

use crate::credential_task;
use crate::error::AppError;
use crate::events;
use crate::state::{AppState, MonitorConfig};
use opencode_core::{evaluate_alerts, QuotaService};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

/// Spawns the monitor loop. The task lives for the whole application run and
/// is driven by the `MonitorConfig` watch channel (`None` = disabled).
pub fn spawn(app: AppHandle, mut config_rx: tokio::sync::watch::Receiver<Option<MonitorConfig>>) {
    tauri::async_runtime::spawn(async move {
        loop {
            let config = *config_rx.borrow();
            match config {
                None => {
                    // Disabled: wait for the next configuration change.
                    if config_rx.changed().await.is_err() {
                        return;
                    }
                    continue;
                }
                Some(MonitorConfig { interval_secs }) => {
                    tokio::select! {
                        _ = tokio::time::sleep(std::time::Duration::from_secs(interval_secs)) => {
                            run_once(&app).await;
                        }
                        _ = config_rx.changed() => {
                            // Interval or enablement changed; re-read the config.
                            continue;
                        }
                    }
                }
            }
        }
    });
}

/// Fetches, parses, alerts, and broadcasts one quota cycle.
///
/// Shared by the monitor task and the manual `refresh_usage` command so both
/// paths behave identically (including alert evaluation).
pub async fn run_once(app: &AppHandle) {
    let state = app.state::<Arc<AppState>>().inner().clone();

    if !state.configured() {
        return;
    }
    {
        let mut usage = state.usage.lock().expect("usage mutex");
        if usage.loading {
            return;
        }
        usage.loading = true;
        usage.error = None;
    }
    {
        let mut monitor = state.monitor.lock().expect("monitor mutex");
        monitor.loading = true;
    }
    emit_monitor_status(app);
    // The loading state may change the ideal Full height (skeleton before
    // the first report); keep the native window in sync with the DTO.
    crate::window::float_window::sync_float_size(app);

    let config = state.config.lock().expect("config mutex").clone();
    let workspace = config.opencode_workspace_id.clone().unwrap_or_default();
    let service = state.service.clone();
    let now_ms = chrono::Utc::now().timestamp_millis();

    let result = async {
        let cookie = credential_task::load_cookie().await?;
        service
            .fetch_quota(&workspace, &cookie)
            .await
            .map_err(AppError::from)
    }
    .await;

    match result {
        Ok(report) => {
            tracing::info!(windows = report.windows.len(), "quota refresh succeeded");
            let decisions = if config.monitor_enabled {
                let mut monitor = state.monitor.lock().expect("monitor mutex");
                let evaluation =
                    evaluate_alerts(&report, &config.thresholds, &monitor.last_alerted);
                monitor.last_alerted = evaluation.next_alerted;
                evaluation.decisions
            } else {
                Vec::new()
            };
            if !decisions.is_empty() {
                crate::notifications::deliver(app, decisions).await;
            }
            {
                let mut usage = state.usage.lock().expect("usage mutex");
                usage.report = Some(report.clone());
                usage.error = None;
                usage.loading = false;
                usage.last_success_ms = Some(now_ms);
            }
            {
                let mut monitor = state.monitor.lock().expect("monitor mutex");
                monitor.loading = false;
                monitor.error = None;
                monitor.last_fetch_ms = Some(now_ms);
            }
            // Quota count changes the ideal Full height; re-sync the window.
            crate::window::float_window::sync_float_size(app);
            let _ = app.emit(events::QUOTA_UPDATED, &report);
        }
        Err(error) => {
            tracing::warn!(%error, "quota refresh failed");
            {
                let mut usage = state.usage.lock().expect("usage mutex");
                usage.loading = false;
                usage.error = Some(error.clone());
            }
            {
                let mut monitor = state.monitor.lock().expect("monitor mutex");
                monitor.loading = false;
                monitor.error = Some(error.clone());
            }
            crate::window::float_window::sync_float_size(app);
            let _ = app.emit(events::QUOTA_ERROR, &error);
        }
    }
    emit_monitor_status(app);
}

fn emit_monitor_status(app: &AppHandle) {
    let state = app.state::<Arc<AppState>>().inner().clone();
    let _ = app.emit(events::MONITOR_STATUS, state.monitor_dto());
}

/// Convenience for commands: builds a `QuotaService` once.
pub fn service() -> Result<QuotaService, AppError> {
    QuotaService::new().map_err(AppError::from)
}

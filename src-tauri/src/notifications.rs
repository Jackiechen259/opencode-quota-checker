//! Delivering threshold decisions through the system notification service.
//!
//! Uses `tauri-plugin-notification`; delivery errors are recorded in the
//! monitor state and surfaced as a `monitor://status` event, never thrown.

use crate::error::AppError;
use crate::state::AppState;
use opencode_core::AlertDecision;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

/// Delivers alert decisions, deduplicated upstream by `evaluate_alerts`.
pub async fn deliver(app: &AppHandle, decisions: Vec<AlertDecision>) {
    for decision in decisions {
        let result = app
            .notification()
            .builder()
            .title(&decision.title)
            .body(&decision.body)
            .show();
        if let Err(error) = result {
            tracing::warn!(%error, "desktop notification failed");
            let state = app.state::<Arc<AppState>>().inner().clone();
            let mut monitor = state.monitor.lock().expect("monitor mutex");
            monitor.notification_error = Some(AppError::new(
                "notification_failed",
                "系统通知发送失败，请检查系统通知设置。",
                error.to_string(),
            ));
            drop(monitor);
            let _ = app.emit(crate::events::MONITOR_STATUS, state.monitor_dto());
        }
    }
}

/// Requests notification permission where the platform requires it (macOS).
pub fn request_permission(app: &AppHandle) {
    let permission = app.notification().permission_state();
    match permission {
        Ok(tauri_plugin_notification::PermissionState::Prompt) => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = app.notification().request_permission() {
                    tracing::warn!(%error, "notification permission request failed");
                }
            });
        }
        Ok(_) => {}
        Err(error) => tracing::warn!(%error, "cannot query notification permission"),
    }
}

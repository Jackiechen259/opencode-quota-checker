//! Application updater driven by `tauri-plugin-updater`.
//!
//! The whole state machine lives in Rust (`AppState::updater`); the frontend
//! only renders snapshots from `update://state` events and confirms the final
//! install. Flow: check → download (auto when enabled) → user confirmation →
//! install. The old custom manifest updater is replaced by the Tauri updater;
//! the legacy `update.json` bridge for Iced clients is published separately
//! by the release workflow.

use crate::error::{AppError, UpdateError};
use crate::events;
use crate::state::{AppState, UpdateStatus};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

/// How often the periodic background check runs (matches the archived client).
const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
/// Progress events are throttled to avoid flooding the frontend.
const PROGRESS_MIN_INTERVAL: Duration = Duration::from_millis(200);

/// Spawns the periodic background check loop. The startup check is triggered
/// separately after the config has been loaded.
pub fn spawn_auto_check(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(CHECK_INTERVAL).await;
            let state = app.state::<Arc<AppState>>().inner().clone();
            let enabled = state
                .config
                .lock()
                .expect("config mutex")
                .update_checks_enabled;
            if enabled {
                check(&app).await;
            }
        }
    });
}

/// Runs one manifest check and applies the result to the state machine.
pub async fn check(app: &AppHandle) {
    let state = app.state::<Arc<AppState>>().inner().clone();
    {
        let mut updater = state.updater.lock().expect("updater mutex");
        if updater.status.busy() {
            return;
        }
        updater.status = UpdateStatus::Checking;
        updater.error = None;
    }
    emit(app);

    let result = if cfg!(debug_assertions) {
        Err(UpdateError::Check(
            "update checks require a packaged build".to_owned(),
        ))
    } else {
        match app.updater() {
            Ok(updater) => updater
                .check()
                .await
                .map_err(|error| UpdateError::Check(error.to_string())),
            Err(error) => Err(UpdateError::Check(error.to_string())),
        }
    };

    // Apply the outcome with the guard strictly scoped, so no `MutexGuard`
    // can ever be held across the `download` await below or across another
    // lock acquisition (see the lock-ordering rules on `AppState`).
    let available = {
        let mut updater = state.updater.lock().expect("updater mutex");
        updater.last_checked_ms = Some(chrono::Utc::now().timestamp_millis());
        match result {
            Ok(Some(update)) => {
                updater.available = Some(update);
                updater.status = UpdateStatus::Available;
                true
            }
            Ok(None) => {
                updater.status = UpdateStatus::UpToDate;
                updater.available = None;
                updater.downloaded = None;
                updater.downloaded_version = None;
                updater.error = None;
                false
            }
            Err(error) => {
                updater.status = UpdateStatus::Error;
                updater.error = Some(error.into());
                false
            }
        }
    };
    emit(app);
    if available {
        let auto_download = state
            .config
            .lock()
            .expect("config mutex")
            .auto_download_updates;
        if auto_download {
            download(app).await;
        }
    }
}

/// Downloads the available update with throttled progress events.
///
/// The plugin verifies the package signature before returning the bytes.
pub async fn download(app: &AppHandle) {
    let state = app.state::<Arc<AppState>>().inner().clone();
    let (update, version) = {
        let mut updater = state.updater.lock().expect("updater mutex");
        if updater.status.busy() {
            return;
        }
        match updater.available.clone() {
            Some(update) => {
                let version = update.version.clone();
                updater.status = UpdateStatus::Downloading;
                updater.error = None;
                updater.progress = None;
                (update, version)
            }
            None => return,
        }
    };
    emit(app);

    let throttle = Arc::new(Mutex::new(Option::<Instant>::None));
    let progress_app = app.clone();
    let progress_state = state.clone();
    let result = update
        .download(
            move |chunk_len, total| {
                // The updater and throttle guards are never held together:
                // each is scoped to its own block (lock-ordering rules).
                {
                    let mut updater = progress_state.updater.lock().expect("updater mutex");
                    let downloaded = updater
                        .progress
                        .map_or(0, |(downloaded, _)| downloaded)
                        .saturating_add(chunk_len as u64);
                    updater.progress = Some((downloaded, total));
                }
                let should_emit = {
                    let mut last = throttle.lock().expect("progress throttle mutex");
                    let now = Instant::now();
                    let due = last.is_none_or(|previous| {
                        now.duration_since(previous) >= PROGRESS_MIN_INTERVAL
                    });
                    if due {
                        *last = Some(now);
                    }
                    due
                };
                if should_emit {
                    let _ = progress_app.emit(events::UPDATE_STATE, progress_state.update_dto());
                }
            },
            || {},
        )
        .await;

    let mut updater = state.updater.lock().expect("updater mutex");
    match result {
        Ok(bytes) => {
            updater.downloaded = Some(bytes);
            updater.downloaded_version = Some(version);
            updater.status = UpdateStatus::ReadyToInstall;
            updater.error = None;
            drop(updater);
            emit(app);
        }
        Err(error) => {
            updater.status = UpdateStatus::Error;
            updater.error = Some(UpdateError::Download(error.to_string()).into());
            drop(updater);
            emit(app);
        }
    }
}

/// Installs the verified update after user confirmation.
///
/// Windows/Linux AppImage installers replace the running binary, so the app
/// exits right after the installer starts; macOS opens the DMG and keeps
/// running.
pub async fn install(app: &AppHandle) -> Result<(), AppError> {
    let state = app.state::<Arc<AppState>>().inner().clone();
    let (update, bytes) = {
        let mut updater = state.updater.lock().expect("updater mutex");
        match (updater.available.clone(), updater.downloaded.take()) {
            (Some(update), Some(bytes)) => {
                updater.status = UpdateStatus::Installing;
                updater.error = None;
                (update, bytes)
            }
            _ => {
                return Err(AppError::new(
                    "update_not_ready",
                    "更新包尚未就绪，请先下载更新。",
                    "no downloaded update",
                ))
            }
        }
    };
    emit(app);

    match update.install(bytes) {
        Ok(()) => {
            tracing::info!("update installer launched");
            #[cfg(any(target_os = "windows", target_os = "linux"))]
            app.exit(0);
            Ok(())
        }
        Err(error) => {
            let error = AppError::from(UpdateError::Install(error.to_string()));
            let mut updater = state.updater.lock().expect("updater mutex");
            updater.status = UpdateStatus::Error;
            updater.error = Some(error.clone());
            drop(updater);
            emit(app);
            Err(error)
        }
    }
}

/// Marks the dashboard update banner as dismissed for this run.
pub fn dismiss(app: &AppHandle) {
    let state = app.state::<Arc<AppState>>().inner().clone();
    state
        .updater
        .lock()
        .expect("updater mutex")
        .banner_dismissed = true;
    emit(app);
}

fn emit(app: &AppHandle) {
    let state = app.state::<Arc<AppState>>().inner().clone();
    let _ = app.emit(events::UPDATE_STATE, state.update_dto());
}

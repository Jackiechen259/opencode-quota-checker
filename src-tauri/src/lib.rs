//! Tauri application assembly: plugins, shared state, tray, monitor,
//! windows, and the IPC command surface.

mod actions;
mod commands;
// `credential_task`, `state`, `config`, `window` and `error` are public so
// the integration tests in `src-tauri/tests/` can exercise the shared state
// machines. The crate is a desktop binary; nothing else consumes this API
// surface.
pub mod config;
pub mod credential_task;
pub mod error;
mod events;
mod icons;
mod launcher;
mod monitor;
mod notifications;
mod persistence;
pub mod state;
mod tray;
mod updater;
pub mod window;

use crate::config::{AppConfig, ConfigStore};
use crate::state::AppState;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::sync::watch;

/// Application entry point invoked by `main.rs`.
pub fn run() {
    init_logging();

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            setup_app(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::get_app_status,
            commands::app::quit_app,
            commands::quota::get_usage,
            commands::quota::refresh_usage,
            commands::quota::get_raw_dashboard,
            commands::config::get_config,
            commands::config::save_config,
            commands::config::set_monitor,
            commands::credentials::has_credentials,
            commands::credentials::recheck_credentials,
            commands::credentials::save_credentials,
            commands::credentials::clear_credentials,
            commands::credentials::open_login_page,
            commands::monitor::get_monitor_status,
            commands::float::get_float_state,
            commands::float::open_float_window,
            commands::float::close_float_window,
            commands::float::set_float_mode,
            commands::update::get_update_state,
            commands::update::check_for_update,
            commands::update::download_update,
            commands::update::install_update,
            commands::update::dismiss_update,
            commands::update::open_release_notes,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the OpenCode Quota Checker");
}

fn init_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "opencode_quota_checker_lib=info,opencode_core=info".into()),
        )
        .init();
}

/// Loads the shared state and starts every background service.
fn setup_app(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Shared state: config, credentials, quota service.
    let (monitor_tx, monitor_rx) = watch::channel(None);
    let state = Arc::new(AppState::new(monitor::service()?, monitor_tx));

    let (config, config_error) =
        match ConfigStore::discover().and_then(|store| store.load_or_default()) {
            Ok(config) => (config, None),
            Err(error) => {
                tracing::error!(%error, "failed to load configuration; using defaults");
                (AppConfig::default(), Some(error.into()))
            }
        };
    state.apply_config(config);

    // The keyring availability check runs as a background task on the
    // blocking pool with a soft timeout — never on the setup thread. The
    // window and its shell appear immediately; a wedged Credential Manager
    // can delay only the credential result, never the UI.
    {
        let state = state.clone();
        let handle = app.handle().clone();
        tauri::async_runtime::spawn(async move {
            let result = credential_task::check_credentials().await;
            state.apply_credential_check(result);
            let _ = handle.emit(events::APP_STATUS, state.status_dto());
        });
    }
    *state.config_error.write().expect("config error rwlock") = config_error;
    app.manage(state.clone());
    tracing::info!(
        configured = state.configured(),
        "application state initialized"
    );

    // 2. System tray (best effort; close behavior falls back to Exit).
    match tray::init(app.handle()) {
        Ok(handle) => *state.tray.lock().expect("tray mutex") = Some(handle),
        Err(error) => {
            tracing::error!(%error, "tray initialization failed; close behavior is Exit");
            *state.tray_error.write().expect("tray error rwlock") = Some(error);
        }
    }

    // 3. Notifications (permission prompt on macOS).
    notifications::request_permission(app.handle());

    // 4. Background monitor.
    monitor::spawn(app.handle().clone(), monitor_rx);
    state.push_monitor_config();

    // 5. Window behavior: the caption × and Alt+F4 always terminate the
    // process. Hiding to the tray is a separate, explicit action (header
    // menu / tray menu); closing and hiding never share a path.
    if let Some(main_window) = app.get_webview_window("main") {
        let handle = app.handle().clone();
        main_window.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                tracing::info!("main window close request; exiting");
                match crate::actions::main_close_action() {
                    crate::actions::MainCloseAction::Exit => handle.exit(0),
                    // Unreachable by design: closing never hides. The arm
                    // exists so the close policy stays a single decision.
                    crate::actions::MainCloseAction::HideToTray => {}
                }
            }
        });
    }

    // 6. Restore the floating window when configured.
    if state.config.lock().expect("config mutex").float_open {
        if let Err(error) = window::float_window::open(app.handle(), &state) {
            tracing::error!(%error, "failed to restore the floating window");
        }
    }

    // 7. Initial quota refresh when configured and monitoring is enabled.
    if state.configured() && state.config.lock().expect("config mutex").monitor_enabled {
        let handle = app.handle().clone();
        tauri::async_runtime::spawn(async move {
            monitor::run_once(&handle).await;
        });
    }

    // 8. Updater: periodic checks plus a startup check when enabled.
    updater::spawn_auto_check(app.handle().clone());
    if state
        .config
        .lock()
        .expect("config mutex")
        .update_checks_enabled
    {
        let handle = app.handle().clone();
        tauri::async_runtime::spawn(async move {
            updater::check(&handle).await;
        });
    }

    Ok(())
}

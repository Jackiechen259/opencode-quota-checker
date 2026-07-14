pub mod client;
pub mod commands;
pub mod credential;
pub mod models;
pub mod monitor;
pub mod signing;

use commands::SafeState;
use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(SafeState(Mutex::new(commands::AppState {
            monitor: monitor::Monitor::new(),
        })))
        .invoke_handler(tauri::generate_handler![
            commands::set_credentials,
            commands::has_credentials,
            commands::clear_credentials,
            commands::fetch_usage,
            commands::fetch_usage_raw,
            commands::start_monitor,
            commands::stop_monitor,
            commands::get_monitor_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

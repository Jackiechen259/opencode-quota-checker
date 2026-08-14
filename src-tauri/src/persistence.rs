//! Persisting the shared config to disk.

use crate::config::ConfigStore;
use std::sync::Arc;
use tauri::Manager;

/// Writes the current in-memory config atomically to the standard location.
///
/// Silent by design: persistence failures are logged, never surfaced as a
/// blocking error, because the in-memory state remains authoritative for the
/// current run (matches the archived Iced client).
pub fn persist_config(app: &tauri::AppHandle) {
    let state = app
        .state::<Arc<crate::state::AppState>>()
        .inner()
        .clone();
    persist_config_state(&state);
}

/// Writes the current in-memory config for an already-resolved state handle.
pub fn persist_config_state(state: &crate::state::AppState) {
    let config = state.config.lock().expect("config mutex").clone();
    let result = ConfigStore::discover().and_then(|store| store.save(&config));
    if let Err(error) = result {
        tracing::error!(%error, "failed to persist configuration");
    }
}

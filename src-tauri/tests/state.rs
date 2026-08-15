//! Integration tests for the shared state machines.
//!
//! These live outside the lib on purpose: the lib's test harness does not
//! receive the Common-Controls v6 manifest that `build.rs` attaches to test
//! targets, and constructing `AppState` pulls in `muda` (tray menu) sections
//! that import comctl32 v6 entry points. Integration tests get the manifest
//! and therefore link cleanly on Windows.

use opencode_core::QuotaService;
use opencode_quota_checker_lib::config::{AppConfig, FloatMode};
use opencode_quota_checker_lib::credential_task::CredentialCheckResult;
use opencode_quota_checker_lib::error::AppError;
use opencode_quota_checker_lib::state::{AppState, CredentialPhase, UpdateStatus};
use std::sync::Arc;
use tokio::sync::watch;

fn state() -> Arc<AppState> {
    let (tx, _rx) = watch::channel(None);
    Arc::new(AppState::new(
        QuotaService::new().expect("quota service"),
        tx,
    ))
}

#[test]
fn busy_statuses_block_new_updates() {
    assert!(UpdateStatus::Checking.busy());
    assert!(UpdateStatus::Downloading.busy());
    assert!(UpdateStatus::Installing.busy());
    assert!(!UpdateStatus::Idle.busy());
    assert!(!UpdateStatus::Available.busy());
    assert!(!UpdateStatus::ReadyToInstall.busy());
    assert!(!UpdateStatus::UpToDate.busy());
    assert!(!UpdateStatus::Error.busy());
}

#[test]
fn update_dto_never_exposes_package_bytes() {
    let state = state();
    {
        let mut updater = state.updater.lock().expect("updater mutex");
        updater.status = UpdateStatus::ReadyToInstall;
        updater.downloaded = Some(vec![1, 2, 3, 4]);
        updater.downloaded_version = Some("9.9.9".to_owned());
    }
    let dto = state.update_dto();
    assert_eq!(dto.status, UpdateStatus::ReadyToInstall);
    assert_eq!(dto.downloaded_version.as_deref(), Some("9.9.9"));
    let json = serde_json::to_string(&dto).expect("dto serializes");
    // The package bytes and the `downloaded` container must never cross the
    // IPC boundary; only the version label is exposed.
    assert!(!json.contains("[1,2,3,4]"));
    assert!(!json.contains("\"downloaded\":"));
    assert!(!json.contains("\"bytes\":"));
}

#[test]
fn monitor_dto_reflects_state() {
    let state = state();
    {
        let mut monitor = state.monitor.lock().expect("monitor mutex");
        monitor.enabled = true;
        monitor.interval_secs = 60;
        monitor.last_fetch_ms = Some(1_234);
    }
    let dto = state.monitor_dto();
    assert!(dto.enabled);
    assert_eq!(dto.interval_secs, 60);
    assert_eq!(dto.last_fetch_ms, Some(1_234));
}

#[test]
fn float_dto_marks_top_docked() {
    let state = state();
    state.floating.lock().expect("float mutex").top_docked = true;
    let dto = state.float_dto();
    assert!(dto.top_docked);
    assert_eq!(dto.mode, FloatMode::Full);
}

#[test]
fn apply_config_drives_monitor_channel() {
    let state = state();
    {
        let mut credentials = state.credentials.lock().expect("credential mutex");
        credentials.available = true;
    }
    let mut rx = state.monitor_tx.subscribe();
    // `apply_config` replaces the whole config, so the workspace must be part
    // of the applied value (like a real `save_config` round-trip).
    let config = AppConfig {
        monitor_enabled: true,
        monitor_interval_secs: 120,
        opencode_workspace_id: Some("ws-test".to_owned()),
        ..AppConfig::default()
    };
    state.apply_config(config);
    let payload = *rx.borrow_and_update();
    assert_eq!(
        payload.map(|config| config.interval_secs),
        Some(120),
        "monitor config is pushed to the background task"
    );
}

#[test]
fn monitor_config_is_none_when_not_configured() {
    let state = state();
    let mut rx = state.monitor_tx.subscribe();
    state.apply_config(AppConfig::default());
    let payload = *rx.borrow_and_update();
    // No keyring credential and no workspace: the monitor must stay off.
    assert!(payload.is_none());
}

#[test]
fn status_dto_carries_version_and_configured_flag() {
    let state = state();
    let dto = state.status_dto();
    assert_eq!(dto.version, env!("CARGO_PKG_VERSION"));
    assert!(!dto.configured);
    assert!(!dto.tray_available);
}

#[test]
fn credential_check_never_leaves_checking() {
    let state = state();
    let results = [
        CredentialCheckResult::Available,
        CredentialCheckResult::Missing,
        CredentialCheckResult::Error(AppError::new(
            "keyring_error",
            "无法访问系统钥匙串。",
            "keyring failed",
        )),
        CredentialCheckResult::Timeout,
    ];
    for result in results {
        state.apply_credential_check(result);
        let credentials = state.credentials.lock().expect("credential mutex");
        assert_ne!(
            credentials.phase,
            CredentialPhase::Checking,
            "every terminal credential outcome must leave the Checking phase"
        );
    }
}

#[test]
fn credential_missing_is_not_available() {
    let state = state();
    state.apply_credential_check(CredentialCheckResult::Missing);
    let credentials = state.credentials.lock().expect("credential mutex");
    assert_eq!(credentials.phase, CredentialPhase::Missing);
    assert!(!credentials.available);
    assert!(credentials.error.is_none());
}

#[test]
fn credential_timeout_is_recoverable() {
    let state = state();
    state.apply_credential_check(CredentialCheckResult::Timeout);
    let credentials = state.credentials.lock().expect("credential mutex");
    assert_eq!(credentials.phase, CredentialPhase::Timeout);
    assert!(!credentials.available);
    let error = credentials.error.as_ref().expect("timeout error is set");
    assert!(
        error.user.contains("超时"),
        "timeout must produce a user-facing message, got: {}",
        error.user
    );
}

#[test]
fn credential_available_enables_configured() {
    let state = state();
    state.apply_credential_check(CredentialCheckResult::Available);
    {
        let credentials = state.credentials.lock().expect("credential mutex");
        assert_eq!(credentials.phase, CredentialPhase::Available);
        assert!(credentials.available);
    }
    {
        let mut config = state.config.lock().expect("config mutex");
        config.opencode_workspace_id = Some("ws-test".to_owned());
    }
    assert!(state.configured());
}

#[test]
fn status_dto_serializes_credential_phase() {
    let state = state();
    state.apply_credential_check(CredentialCheckResult::Timeout);
    let json = serde_json::to_string(&state.status_dto()).expect("dto serializes");
    assert!(
        json.contains("\"phase\":\"timeout\""),
        "the credential phase must cross the IPC boundary, got: {json}"
    );
}

//! IPC DTO JSON contract regression tests.
//!
//! The Rust DTOs serialize with `#[serde(rename_all = "camelCase")]` and the
//! frontend types in `src/types/models.ts` mirror the exact JSON keys. This
//! suite pins that contract on the Rust side: if a rename rule ever changes
//! (or a field is added without the frontend counterpart), CI fails here
//! instead of silently producing `undefined` fields that look like a hung
//! app in the UI (the "正在检查系统钥匙串…" freeze was exactly that).

use opencode_core::QuotaService;
use opencode_quota_checker_lib::credential_task::CredentialCheckResult;
use opencode_quota_checker_lib::state::{AppState, UpdateStatus};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::watch;

fn state() -> Arc<AppState> {
    let (tx, _rx) = watch::channel(None);
    Arc::new(AppState::new(
        QuotaService::new().expect("quota service"),
        tx,
    ))
}

fn assert_has(json: &Value, field: &str) {
    assert!(
        json.get(field).is_some(),
        "serialized DTO must contain camelCase field `{field}`, got: {json}"
    );
}

fn assert_lacks(json: &Value, field: &str) {
    assert!(
        json.get(field).is_none(),
        "serialized DTO must NOT contain snake_case field `{field}`, got: {json}"
    );
}

#[test]
fn app_status_dto_uses_camel_case_contract() {
    let state = state();
    state.apply_credential_check(CredentialCheckResult::Missing);
    {
        let mut monitor = state.monitor.lock().expect("monitor mutex");
        monitor.last_fetch_ms = Some(1_234);
    }
    {
        let mut updater = state.updater.lock().expect("updater mutex");
        updater.status = UpdateStatus::ReadyToInstall;
        updater.downloaded_version = Some("9.9.9".to_owned());
        updater.last_checked_ms = Some(5_678);
        updater.banner_dismissed = true;
    }
    {
        let mut floating = state.floating.lock().expect("float mutex");
        floating.top_docked = true;
    }
    let json = serde_json::to_value(state.status_dto()).expect("dto serializes");

    assert_has(&json, "configLoaded");
    assert_has(&json, "configError");
    assert_has(&json, "trayAvailable");
    assert_has(&json, "trayError");
    assert_has(&json, "monitor");
    assert_has(&json, "float");
    assert_has(&json, "update");
    assert_lacks(&json, "config_loaded");
    assert_lacks(&json, "config_error");
    assert_lacks(&json, "tray_available");
    assert_lacks(&json, "tray_error");

    let monitor = json.get("monitor").expect("monitor snapshot");
    assert_has(monitor, "intervalSecs");
    assert_has(monitor, "lastFetchMs");
    assert_has(monitor, "notificationError");
    assert_lacks(monitor, "interval_secs");
    assert_lacks(monitor, "last_fetch_ms");
    assert_lacks(monitor, "notification_error");

    let float = json.get("float").expect("float snapshot");
    assert_has(float, "configuredMode");
    assert_has(float, "presentationMode");
    assert_has(float, "topDocked");
    // The old single `mode` field is gone: the DTO must not smuggle one
    // field with three meanings (persisted / native / render).
    assert_lacks(float, "mode");
    assert_lacks(float, "top_docked");

    let update = json.get("update").expect("update snapshot");
    assert_has(update, "downloadedVersion");
    assert_has(update, "lastCheckedMs");
    assert_has(update, "bannerDismissed");
    assert_lacks(update, "downloaded_version");
    assert_lacks(update, "last_checked_ms");
    assert_lacks(update, "banner_dismissed");
}

#[test]
fn boot_status_dto_is_a_small_camel_case_subset() {
    let state = state();
    state.apply_credential_check(CredentialCheckResult::Available);
    let json = serde_json::to_value(state.boot_dto()).expect("dto serializes");

    assert_has(&json, "version");
    assert_has(&json, "configured");
    assert_has(&json, "configLoaded");
    assert_has(&json, "configError");
    assert_has(&json, "credentials");
    assert_lacks(&json, "config_loaded");
    // Boot must not depend on runtime subsystems: no tray/monitor/float/
    // updater state may cross this boundary.
    assert_lacks(&json, "trayAvailable");
    assert_lacks(&json, "monitor");
    assert_lacks(&json, "float");
    assert_lacks(&json, "update");
}

#[test]
fn usage_dto_uses_camel_case_contract() {
    let state = state();
    {
        let mut usage = state.usage.lock().expect("usage mutex");
        usage.last_success_ms = Some(42);
    }
    let json = serde_json::to_value(state.usage_dto()).expect("dto serializes");
    assert_has(&json, "lastSuccessMs");
    assert_lacks(&json, "last_success_ms");
}

#[test]
fn update_info_dto_uses_camel_case_contract() {
    // Exercise the real `UpdateInfoDto` shape through `update_dto`: the
    // `available` field must serialize `releaseNotesUrl` in camelCase.
    let state = state();
    {
        let mut updater = state.updater.lock().expect("updater mutex");
        updater.status = UpdateStatus::Available;
    }
    let json = serde_json::to_value(state.update_dto()).expect("dto serializes");
    // Without a plugin `Update` object `available` is null; assert the DTO
    // shape is still correct at the container level.
    assert_has(&json, "available");
    assert_has(&json, "downloadedVersion");
    assert_has(&json, "progress");
    assert_has(&json, "error");
    assert_has(&json, "lastCheckedMs");
    assert_has(&json, "bannerDismissed");
    assert_lacks(&json, "downloaded_version");
}

#[test]
fn credential_and_update_phases_stay_snake_case() {
    // The enums intentionally serialize snake_case; the frontend mirrors
    // them as such. Pin the exact strings so a rename cannot drift.
    let timeout_state = state();
    timeout_state.apply_credential_check(CredentialCheckResult::Timeout);
    let json = serde_json::to_value(timeout_state.boot_dto()).expect("dto serializes");
    assert_eq!(
        json.pointer("/credentials/phase"),
        Some(&json!("timeout")),
        "credential phase must serialize as snake_case string"
    );

    let update_state = state();
    update_state.updater.lock().expect("updater mutex").status = UpdateStatus::ReadyToInstall;
    let json = serde_json::to_value(update_state.update_dto()).expect("dto serializes");
    assert_eq!(
        json.pointer("/status"),
        Some(&json!("ready_to_install")),
        "update status must serialize as snake_case string"
    );
}

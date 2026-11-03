//! IPC DTO JSON contract regression tests.
//!
//! The Rust DTOs serialize with `#[serde(rename_all = "camelCase")]` and the
//! frontend types in `src/types/models.ts` mirror the exact JSON keys. This
//! suite pins that contract on the Rust side: if a rename rule ever changes
//! (or a field is added without the frontend counterpart), CI fails here
//! instead of silently producing `undefined` fields that look like a hung
//! app in the UI (the "正在检查系统钥匙串…" freeze was exactly that).

use opencode_core::{
    BillingReport, GoAccess, GoActivityDay, GoActivityReport, GoMeter, GoModelUsage,
    GoStatusReport, OpenCodeError, QuotaService, TokenUsageReport,
};
use opencode_quota_checker_lib::credential_task::CredentialCheckResult;
use opencode_quota_checker_lib::error::AppError;
use opencode_quota_checker_lib::float::state::{DockPlan, FloatEdge};
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
        state
            .floating
            .lock()
            .expect("float mutex")
            .apply_dock_plan(DockPlan::docked(FloatEdge::Right));
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
    assert_eq!(
        float.get("presentationMode"),
        Some(&json!("docked_right")),
        "a docked presentation carries its edge in the presentation name"
    );
    assert_eq!(
        float.get("configuredMode"),
        Some(&json!("full")),
        "the persisted layout is independent of the dock edge"
    );
    // The old single `mode` field is gone: the DTO must not smuggle one
    // field with three meanings (persisted / native / render). The removed
    // `topDocked` flag is redundant now that `presentationMode` carries the
    // edge; a resurrected copy would desynchronize on side docking.
    assert_lacks(float, "mode");
    assert_lacks(float, "topDocked");
    assert_lacks(float, "top_docked");
    // The hover card is a presentation flag of the docked strip, not a mode:
    // it never changes `presentationMode` and it is false for this snapshot
    // (nothing hovers a docked window that was never shown).
    assert_has(float, "dockHover");
    assert_eq!(float.get("dockHover"), Some(&json!(false)));
    assert_lacks(float, "dock_hover");

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

#[test]
fn token_usage_dto_uses_the_expected_contract() {
    let state = state();
    {
        let mut token = state.token_usage.lock().expect("token usage mutex");
        token.report = Some(TokenUsageReport {
            range: "30d".to_owned(),
            requests: 123,
            input_tokens: 1_000,
            output_tokens: 2_000,
            cache_read_tokens: 3_000,
            cache_write_tokens: 100,
            total_tokens: 6_100,
            cost_microcents: 30_000_000,
            fetched_at: 1_778_800_000_000,
        });
        token.last_success_ms = Some(42);
    }
    let json = serde_json::to_value(state.token_usage_dto()).expect("dto serializes");

    // Container fields are camelCase.
    assert_has(&json, "lastSuccessMs");
    assert_has(&json, "loading");
    assert_has(&json, "report");
    assert_has(&json, "error");
    assert_lacks(&json, "last_success_ms");

    // The embedded core report keeps its serde field names (snake_case).
    let report = json.get("report").expect("report snapshot");
    assert_has(report, "input_tokens");
    assert_has(report, "output_tokens");
    assert_has(report, "cache_read_tokens");
    assert_has(report, "cache_write_tokens");
    assert_has(report, "total_tokens");
    assert_has(report, "cost_microcents");
    assert_lacks(report, "inputTokens");
    assert_lacks(report, "costMicrocents");
    assert_eq!(report.get("total_tokens"), Some(&json!(6_100)));
}

#[test]
fn usage_permission_error_has_a_stable_code() {
    let error = AppError::from(OpenCodeError::UsagePermissionDenied);
    assert_eq!(error.code, "usage_permission_denied");
    assert!(
        error.user.contains("Usage › Read"),
        "the permission message must name the required scope, got: {}",
        error.user
    );
}

#[test]
fn reset_token_usage_invalidates_in_flight_results() {
    let state = state();
    state.token_usage.lock().expect("token usage mutex").report = Some(TokenUsageReport {
        range: "30d".to_owned(),
        requests: 1,
        input_tokens: 1,
        output_tokens: 1,
        cache_read_tokens: 1,
        cache_write_tokens: 1,
        total_tokens: 4,
        cost_microcents: 1,
        fetched_at: 1,
    });
    let before = state
        .token_usage
        .lock()
        .expect("token usage mutex")
        .generation;
    state.reset_token_usage();
    let token = state.token_usage.lock().expect("token usage mutex");
    assert!(token.report.is_none(), "reset must clear the stale report");
    assert_eq!(
        token.generation,
        before + 1,
        "reset must invalidate an in-flight response"
    );
}

#[test]
fn go_overview_dto_uses_the_expected_contract() {
    let state = state();
    {
        let mut go = state.go_overview.lock().expect("go overview mutex");
        go.status = Some(GoStatusReport {
            product: "go".to_owned(),
            cancel_at_period_end: false,
            use_balance: false,
            access: Some(GoAccess {
                starts_at: 1,
                five_hour: None,
                week: None,
                month: Some(GoMeter {
                    used_microcents: 24,
                    limit_microcents: 100,
                    resets_at: 2,
                    percent: 24.0,
                    limited: false,
                }),
            }),
            fetched_at: 3,
        });
        go.activity = Some(GoActivityReport {
            starts_at: 1,
            daily: vec![GoActivityDay {
                date: "2026-10-26".to_owned(),
                cost_microcents: 10,
                requests: 2,
                tokens: 3,
            }],
            models: vec![GoModelUsage {
                provider: "opencode".to_owned(),
                model: "deepseek-v4.1-flash".to_owned(),
                requests: 2,
            }],
            total_cost_microcents: 10,
            total_requests: 2,
            total_tokens: 3,
            fetched_at: 4,
        });
        go.billing = Some(BillingReport {
            available_microcents: 0,
            balance_microcents: 0,
            billing_mode: "prepaid".to_owned(),
            fetched_at: 5,
        });
        go.last_success_ms = Some(6);
    }
    let json = serde_json::to_value(state.go_overview_dto()).expect("dto serializes");

    // Container fields are camelCase.
    assert_has(&json, "status");
    assert_has(&json, "activity");
    assert_has(&json, "billing");
    assert_has(&json, "statusError");
    assert_has(&json, "activityError");
    assert_has(&json, "billingError");
    assert_has(&json, "lastSuccessMs");
    assert_lacks(&json, "status_error");

    // Nested core reports keep their snake_case fields.
    let status = json.get("status").expect("status snapshot");
    assert_has(status, "cancel_at_period_end");
    assert_has(status, "use_balance");
    assert_lacks(status, "cancelAtPeriodEnd");
    let access = status.get("access").expect("access");
    assert_has(access, "starts_at");
    assert_lacks(access, "startsAt");
    let month = access.get("month").expect("monthly meter");
    assert_has(month, "used_microcents");
    assert_has(month, "limit_microcents");
    assert_has(month, "resets_at");
    assert_has(month, "percent");
    assert_has(month, "limited");
    assert_lacks(month, "usedMicroCents");

    let activity = json.get("activity").expect("activity snapshot");
    assert_has(activity, "starts_at");
    assert_has(activity, "total_cost_microcents");
    assert_has(activity, "total_requests");
    assert_has(activity, "total_tokens");
    assert_lacks(activity, "totalRequests");
    let first_day = activity
        .get("daily")
        .and_then(Value::as_array)
        .and_then(|days| days.first())
        .expect("first day");
    assert_has(first_day, "cost_microcents");
    assert_has(first_day, "requests");
    assert_has(first_day, "tokens");
    assert_lacks(first_day, "costMicrocents");

    let billing = json.get("billing").expect("billing snapshot");
    assert_has(billing, "available_microcents");
    assert_has(billing, "balance_microcents");
    assert_has(billing, "billing_mode");
    assert_lacks(billing, "availableMicroCents");
}

#[test]
fn reset_go_overview_invalidates_in_flight_results() {
    let state = state();
    state.go_overview.lock().expect("go overview mutex").status = Some(GoStatusReport {
        product: "go".to_owned(),
        cancel_at_period_end: false,
        use_balance: false,
        access: None,
        fetched_at: 1,
    });
    let before = state
        .go_overview
        .lock()
        .expect("go overview mutex")
        .generation;
    state.reset_go_overview();
    let go = state.go_overview.lock().expect("go overview mutex");
    assert!(go.status.is_none(), "reset must clear the stale status");
    assert_eq!(go.generation, before + 1);
}

#[test]
fn billing_permission_error_has_a_stable_code() {
    let error = AppError::from(OpenCodeError::BillingPermissionDenied);
    assert_eq!(error.code, "billing_permission_denied");
    assert!(
        error.user.contains("Billing › Read"),
        "the billing message must name the required scope, got: {}",
        error.user
    );
}

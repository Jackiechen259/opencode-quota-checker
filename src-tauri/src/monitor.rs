//! Background quota monitor.
//!
//! The monitor is a long-lived tokio task owned by the Rust backend, not by
//! any window or webview. It keeps polling while the main window is hidden to
//! the tray, evaluates thresholds with `opencode-core`, deduplicates alerts
//! per subscription cycle, delivers notifications, and emits `quota://*` /
//! `monitor://status` events. The React frontend only listens.
//!
//! Token usage is a second, best-effort data source fetched alongside the Go
//! allowance: it has its own state and events, runs in its own task, and can
//! never delay or fail a quota cycle.

use crate::credential_task;
use crate::error::AppError;
use crate::events;
use crate::state::{AppState, MonitorConfig};
use opencode_core::{evaluate_alerts, QuotaService};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

/// Minimum time between background token-usage requests.
///
/// The Go allowance keeps the user's configured cadence (30-3600 s), but the
/// 30-day token summary changes slowly; one request per five minutes is plenty
/// and keeps the internal Console endpoint out of Cloudflare rate limits.
const TOKEN_USAGE_MIN_INTERVAL_MS: i64 = 5 * 60 * 1_000;

/// How one monitor cycle treats the token-usage side channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenRefresh {
    /// Respect the five-minute throttle (background monitor).
    Throttled,
    /// Fetch immediately (startup probe, manual refresh, credential save).
    Force,
}

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
/// paths behave identically (including alert evaluation). The token-usage
/// side channel is throttled to five minutes.
pub async fn run_once(app: &AppHandle) {
    run_cycle(app, TokenRefresh::Throttled).await;
}

/// Runs one cycle and forces a token-usage request.
///
/// Used by the manual refresh command and by credential save/test probes so
/// the user sees fresh token data without waiting for the throttle window.
pub async fn run_once_forced(app: &AppHandle) {
    run_cycle(app, TokenRefresh::Force).await;
}

async fn run_cycle(app: &AppHandle, token_refresh: TokenRefresh) {
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
    crate::float::controller::sync_size(app);

    let config = state.config.lock().expect("config mutex").clone();
    let workspace = config.opencode_workspace_id.clone();
    let service = state.service.clone();
    let now_ms = chrono::Utc::now().timestamp_millis();

    // Read the key once; both the quota and token requests use the same value.
    let api_key = match credential_task::load_api_key().await {
        Ok(api_key) => api_key,
        Err(error) => {
            apply_quota_error(app, &state, error);
            emit_monitor_status(app);
            return;
        }
    };

    if matches!(token_refresh, TokenRefresh::Force) || token_is_due(&state, now_ms) {
        start_token_refresh(app, &state, api_key.clone(), now_ms);
    }

    let result = service
        .fetch_quota(&api_key, workspace.as_deref())
        .await
        .map_err(AppError::from);

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
            crate::float::controller::sync_size(app);
            let _ = app.emit(events::QUOTA_UPDATED, &report);
        }
        Err(error) => {
            apply_quota_error(app, &state, error);
        }
    }
    emit_monitor_status(app);
}

/// Clears the quota loading flags and publishes one failed quota cycle.
///
/// Token-usage state is deliberately untouched: a token failure is not a
/// quota failure and vice versa.
fn apply_quota_error(app: &AppHandle, state: &Arc<AppState>, error: AppError) {
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
    crate::float::controller::sync_size(app);
    let _ = app.emit(events::QUOTA_ERROR, &error);
}

/// Whether the token-usage side channel is due for another request.
fn token_is_due(state: &Arc<AppState>, now_ms: i64) -> bool {
    let token = state.token_usage.lock().expect("token usage mutex");
    token_due(token.last_attempt_ms, now_ms)
}

/// Pure throttle decision, kept separate so it can be tested without building
/// an `AppState` (and therefore without initializing the HTTP client).
fn token_due(last_attempt_ms: Option<i64>, now_ms: i64) -> bool {
    match last_attempt_ms {
        None => true,
        Some(last) => now_ms.saturating_sub(last) >= TOKEN_USAGE_MIN_INTERVAL_MS,
    }
}

/// Starts one detached token-usage request.
///
/// The task is independent of the quota request: it cannot block the quota
/// response, and its success/error only touches `AppState::token_usage`.
fn start_token_refresh(app: &AppHandle, state: &Arc<AppState>, api_key: String, now_ms: i64) {
    let generation = {
        let mut token = state.token_usage.lock().expect("token usage mutex");
        if token.loading {
            return;
        }
        token.loading = true;
        token.last_attempt_ms = Some(now_ms);
        token.generation
    };
    let _ = app.emit(events::TOKEN_USAGE_STATE, state.token_usage_dto());

    let app = app.clone();
    let state = Arc::clone(state);
    let service = state.service.clone();
    tauri::async_runtime::spawn(async move {
        let result = service
            .fetch_token_usage(&api_key)
            .await
            .map_err(AppError::from);

        {
            let mut token = state.token_usage.lock().expect("token usage mutex");
            if token.generation != generation {
                // Credentials were replaced or cleared while this request was
                // in flight; drop the stale result instead of overwriting the
                // new key's state.
                return;
            }
            token.loading = false;
            match result {
                Ok(report) => {
                    tracing::debug!(
                        total_tokens = report.total_tokens,
                        requests = report.requests,
                        "token usage refreshed"
                    );
                    token.report = Some(report);
                    token.error = None;
                    token.last_success_ms = Some(chrono::Utc::now().timestamp_millis());
                }
                Err(error) => {
                    tracing::warn!(%error, "token usage refresh failed");
                    token.error = Some(error);
                }
            }
        }
        let _ = app.emit(events::TOKEN_USAGE_STATE, state.token_usage_dto());
    });
}

fn emit_monitor_status(app: &AppHandle) {
    let state = app.state::<Arc<AppState>>().inner().clone();
    let _ = app.emit(events::MONITOR_STATUS, state.monitor_dto());
}

/// Convenience for commands: builds a `QuotaService` once.
pub fn service() -> Result<QuotaService, AppError> {
    QuotaService::new().map_err(AppError::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_throttle_still_applies_after_a_failed_first_attempt() {
        assert!(token_due(None, 0), "the first attempt is always due");
        assert!(
            !token_due(Some(1_000), 1_000 + TOKEN_USAGE_MIN_INTERVAL_MS - 1),
            "a failed first request must not retry every quota cycle"
        );
        assert!(
            token_due(Some(1_000), 1_000 + TOKEN_USAGE_MIN_INTERVAL_MS),
            "the request becomes due after the throttle window"
        );
    }
}

//! On-demand Go overview fetch for the Go usage page.
//!
//! The page is the only consumer of this data, so the fetch is intentionally
//! not part of the background monitor: opening the page or pressing refresh
//! starts it, and a five-minute cache prevents repeated requests while
//! navigating. The three Console requests run concurrently and each source has
//! its own error, allowing a partial page (for example, status and activity
//! without billing credit).

use crate::credential_task;
use crate::error::AppError;
use crate::events;
use crate::state::AppState;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

/// Minimum time between on-demand Go overview requests.
pub const GO_OVERVIEW_CACHE_MS: i64 = 5 * 60 * 1_000;

/// Starts a Go overview fetch when the cache is stale or `force` is set.
///
/// The function returns immediately; progress and the result arrive through
/// `go://overview`. A fetch already in flight is never duplicated.
pub fn request(app: &AppHandle, state: &Arc<AppState>, force: bool) {
    if !state.configured() {
        return;
    }
    let now_ms = chrono::Utc::now().timestamp_millis();
    let generation = {
        let mut go = state.go_overview.lock().expect("go overview mutex");
        if go.loading {
            return;
        }
        let due = force
            || go
                .last_attempt_ms
                .is_none_or(|last| now_ms.saturating_sub(last) >= GO_OVERVIEW_CACHE_MS);
        if !due {
            return;
        }
        go.loading = true;
        go.last_attempt_ms = Some(now_ms);
        go.generation
    };
    let _ = app.emit(events::GO_OVERVIEW_STATE, state.go_overview_dto());

    let app = app.clone();
    let state = Arc::clone(state);
    let service = state.service.clone();
    tauri::async_runtime::spawn(async move {
        let api_key = match credential_task::load_api_key().await {
            Ok(api_key) => api_key,
            Err(error) => {
                finish_with_key_error(app, state, generation, error);
                return;
            }
        };

        let (status, activity, billing) = tokio::join!(
            service.fetch_go_status(&api_key, force),
            service.fetch_go_activity(&api_key),
            service.fetch_billing_status(&api_key),
        );
        finish(app, state, generation, status, activity, billing);
    });
}

fn finish_with_key_error(app: AppHandle, state: Arc<AppState>, generation: u64, error: AppError) {
    {
        let mut go = state.go_overview.lock().expect("go overview mutex");
        if go.generation != generation {
            return;
        }
        go.loading = false;
        go.status_error = Some(error.clone());
        go.activity_error = Some(error.clone());
        go.billing_error = Some(error);
    }
    let _ = app.emit(events::GO_OVERVIEW_STATE, state.go_overview_dto());
}

fn finish(
    app: AppHandle,
    state: Arc<AppState>,
    generation: u64,
    status: Result<Option<opencode_core::GoStatusReport>, opencode_core::OpenCodeError>,
    activity: Result<Option<opencode_core::GoActivityReport>, opencode_core::OpenCodeError>,
    billing: Result<opencode_core::BillingReport, opencode_core::OpenCodeError>,
) {
    let (status_value, status_error) = split(status);
    let (activity_value, activity_error) = split(activity);
    let (billing_value, billing_error) = match billing {
        Ok(value) => (Some(value), None),
        Err(error) => (None, Some(AppError::from(error))),
    };

    if let Some(error) = status_error.as_ref() {
        tracing::warn!(%error, "go status refresh failed");
    }
    if let Some(error) = activity_error.as_ref() {
        tracing::warn!(%error, "go activity refresh failed");
    }
    if let Some(error) = billing_error.as_ref() {
        tracing::debug!(%error, "go billing refresh failed");
    }

    tracing::debug!(
        has_status = status_value.is_some(),
        has_activity = activity_value.is_some(),
        has_billing = billing_value.is_some(),
        "go overview refreshed"
    );
    let now_ms = chrono::Utc::now().timestamp_millis();
    {
        let mut go = state.go_overview.lock().expect("go overview mutex");
        if go.generation != generation {
            return;
        }
        go.loading = false;

        // A successful response (including `null`) replaces the source; an
        // error keeps the previous value so the page can show stale data with a
        // warning instead of going blank.
        match (status_value, status_error) {
            (Some(value), _) => {
                go.status = Some(value);
                go.status_error = None;
            }
            (None, Some(error)) => go.status_error = Some(error),
            (None, None) => {
                go.status = None;
                go.status_error = None;
            }
        }
        match (activity_value, activity_error) {
            (Some(value), _) => {
                go.activity = Some(value);
                go.activity_error = None;
            }
            (None, Some(error)) => go.activity_error = Some(error),
            (None, None) => {
                go.activity = None;
                go.activity_error = None;
            }
        }
        match (billing_value, billing_error) {
            (Some(value), _) => {
                go.billing = Some(value);
                go.billing_error = None;
            }
            (None, Some(error)) => go.billing_error = Some(error),
            (None, None) => {
                go.billing = None;
                go.billing_error = None;
            }
        }

        if go.status.is_some() || go.activity.is_some() {
            go.last_success_ms = Some(now_ms);
        }
    }
    let _ = app.emit(events::GO_OVERVIEW_STATE, state.go_overview_dto());
}

fn split<T>(
    result: Result<Option<T>, opencode_core::OpenCodeError>,
) -> (Option<T>, Option<AppError>) {
    match result {
        Ok(value) => (value, None),
        Err(error) => (None, Some(AppError::from(error))),
    }
}

//! Blocking credential layer.
//!
//! Every system-keyring operation (Windows Credential Manager, macOS
//! Keychain, Linux Secret Service) is synchronous and can stall for an
//! unbounded amount of time when the OS credential service misbehaves.
//!
//! Tauri runs synchronous `#[tauri::command]` functions on the main thread,
//! so a wedged keyring would freeze the whole window — including the webview
//! event loop that drives the title bar, drag regions and close button. This
//! module is the only place the app talks to the keyring:
//!
//! * [`tokio::task::spawn_blocking`] moves every synchronous call to the
//!   dedicated blocking pool, so neither the main thread nor any async
//!   executor thread is ever pinned.
//! * [`tokio::time::timeout`] applies a soft deadline ([`CREDENTIAL_TIMEOUT`]).
//!   The blocking thread itself cannot be cancelled, but the UI stops waiting
//!   after the deadline and receives an explicit result — a wedged
//!   Credential Manager can never leave the app stuck on
//!   "正在检查系统钥匙串…".
//!
//! The synchronous keyring abstraction in `opencode_core` is unchanged and
//! still the single storage backend; only the *way* it is invoked lives here.
//!
//! No secret ever crosses the logging boundary: only operation names,
//! durations and error kinds are traced.

use crate::error::AppError;
use opencode_core::{OpenCodeAuthStore, OpenCodeError};
use std::time::{Duration, Instant};
use tokio::time::timeout;

/// Soft deadline for every keyring operation.
///
/// Windows Credential Manager may hang indefinitely; after this the UI
/// proceeds without the result while the blocking worker keeps running in the
/// background. The worker is deliberately not cancelled — the goal is that
/// the UI never waits on it, not that the OS call is interrupted.
pub const CREDENTIAL_TIMEOUT: Duration = Duration::from_secs(5);

/// Outcome of one blocking-worker run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockingOutcome<T> {
    /// The operation finished and produced a value.
    Completed(T),
    /// The worker panicked before producing a value.
    WorkerPanicked,
    /// The worker did not finish within the soft deadline.
    TimedOut,
}

/// Runs a synchronous operation on the blocking pool with a soft timeout.
///
/// `operation` is `FnOnce` so callers can move captures (e.g. a cookie) in
/// without sharing state. Returns `BlockingOutcome::TimedOut` once `deadline`
/// elapses; the blocking thread keeps running in the background and its
/// result is dropped.
pub async fn run_blocking<T>(
    operation: impl FnOnce() -> T + Send + 'static,
    deadline: Duration,
) -> BlockingOutcome<T>
where
    T: Send + 'static,
{
    match timeout(deadline, tokio::task::spawn_blocking(operation)).await {
        Ok(Ok(value)) => BlockingOutcome::Completed(value),
        Ok(Err(join_error)) => {
            tracing::warn!(error = %join_error, "credential worker panicked");
            BlockingOutcome::WorkerPanicked
        }
        Err(_elapsed) => BlockingOutcome::TimedOut,
    }
}

/// Result of the credential availability check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialCheckResult {
    /// A cookie is stored and readable.
    Available,
    /// No cookie is stored (normal first-run state).
    Missing,
    /// The keyring itself failed; `error` carries the details.
    Error(AppError),
    /// The keyring did not answer within the soft deadline.
    Timeout,
}

/// Checks whether an OpenCode auth cookie is stored, with a soft timeout.
pub async fn check_credentials() -> CredentialCheckResult {
    #[cfg(debug_assertions)]
    if let Some(delay) = simulated_keyring_delay() {
        // Test-only: simulate a wedged Credential Manager so the timeout and
        // UI-responsiveness paths can be exercised on a real machine.
        return check_credentials_with(
            move || {
                std::thread::sleep(delay);
                OpenCodeAuthStore.load()
            },
            CREDENTIAL_TIMEOUT,
        )
        .await;
    }
    check_credentials_with(|| OpenCodeAuthStore.load(), CREDENTIAL_TIMEOUT).await
}

/// Debug-build-only hook: `OQC_KEYRING_DELAY_MS` makes the boot keyring
/// check block for the given duration (inside the blocking worker), which
/// forces the soft-timeout path. Never compiled into release builds.
#[cfg(debug_assertions)]
fn simulated_keyring_delay() -> Option<Duration> {
    std::env::var("OQC_KEYRING_DELAY_MS")
        .ok()
        .and_then(|value| value.parse().ok())
        .map(Duration::from_millis)
}

/// Testable core of [`check_credentials`]: runs an injected load operation.
async fn check_credentials_with(
    load: impl FnOnce() -> Result<String, OpenCodeError> + Send + 'static,
    deadline: Duration,
) -> CredentialCheckResult {
    let started = Instant::now();
    tracing::info!("credential check started");
    match run_blocking(load, deadline).await {
        BlockingOutcome::Completed(Ok(_)) => {
            tracing::info!(
                duration_ms = started.elapsed().as_millis() as u64,
                "credential check success"
            );
            CredentialCheckResult::Available
        }
        BlockingOutcome::Completed(Err(OpenCodeError::CredentialsMissing)) => {
            tracing::info!(
                duration_ms = started.elapsed().as_millis() as u64,
                "credential missing"
            );
            CredentialCheckResult::Missing
        }
        BlockingOutcome::Completed(Err(error)) => {
            tracing::warn!(
                duration_ms = started.elapsed().as_millis() as u64,
                error = %error,
                "credential check failed"
            );
            CredentialCheckResult::Error(error.into())
        }
        BlockingOutcome::TimedOut => {
            tracing::warn!(
                duration_ms = started.elapsed().as_millis() as u64,
                "credential check timeout"
            );
            CredentialCheckResult::Timeout
        }
        BlockingOutcome::WorkerPanicked => CredentialCheckResult::Error(worker_panicked_error()),
    }
}

/// Loads the stored cookie on the blocking pool with a soft timeout.
///
/// Used by every quota request: the cookie read must never stall the UI, and
/// it must complete (or time out) before the HTTP request starts.
pub async fn load_cookie() -> Result<String, AppError> {
    load_cookie_with(|| OpenCodeAuthStore.load(), CREDENTIAL_TIMEOUT).await
}

/// Testable core of [`load_cookie`].
async fn load_cookie_with(
    load: impl FnOnce() -> Result<String, OpenCodeError> + Send + 'static,
    deadline: Duration,
) -> Result<String, AppError> {
    let started = Instant::now();
    match run_blocking(load, deadline).await {
        BlockingOutcome::Completed(Ok(cookie)) => {
            tracing::info!(
                duration_ms = started.elapsed().as_millis() as u64,
                "credential load success"
            );
            Ok(cookie)
        }
        BlockingOutcome::Completed(Err(error)) => {
            tracing::warn!(
                duration_ms = started.elapsed().as_millis() as u64,
                error = %error,
                "credential load failed"
            );
            Err(error.into())
        }
        BlockingOutcome::TimedOut => {
            tracing::warn!(
                duration_ms = started.elapsed().as_millis() as u64,
                "credential load timeout"
            );
            Err(timeout_error("读取"))
        }
        BlockingOutcome::WorkerPanicked => Err(worker_panicked_error()),
    }
}

/// Saves the OpenCode auth cookie on the blocking pool with a soft timeout.
///
/// The cookie is moved into the blocking worker and never logged.
pub async fn save_cookie(cookie: String) -> Result<(), AppError> {
    save_cookie_with(move || OpenCodeAuthStore.save(&cookie), CREDENTIAL_TIMEOUT).await
}

/// Testable core of [`save_cookie`].
async fn save_cookie_with(
    save: impl FnOnce() -> Result<(), OpenCodeError> + Send + 'static,
    deadline: Duration,
) -> Result<(), AppError> {
    let started = Instant::now();
    tracing::info!("credential save started");
    match run_blocking(save, deadline).await {
        BlockingOutcome::Completed(Ok(())) => {
            tracing::info!(
                duration_ms = started.elapsed().as_millis() as u64,
                "credential save success"
            );
            Ok(())
        }
        BlockingOutcome::Completed(Err(error)) => {
            tracing::warn!(
                duration_ms = started.elapsed().as_millis() as u64,
                error = %error,
                "credential save failed"
            );
            Err(error.into())
        }
        BlockingOutcome::TimedOut => {
            tracing::warn!(
                duration_ms = started.elapsed().as_millis() as u64,
                "credential save timeout"
            );
            Err(timeout_error("保存"))
        }
        BlockingOutcome::WorkerPanicked => Err(worker_panicked_error()),
    }
}

/// Clears the OpenCode auth cookie on the blocking pool with a soft timeout.
pub async fn clear_cookie() -> Result<(), AppError> {
    clear_cookie_with(|| OpenCodeAuthStore.clear(), CREDENTIAL_TIMEOUT).await
}

/// Testable core of [`clear_cookie`].
async fn clear_cookie_with(
    clear: impl FnOnce() -> Result<(), OpenCodeError> + Send + 'static,
    deadline: Duration,
) -> Result<(), AppError> {
    let started = Instant::now();
    tracing::info!("credential clear started");
    match run_blocking(clear, deadline).await {
        BlockingOutcome::Completed(Ok(())) => {
            tracing::info!(
                duration_ms = started.elapsed().as_millis() as u64,
                "credential clear success"
            );
            Ok(())
        }
        BlockingOutcome::Completed(Err(error)) => {
            tracing::warn!(
                duration_ms = started.elapsed().as_millis() as u64,
                error = %error,
                "credential clear failed"
            );
            Err(error.into())
        }
        BlockingOutcome::TimedOut => {
            tracing::warn!(
                duration_ms = started.elapsed().as_millis() as u64,
                "credential clear timeout"
            );
            Err(timeout_error("清除"))
        }
        BlockingOutcome::WorkerPanicked => Err(worker_panicked_error()),
    }
}

/// User-facing error for a timed-out keyring operation.
pub fn timeout_error(operation: &str) -> AppError {
    AppError::new(
        "keyring_timeout",
        match operation {
            "读取" => "无法读取系统凭据：系统钥匙串响应超时。请重试。".to_owned(),
            _ => format!("系统钥匙串响应超时，未能{operation}系统凭据。请重试。"),
        },
        format!(
            "credential {operation} timed out after {} seconds",
            CREDENTIAL_TIMEOUT.as_secs()
        ),
    )
}

fn worker_panicked_error() -> AppError {
    AppError::new(
        "keyring_worker_panicked",
        "系统钥匙串操作异常终止，请重试。",
        "credential worker panicked",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    const FAST_DEADLINE: Duration = Duration::from_millis(20);
    const SLOW_WORK: Duration = Duration::from_millis(200);

    #[tokio::test]
    async fn blocking_operation_success_completes() {
        let outcome = run_blocking(|| 42, CREDENTIAL_TIMEOUT).await;
        assert_eq!(outcome, BlockingOutcome::Completed(42));
    }

    #[tokio::test]
    async fn blocking_operation_panic_reports_worker_panicked() {
        let outcome = run_blocking(|| -> i32 { panic!("boom") }, CREDENTIAL_TIMEOUT).await;
        assert_eq!(outcome, BlockingOutcome::WorkerPanicked);
    }

    #[tokio::test]
    async fn slow_operation_times_out_and_future_returns() {
        let started = Instant::now();
        let outcome = run_blocking(
            || {
                thread::sleep(SLOW_WORK);
                1
            },
            FAST_DEADLINE,
        )
        .await;
        assert_eq!(outcome, BlockingOutcome::TimedOut);
        assert!(
            started.elapsed() < SLOW_WORK,
            "the future must return promptly after the soft deadline, \
             not wait for the blocking worker"
        );
    }

    #[tokio::test]
    async fn timed_out_worker_keeps_running_in_background() {
        // The timeout is soft: the blocking worker is not cancelled, it just
        // stops being awaited. The flag proves the worker still completes.
        let finished = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = finished.clone();
        let outcome = run_blocking(
            move || {
                thread::sleep(Duration::from_millis(300));
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
                1
            },
            FAST_DEADLINE,
        )
        .await;
        assert_eq!(outcome, BlockingOutcome::TimedOut);
        while !finished.load(std::sync::atomic::Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    #[tokio::test]
    async fn check_maps_available() {
        let result = check_credentials_with(|| Ok("cookie".to_owned()), CREDENTIAL_TIMEOUT).await;
        assert_eq!(result, CredentialCheckResult::Available);
    }

    #[tokio::test]
    async fn check_maps_missing() {
        let result = check_credentials_with(
            || Err(OpenCodeError::CredentialsMissing),
            CREDENTIAL_TIMEOUT,
        )
        .await;
        assert_eq!(result, CredentialCheckResult::Missing);
    }

    #[tokio::test]
    async fn check_maps_keyring_error() {
        let result = check_credentials_with(
            || Err(OpenCodeError::CredentialsInvalid("boom".to_owned())),
            CREDENTIAL_TIMEOUT,
        )
        .await;
        assert!(
            matches!(result, CredentialCheckResult::Error(_)),
            "keyring errors must surface as Error"
        );
    }

    #[tokio::test]
    async fn check_maps_timeout_and_returns() {
        let started = Instant::now();
        let result = check_credentials_with(
            || {
                thread::sleep(SLOW_WORK);
                Ok("late".to_owned())
            },
            FAST_DEADLINE,
        )
        .await;
        assert_eq!(result, CredentialCheckResult::Timeout);
        assert!(
            started.elapsed() < SLOW_WORK,
            "the check must return after the soft deadline, not after the worker"
        );
    }

    #[tokio::test]
    async fn load_cookie_success_returns_value() {
        let result = load_cookie_with(|| Ok("cookie".to_owned()), CREDENTIAL_TIMEOUT).await;
        assert_eq!(result.expect("cookie loads"), "cookie");
    }

    #[tokio::test]
    async fn load_cookie_timeout_is_a_ui_error() {
        let result = load_cookie_with(
            || {
                thread::sleep(SLOW_WORK);
                Ok("late".to_owned())
            },
            FAST_DEADLINE,
        )
        .await;
        let error = result.expect_err("timeout must fail the load");
        assert!(
            error.user.contains("超时"),
            "timeout must produce a user-facing message, got: {}",
            error.user
        );
        assert!(
            error.detail.contains("timed out"),
            "timeout must produce a technical detail, got: {}",
            error.detail
        );
    }

    #[tokio::test]
    async fn save_cookie_timeout_is_a_ui_error() {
        let result = save_cookie_with(
            || {
                thread::sleep(SLOW_WORK);
                Ok(())
            },
            FAST_DEADLINE,
        )
        .await;
        let error = result.expect_err("timeout must fail the save");
        assert!(error.user.contains("超时"));
    }

    #[tokio::test]
    async fn clear_cookie_success_completes() {
        let result = clear_cookie_with(|| Ok(()), CREDENTIAL_TIMEOUT).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn clear_cookie_error_is_surfaced() {
        let result = clear_cookie_with(
            || Err(OpenCodeError::CredentialsInvalid("boom".to_owned())),
            CREDENTIAL_TIMEOUT,
        )
        .await;
        assert!(result.is_err());
    }
}

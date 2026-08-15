//! Shared application state.
//!
//! One `AppState` instance lives in the Tauri manager and is shared by the
//! main window, the floating window, the tray, the monitor task and every IPC
//! command. The frontend never mirrors this state; it receives snapshots and
//! events.

use crate::config::{AppConfig, FloatMode, FloatPosition};
use crate::credential_task::CredentialCheckResult;
use crate::error::AppError;
use opencode_core::{QuotaService, UsageReport};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::watch;

/// Configuration watched by the background monitor task.
///
/// `None` disables the loop; `Some` carries the interval for the next cycle.
#[derive(Debug, Clone, Copy)]
pub struct MonitorConfig {
    pub interval_secs: u64,
}

/// Lifecycle of the system-keyring availability check.
///
/// A check either completes into a terminal phase or times out; no error path
/// may leave the state machine on [`CredentialPhase::Checking`], so the UI
/// can never be stuck on "正在检查系统钥匙串…" forever.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialPhase {
    /// The initial or retried keyring check is in flight.
    #[default]
    Checking,
    /// A cookie is stored and readable.
    Available,
    /// No cookie is stored (normal first-run state).
    Missing,
    /// The keyring failed; `error` carries the details.
    Error,
    /// The keyring did not answer within the soft deadline.
    Timeout,
}

/// Keyring availability reported during startup.
#[derive(Debug, Default)]
pub struct CredentialStatus {
    pub phase: CredentialPhase,
    pub available: bool,
    pub error: Option<AppError>,
}

/// Latest usage request state shared with the frontend.
#[derive(Debug, Default)]
pub struct UsageState {
    pub report: Option<UsageReport>,
    /// Latest raw dashboard response, held only in memory.
    pub raw: Option<String>,
    pub loading: bool,
    pub raw_loading: bool,
    pub error: Option<AppError>,
    pub last_success_ms: Option<i64>,
}

/// Monitoring and notification deduplication state.
#[derive(Debug, Default)]
pub struct MonitorState {
    /// Whether the monitor is enabled by configuration.
    pub enabled: bool,
    /// Configured polling interval in seconds.
    pub interval_secs: u64,
    /// Whether a fetch is currently in flight.
    pub loading: bool,
    /// Timestamp of the last successful fetch.
    pub last_fetch_ms: Option<i64>,
    /// Latest fetch error while retaining any previous report.
    pub error: Option<AppError>,
    /// Last alerted subscription cycle per quota window (deduplication).
    pub last_alerted: HashMap<String, i64>,
    /// Latest notification delivery error.
    pub notification_error: Option<AppError>,
}

/// Transient floating-window state.
#[derive(Debug, Default)]
pub struct FloatState {
    pub open: bool,
    pub mode: FloatMode,
    /// Whether the window is temporarily snapped to the monitor top.
    pub top_docked: bool,
    /// Whether a move event is waiting for debounced persistence.
    pub position_dirty: bool,
    /// Debounce generation; only the newest move wins the write.
    pub position_gen: u64,
}

/// Lifecycle of the updater state machine; mirrors the archived client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateStatus {
    #[default]
    Idle,
    Checking,
    Available,
    Downloading,
    ReadyToInstall,
    Installing,
    UpToDate,
    Error,
}

impl UpdateStatus {
    /// Whether an update request should be refused to avoid concurrency.
    pub fn busy(self) -> bool {
        matches!(self, Self::Checking | Self::Downloading | Self::Installing)
    }
}

/// Update state; `available` holds the plugin update object (never crossed
/// over IPC) and `downloaded` holds the verified package bytes in memory.
#[derive(Default)]
pub struct UpdateState {
    pub status: UpdateStatus,
    pub available: Option<tauri_plugin_updater::Update>,
    /// Verified package bytes from the plugin download.
    pub downloaded: Option<Vec<u8>>,
    pub downloaded_version: Option<String>,
    /// Bytes downloaded so far and total bytes when known.
    pub progress: Option<(u64, Option<u64>)>,
    pub error: Option<AppError>,
    pub last_checked_ms: Option<i64>,
    pub banner_dismissed: bool,
}

/// The single state container shared by every window and background task.
pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub usage: Mutex<UsageState>,
    pub monitor: Mutex<MonitorState>,
    pub floating: Mutex<FloatState>,
    pub updater: Mutex<UpdateState>,
    pub credentials: Mutex<CredentialStatus>,
    /// Tray handle; `None` when the tray failed to initialize.
    pub tray: Mutex<Option<crate::tray::TrayHandle>>,
    pub tray_error: RwLock<Option<String>>,
    /// Error from the startup config load (corrupt file), shown in settings.
    pub config_error: RwLock<Option<AppError>>,
    pub config_loaded: AtomicBool,
    /// Cloneable HTTP + parsing service.
    pub service: QuotaService,
    /// Drives the background monitor task.
    pub monitor_tx: watch::Sender<Option<MonitorConfig>>,
}

impl AppState {
    /// Creates fresh state before any config load.
    pub fn new(service: QuotaService, monitor_tx: watch::Sender<Option<MonitorConfig>>) -> Self {
        Self {
            config: Mutex::new(AppConfig::default()),
            usage: Mutex::new(UsageState::default()),
            monitor: Mutex::new(MonitorState::default()),
            floating: Mutex::new(FloatState::default()),
            updater: Mutex::new(UpdateState::default()),
            // The initial keyring check starts in the Checking phase; the
            // background check task applies the terminal outcome.
            credentials: Mutex::new(CredentialStatus::default()),
            tray: Mutex::new(None),
            tray_error: RwLock::new(None),
            config_error: RwLock::new(None),
            config_loaded: AtomicBool::new(false),
            service,
            monitor_tx,
        }
    }

    /// Reports whether the OpenCode workspace and auth cookie are configured.
    pub fn configured(&self) -> bool {
        self.credentials.lock().expect("credential mutex").available
            && self
                .config
                .lock()
                .expect("config mutex")
                .opencode_workspace_id
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
    }

    /// Applies a freshly loaded/saved config to the live state.
    pub fn apply_config(&self, config: AppConfig) {
        *self.config.lock().expect("config mutex") = config.clone();
        self.config_loaded.store(true, Ordering::SeqCst);
        {
            let mut monitor = self.monitor.lock().expect("monitor mutex");
            monitor.enabled = config.monitor_enabled;
            monitor.interval_secs = config.monitor_interval_secs;
        }
        {
            let mut floating = self.floating.lock().expect("float mutex");
            floating.mode = config.float_mode;
            floating.open = config.float_open;
        }
        self.push_monitor_config();
    }

    /// Applies one keyring-check outcome to the credential state.
    ///
    /// Every terminal outcome leaves [`CredentialPhase::Checking`]; the only
    /// way back into `Checking` is a new check (startup or explicit retry).
    pub fn apply_credential_check(&self, result: CredentialCheckResult) {
        let mut credentials = self.credentials.lock().expect("credential mutex");
        match result {
            CredentialCheckResult::Available => {
                credentials.phase = CredentialPhase::Available;
                credentials.available = true;
                credentials.error = None;
            }
            CredentialCheckResult::Missing => {
                credentials.phase = CredentialPhase::Missing;
                credentials.available = false;
                credentials.error = None;
            }
            CredentialCheckResult::Error(error) => {
                credentials.phase = CredentialPhase::Error;
                credentials.available = false;
                credentials.error = Some(error);
            }
            CredentialCheckResult::Timeout => {
                credentials.phase = CredentialPhase::Timeout;
                credentials.available = false;
                credentials.error = Some(crate::credential_task::timeout_error("读取"));
            }
        }
    }

    /// Pushes the current monitor configuration to the background task.
    ///
    /// Never holds a lock while calling `configured()`: `std::sync::Mutex`
    /// is not reentrant, and doing so deadlocks the caller thread.
    pub fn push_monitor_config(&self) {
        let configured = self.configured();
        let (enabled, interval_secs) = {
            let config = self.config.lock().expect("config mutex");
            (config.monitor_enabled, config.monitor_interval_secs)
        };
        let payload = (configured && enabled).then_some(MonitorConfig { interval_secs });
        let _ = self.monitor_tx.send(payload);
    }
}

// ---------------------------------------------------------------------------
// Serializable DTOs crossing the IPC boundary
// ---------------------------------------------------------------------------

/// Snapshot returned by `get_app_status`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatusDto {
    pub version: String,
    pub configured: bool,
    pub config_loaded: bool,
    pub config_error: Option<AppError>,
    pub credentials: CredentialStatusDto,
    pub tray_available: bool,
    pub tray_error: Option<String>,
    pub monitor: MonitorStatusDto,
    pub float: FloatStateDto,
    pub update: UpdateStateDto,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CredentialStatusDto {
    pub phase: CredentialPhase,
    pub available: bool,
    pub error: Option<AppError>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MonitorStatusDto {
    pub enabled: bool,
    pub interval_secs: u64,
    pub loading: bool,
    pub last_fetch_ms: Option<i64>,
    pub error: Option<AppError>,
    pub notification_error: Option<AppError>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FloatStateDto {
    pub open: bool,
    pub mode: FloatMode,
    pub top_docked: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageDto {
    pub report: Option<UsageReport>,
    pub loading: bool,
    pub error: Option<AppError>,
    pub last_success_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfoDto {
    pub version: String,
    pub tag: String,
    pub release_notes_url: String,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProgressDto {
    pub downloaded: u64,
    pub total: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStateDto {
    pub status: UpdateStatus,
    pub available: Option<UpdateInfoDto>,
    pub downloaded_version: Option<String>,
    pub progress: Option<UpdateProgressDto>,
    pub error: Option<AppError>,
    pub last_checked_ms: Option<i64>,
    pub banner_dismissed: bool,
}

impl AppState {
    /// Builds the current update snapshot for IPC/events (no plugin objects).
    pub fn update_dto(&self) -> UpdateStateDto {
        let updater = self.updater.lock().expect("updater mutex");
        UpdateStateDto {
            status: updater.status,
            available: updater.available.as_ref().map(|update| UpdateInfoDto {
                version: update.version.clone(),
                tag: format!("v{}", update.version),
                release_notes_url: format!(
                    "{}/releases/tag/v{}",
                    env!("CARGO_PKG_REPOSITORY"),
                    update.version
                ),
                body: update.body.clone(),
            }),
            downloaded_version: updater.downloaded_version.clone(),
            progress: updater
                .progress
                .map(|(downloaded, total)| UpdateProgressDto { downloaded, total }),
            error: updater.error.clone(),
            last_checked_ms: updater.last_checked_ms,
            banner_dismissed: updater.banner_dismissed,
        }
    }

    /// Builds the current monitor snapshot for IPC/events.
    pub fn monitor_dto(&self) -> MonitorStatusDto {
        let monitor = self.monitor.lock().expect("monitor mutex");
        MonitorStatusDto {
            enabled: monitor.enabled,
            interval_secs: monitor.interval_secs,
            loading: monitor.loading,
            last_fetch_ms: monitor.last_fetch_ms,
            error: monitor.error.clone(),
            notification_error: monitor.notification_error.clone(),
        }
    }

    /// Builds the current floating-window snapshot for IPC/events.
    pub fn float_dto(&self) -> FloatStateDto {
        let floating = self.floating.lock().expect("float mutex");
        FloatStateDto {
            open: floating.open,
            mode: floating.mode,
            top_docked: floating.top_docked,
        }
    }

    /// Builds the full application status snapshot.
    pub fn status_dto(&self) -> AppStatusDto {
        // `configured()` acquires the credentials mutex itself; it must run
        // before we take that mutex here (std Mutex is not reentrant).
        let configured = self.configured();
        let credentials = self.credentials.lock().expect("credential mutex");
        AppStatusDto {
            version: env!("CARGO_PKG_VERSION").to_owned(),
            configured,
            config_loaded: self.config_loaded.load(Ordering::SeqCst),
            config_error: self
                .config_error
                .read()
                .expect("config error rwlock")
                .clone(),
            credentials: CredentialStatusDto {
                phase: credentials.phase,
                available: credentials.available,
                error: credentials.error.clone(),
            },
            tray_available: self.tray.lock().expect("tray mutex").is_some(),
            tray_error: self.tray_error.read().expect("tray error rwlock").clone(),
            monitor: self.monitor_dto(),
            float: self.float_dto(),
            update: self.update_dto(),
        }
    }

    /// Records a moved float position and schedules a debounced persistence.
    ///
    /// Takes the `Arc` so the spawned task can keep its own reference to the
    /// shared state.
    pub fn mark_float_moved(self: &Arc<Self>, app: &tauri::AppHandle, position: FloatPosition) {
        {
            let mut config = self.config.lock().expect("config mutex");
            config.float_position = Some(position);
        }
        {
            let mut floating = self.floating.lock().expect("float mutex");
            floating.position_dirty = true;
            floating.position_gen += 1;
            let generation = floating.position_gen;
            drop(floating);
            let state = Arc::clone(self);
            let handle = app.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(750)).await;
                let mut floating = state.floating.lock().expect("float mutex");
                if floating.position_dirty && floating.position_gen == generation {
                    floating.position_dirty = false;
                    crate::persistence::persist_config(&handle);
                }
            });
        }
    }
}

use crate::config::AppConfig;
use crate::platform::tray::TrayAction;
use crate::state::UiError;
use iced::window;
use std::fmt;
use volc_core::UsageReport;

/// Input text whose debug representation must never expose its value.
#[derive(Clone)]
pub struct SensitiveInput(pub String);

impl fmt::Debug for SensitiveInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

/// Threshold field edited in the settings overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThresholdField {
    /// Five-hour quota.
    FiveHour,
    /// Weekly quota.
    Weekly,
    /// Monthly quota.
    Monthly,
}

/// Every external event and asynchronous result entering the application.
#[derive(Debug, Clone)]
pub enum Message {
    MainWindowOpened(window::Id),
    CloseRequested(window::Id),
    PollTray,
    Tray(TrayAction),
    HideMain,
    ConfigLoaded(Result<AppConfig, UiError>),
    OpenSettings,
    CloseSettings,
    IntervalChanged(String),
    ThresholdChanged(ThresholdField, String),
    StartMonitor,
    StopMonitor,
    ConfigSaved(Result<AppConfig, UiError>),
    MonitorTick,
    NotificationsDelivered(Result<(), UiError>),
    CredentialsChecked(Result<bool, UiError>),
    AccessKeyChanged(SensitiveInput),
    SecretKeyChanged(SensitiveInput),
    SaveCredentials,
    CredentialsSaved(Result<(), UiError>),
    ClearCredentials,
    CredentialsCleared(Result<(), UiError>),
    Refresh,
    UsageLoaded(Result<UsageReport, UiError>),
    LoadRaw,
    RawLoaded(Result<String, UiError>),
    Tick(i64),
    Exit,
}

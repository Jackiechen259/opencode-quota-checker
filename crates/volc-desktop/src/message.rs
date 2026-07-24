use crate::config::{AppConfig, FloatMode};
use crate::platform::tray::TrayAction;
use crate::state::UiError;
use iced::{keyboard, window, Size};
use std::fmt;
use volc_core::UsageReport;

#[derive(Clone)]
pub struct SensitiveInput(pub String);

impl fmt::Debug for SensitiveInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThresholdField {
    FiveHour,
    Weekly,
    Monthly,
}

#[derive(Debug, Clone)]
pub enum Message {
    MainWindowOpened(window::Id),
    FloatWindowOpened(window::Id),
    CloseRequested(window::Id),
    WindowEvent(window::Id, window::Event),
    Keyboard(keyboard::Event),
    PollTray,
    Tray(TrayAction),
    HideMain,
    ToggleFloat,
    CloseFloat,
    FloatModeChanged(FloatMode),
    DragFloat,
    FloatMonitorSize(window::Id, Option<Size>),
    PersistFloatPosition,
    ConfigPersisted(Result<AppConfig, UiError>),
    ConfigLoaded(Result<AppConfig, UiError>),
    OpenSettings,
    CloseSettings,
    CloseOverlay,
    CopyRaw,
    DismissToast,
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

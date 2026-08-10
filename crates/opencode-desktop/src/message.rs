use crate::config::{AppConfig, FloatMode};
use crate::platform::tray::TrayAction;
use crate::state::UiError;
use crate::update::{UpdateInfo, VerifiedPackage};
#[cfg(not(target_os = "windows"))]
use iced::Size;
use iced::{keyboard, window};
use opencode_core::UsageReport;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderAction {
    Refresh,
    Details,
    Settings,
    Float,
    Hide,
    Exit,
    More,
}

impl HeaderAction {
    /// Actions rendered directly in the control bar, in visual order.
    pub const BAR: &[Self] = &[Self::Refresh, Self::Settings, Self::Float, Self::More];
    /// Actions rendered in the overflow menu, in visual order.
    pub const MENU: &[Self] = &[Self::Details, Self::Hide, Self::Exit];

    /// The focusable sequence for the current menu state; when the menu is
    /// open, Tab cycles through its items.
    pub fn focus_order(menu_open: bool) -> &'static [Self] {
        if menu_open {
            Self::MENU
        } else {
            Self::BAR
        }
    }

    pub fn message(self) -> Message {
        match self {
            Self::Refresh => Message::Refresh,
            Self::Details => Message::LoadRaw,
            Self::Settings => Message::OpenSettings,
            Self::Float => Message::ToggleFloat,
            Self::Hide => Message::HideMain,
            Self::Exit => Message::Exit,
            Self::More => Message::ToggleHeaderMenu,
        }
    }
}

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
    HeaderPressed(HeaderAction),
    /// Toggle the header overflow menu.
    ToggleHeaderMenu,
    /// Close the header overflow menu (outside click or Escape).
    CloseHeaderMenu,
    PollTray,
    Tray(TrayAction),
    HideMain,
    ToggleFloat,
    CloseFloat,
    FloatModeChanged(FloatMode),
    DragFloat,
    /// Begin moving the main window with the title bar.
    DragMainWindow,
    /// Minimize the main window.
    MinimizeMainWindow,
    /// Toggle the main window between maximized and restored.
    ToggleMaximizeMainWindow,
    /// Close the main window through the normal close-to-tray / exit path.
    CloseMainWindow,
    /// True window-maximized state reported by the platform.
    MainWindowMaximized(bool),
    #[cfg(target_os = "windows")]
    FloatWindowGeometry(
        window::Id,
        Option<crate::window::float_window::WindowGeometry>,
    ),
    #[cfg(not(target_os = "windows"))]
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
    OpenCodeWorkspaceChanged(String),
    OpenCodeCookieChanged(SensitiveInput),
    StartOpenCodeLogin,
    SaveOpenCodeCredentials,
    OpenCodeCredentialsSaved(Result<(), UiError>),
    OpenCodeCredentialsCleared(Result<(), UiError>),
    CancelClearCredentials,
    ClearCredentials,
    ConfirmClearCredentials,
    Refresh,
    UsageLoaded(Result<UsageReport, UiError>),
    LoadRaw,
    RawLoaded(Result<String, UiError>),
    Tick(i64),
    /// Manually request an update check.
    CheckForUpdate,
    /// Result of a check; `None` means the running version is current.
    UpdateChecked(Result<Option<UpdateInfo>, UiError>),
    /// Periodic timer to re-check for updates.
    UpdateCheckTick,
    /// Download progress from the streaming downloader.
    UpdateDownloadProgress {
        /// Bytes downloaded so far.
        downloaded: u64,
        /// Total bytes when known.
        total: Option<u64>,
    },
    /// Start downloading the available update.
    DownloadUpdate,
    /// Result of a package download.
    UpdateDownloaded(Result<VerifiedPackage, UiError>),
    /// Confirm and install the verified update.
    InstallUpdate,
    /// Result of launching the platform installer. `Ok(true)` means the app
    /// should exit; `Ok(false)` means the package was opened and it keeps
    /// running.
    UpdateInstallStarted(Result<bool, UiError>),
    /// Hide the dashboard update banner for this run.
    DismissUpdate,
    /// Open the release notes in the system browser.
    OpenReleaseNotes,
    /// Toggle whether startup/periodic checks are enabled.
    UpdateChecksEnabledChanged(bool),
    /// Toggle whether found updates auto-download.
    AutoDownloadUpdatesChanged(bool),
    Exit,
}

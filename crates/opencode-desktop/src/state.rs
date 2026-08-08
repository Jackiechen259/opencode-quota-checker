use crate::message::HeaderAction;
use crate::update::{UpdateError, UpdateInfo, VerifiedPackage};
use iced::window;
use opencode_core::{OpenCodeError, UsageReport};
use std::collections::HashMap;
/// IDs of all windows owned by the application state machine.
#[derive(Debug, Default)]
pub struct WindowState {
    main: Option<window::Id>,
    floating: Option<window::Id>,
}

impl WindowState {
    /// Returns the current main-window ID.
    pub fn main(&self) -> Option<window::Id> {
        self.main
    }

    /// Records the only main-window instance.
    pub fn set_main(&mut self, id: window::Id) {
        self.main = Some(id);
    }

    /// Removes and returns the main-window ID.
    pub fn take_main(&mut self) -> Option<window::Id> {
        self.main.take()
    }

    /// Returns the current floating-window ID.
    pub fn floating(&self) -> Option<window::Id> {
        self.floating
    }

    /// Records the only floating-window instance.
    pub fn set_floating(&mut self, id: window::Id) {
        self.floating = Some(id);
    }

    /// Removes and returns the floating-window ID.
    pub fn take_floating(&mut self) -> Option<window::Id> {
        self.floating.take()
    }
}

/// Cloneable UI-safe representation of a technical error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiError {
    /// Concise message shown in the primary interface.
    pub user: String,
    /// Technical detail shown only in debug/error details.
    pub detail: String,
}

impl From<OpenCodeError> for UiError {
    fn from(error: OpenCodeError) -> Self {
        Self {
            user: error.user_message(),
            detail: error.to_string(),
        }
    }
}

impl From<UpdateError> for UiError {
    fn from(error: UpdateError) -> Self {
        Self {
            user: error.user_message().to_owned(),
            detail: error.to_string(),
        }
    }
}

/// Credential-form and keyring state.
#[derive(Default)]
pub struct CredentialState {
    /// Whether the initial keyring check is running.
    pub checking: bool,
    /// Whether an OpenCode Go auth cookie is available.
    pub opencode: bool,
    /// OpenCode Go Workspace ID form value.
    pub opencode_workspace: String,
    /// OpenCode Go auth cookie form value.
    pub opencode_cookie: String,
    /// Helper text shown under the OpenCode credential form.
    pub login_notice: Option<String>,
    /// Whether a save or clear operation is running.
    pub mutating: bool,
    /// Latest credential-specific error.
    pub error: Option<UiError>,
}

/// Shared usage request and response state.
#[derive(Default)]
pub struct UsageState {
    /// Latest successfully parsed report.
    pub report: Option<UsageReport>,
    /// Latest raw response, held only in memory.
    pub raw: Option<String>,
    /// Whether a parsed report request is in flight.
    pub loading: bool,
    /// Whether a raw request is in flight.
    pub raw_loading: bool,
    /// Latest request error while retaining any previous report.
    pub error: Option<UiError>,
    /// Current display clock used for reset countdowns.
    pub now_ms: i64,
}

impl UsageState {
    /// Creates empty usage state with a current display clock.
    pub fn new() -> Self {
        Self {
            now_ms: chrono::Utc::now().timestamp_millis(),
            ..Self::default()
        }
    }
}

/// Editable settings overlay state.
pub struct SettingsState {
    /// Whether the settings overlay is visible.
    pub open: bool,
    /// Polling interval input.
    pub interval: String,
    /// Five-hour threshold input.
    pub five_hour: String,
    /// Weekly threshold input.
    pub weekly: String,
    /// Monthly threshold input.
    pub monthly: String,
    /// Whether a config write is in progress.
    pub saving: bool,
    /// Latest validation or write error.
    pub error: Option<UiError>,
    /// Latest successful operation message.
    pub notice: Option<String>,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            open: false,
            interval: "300".to_owned(),
            five_hour: "80".to_owned(),
            weekly: "85".to_owned(),
            monthly: "85".to_owned(),
            saving: false,
            error: None,
            notice: None,
        }
    }
}

/// Monitoring notification and deduplication state.
#[derive(Default)]
pub struct MonitorState {
    /// Last alerted subscription cycle per quota window.
    pub last_alerted: HashMap<String, i64>,
    /// Latest notification delivery error.
    pub notification_error: Option<UiError>,
}

/// Transient floating-window persistence state.
#[derive(Default)]
pub struct FloatState {
    /// Whether a move event is waiting for debounced persistence.
    pub position_dirty: bool,
    /// Whether the floating window is temporarily snapped to the monitor top.
    pub top_docked: bool,
}

/// Transient overlays and user feedback.
#[derive(Default)]
pub struct UiState {
    /// Whether the raw-response overlay is visible.
    pub debug_open: bool,
    /// Short user feedback shown without blocking interaction.
    pub toast: Option<String>,
    /// Whether the delete-credential confirmation modal is open.
    pub confirm_clear_credentials: bool,
    /// Keyboard focus used by the compact header action group.
    pub header_focus: Option<HeaderAction>,
}

/// Lifecycle of the updater state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UpdateStatus {
    /// Nothing has been checked or downloaded.
    #[default]
    Idle,
    /// A manifest check is in flight.
    Checking,
    /// A newer stable version is published and (if not auto-downloaded)
    /// waiting for the user to start the download.
    Available,
    /// The package is being downloaded.
    Downloading,
    /// The package is verified and waiting for user confirmation to install.
    ReadyToInstall,
    /// The platform installer is being launched.
    Installing,
    /// The running version is current.
    UpToDate,
    /// The last operation failed; `error` carries the details.
    Error,
}

impl UpdateStatus {
    /// Whether an update request should be refused to avoid concurrency.
    pub fn busy(self) -> bool {
        matches!(self, Self::Checking | Self::Downloading | Self::Installing)
    }
}

/// Download progress reported by the streaming downloader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateProgress {
    /// Bytes downloaded so far.
    pub downloaded: u64,
    /// Total bytes when known.
    pub total: Option<u64>,
}

/// Non-critical update state kept entirely separate from quota monitoring.
#[derive(Debug, Default)]
pub struct UpdateState {
    /// Current lifecycle status.
    pub status: UpdateStatus,
    /// The published update, if any.
    pub available: Option<UpdateInfo>,
    /// The verified package once the download completes.
    pub downloaded: Option<VerifiedPackage>,
    /// Latest download progress for the settings progress bar.
    pub progress: Option<UpdateProgress>,
    /// Latest updater error, shown only in settings.
    pub error: Option<UiError>,
    /// When the last check finished, in epoch milliseconds.
    pub last_checked_at: Option<i64>,
    /// Whether the dashboard banner was dismissed for this run.
    pub banner_dismissed: bool,
}

impl UpdateState {
    /// Whether the dashboard banner should be visible.
    pub fn banner_visible(&self) -> bool {
        !self.banner_dismissed
            && matches!(
                self.status,
                UpdateStatus::Available | UpdateStatus::Downloading | UpdateStatus::ReadyToInstall
            )
    }

    /// Applies the outcome of a manifest check.
    ///
    /// Returns `true` when an auto-download should be started because an
    /// update is available and auto-download is enabled.
    pub fn apply_check_result(
        &mut self,
        now_ms: i64,
        result: Result<Option<UpdateInfo>, UiError>,
        auto_download: bool,
    ) -> bool {
        self.last_checked_at = Some(now_ms);
        self.error = None;
        match result {
            Ok(Some(info)) => {
                self.available = Some(info);
                self.status = UpdateStatus::Available;
                auto_download
            }
            Ok(None) => {
                self.status = UpdateStatus::UpToDate;
                self.available = None;
                self.downloaded = None;
                false
            }
            Err(error) => {
                self.status = UpdateStatus::Error;
                self.error = Some(error);
                false
            }
        }
    }

    /// Moves from `Available` into a running download.
    pub fn begin_download(&mut self) {
        self.status = UpdateStatus::Downloading;
        self.error = None;
        self.progress = None;
    }

    /// Applies the outcome of a package download.
    pub fn apply_download_result(&mut self, result: Result<VerifiedPackage, UiError>) {
        self.progress = None;
        match result {
            Ok(package) => {
                self.downloaded = Some(package);
                self.status = UpdateStatus::ReadyToInstall;
            }
            Err(error) => {
                self.error = Some(error);
                self.status = UpdateStatus::Error;
            }
        }
    }
}

#[cfg(test)]
mod update_state_tests {
    use super::*;
    use crate::update::manifest::PackageType;
    use semver::Version;

    fn update_info() -> UpdateInfo {
        UpdateInfo {
            version: Version::parse("0.2.0").expect("version"),
            tag: "v0.2.0".to_owned(),
            release_notes_url: "https://example.com/releases/tag/v0.2.0".to_owned(),
            platform: crate::update::manifest::UpdatePlatform {
                kind: PackageType::Nsis,
                url: "https://github.com/example/releases/download/v0.2.0/opencode-quota-checker-windows-x86_64.exe".to_owned(),
                sha256: "abc".to_owned(),
            },
        }
    }

    fn verified() -> VerifiedPackage {
        VerifiedPackage {
            path: std::path::PathBuf::from(
                "C:/cache/opencode-quota-checker/update/opencode-quota-checker-windows-x86_64.exe",
            ),
            kind: PackageType::Nsis,
            version: "0.2.0".to_owned(),
        }
    }

    fn error() -> UiError {
        UiError {
            user: "暂时无法检查更新，请稍后重试。".to_owned(),
            detail: "network down".to_owned(),
        }
    }

    #[test]
    fn idle_checking_up_to_date() {
        let mut state = UpdateState::default();
        assert_eq!(state.status, UpdateStatus::Idle);
        state.status = UpdateStatus::Checking;
        let started = state.apply_check_result(1_000, Ok(None), true);
        assert!(!started);
        assert_eq!(state.status, UpdateStatus::UpToDate);
        assert_eq!(state.last_checked_at, Some(1_000));
    }

    #[test]
    fn available_auto_downloads_then_ready_to_install() {
        let mut state = UpdateState {
            status: UpdateStatus::Checking,
            ..UpdateState::default()
        };
        let started = state.apply_check_result(1_000, Ok(Some(update_info())), true);
        assert!(started, "auto-download should start");
        assert_eq!(state.status, UpdateStatus::Available);
        state.begin_download();
        assert_eq!(state.status, UpdateStatus::Downloading);
        state.apply_download_result(Ok(verified()));
        assert_eq!(state.status, UpdateStatus::ReadyToInstall);
        assert!(state.downloaded.is_some());
    }

    #[test]
    fn manual_available_does_not_auto_download() {
        let mut state = UpdateState {
            status: UpdateStatus::Checking,
            ..UpdateState::default()
        };
        let started = state.apply_check_result(1_000, Ok(Some(update_info())), false);
        assert!(!started);
        assert_eq!(state.status, UpdateStatus::Available);
    }

    #[test]
    fn download_error_can_be_retried() {
        let mut state = UpdateState {
            status: UpdateStatus::Checking,
            ..UpdateState::default()
        };
        state.apply_check_result(1_000, Ok(Some(update_info())), false);
        state.begin_download();
        state.apply_download_result(Err(error()));
        assert_eq!(state.status, UpdateStatus::Error);
        assert!(state.error.is_some());
        assert!(state.downloaded.is_none());

        state.begin_download();
        assert_eq!(state.status, UpdateStatus::Downloading);
        assert!(state.error.is_none());
    }

    #[test]
    fn banner_shows_for_available_and_is_dismissible() {
        let mut state = UpdateState {
            status: UpdateStatus::Checking,
            ..UpdateState::default()
        };
        state.apply_check_result(1_000, Ok(Some(update_info())), false);
        assert!(state.banner_visible());
        state.banner_dismissed = true;
        assert!(!state.banner_visible());
    }
}

use opencode_core::{OpenCodeError, Thresholds};
use serde::{Deserialize, Serialize};

/// Current configuration schema version.
pub const SCHEMA_VERSION: u32 = 2;

/// Behavior when the main window receives a close request.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CloseBehavior {
    /// Keep the daemon alive and recover it from the tray.
    #[default]
    MinimizeToTray,
    /// Exit the process.
    Exit,
}

/// Floating-window presentation mode.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum FloatMode {
    /// All quota windows and controls.
    #[default]
    Full,
    /// Highest-risk quota summary.
    Compact,
    /// Single-line docked status.
    Docked,
}

impl FloatMode {
    /// Returns the native inner size for the mode.
    pub fn size(self) -> iced::Size {
        match self {
            Self::Full => iced::Size::new(360.0, 420.0),
            Self::Compact => iced::Size::new(360.0, 148.0),
            Self::Docked => iced::Size::new(360.0, 56.0),
        }
    }
}

/// Persisted screen position of the floating window.
///
/// Windows uses physical virtual-desktop pixels so the position remains stable
/// across monitors with different DPI. Other platforms use Iced logical pixels.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct FloatPosition {
    /// Platform-native horizontal coordinate.
    pub x: i32,
    /// Platform-native vertical coordinate.
    pub y: i32,
}

/// Persisted, non-sensitive application configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AppConfig {
    /// Configuration schema version.
    pub schema_version: u32,
    /// Whether subscription-based monitoring is enabled.
    pub monitor_enabled: bool,
    /// Polling interval in seconds.
    pub monitor_interval_secs: u64,
    /// Per-window notification thresholds.
    pub thresholds: Thresholds,
    /// OpenCode Go workspace identifier (non-sensitive).
    pub opencode_workspace_id: Option<String>,
    /// Main window close behavior.
    pub close_behavior: CloseBehavior,
    /// Whether the floating window should be restored on launch.
    pub float_open: bool,
    /// Floating-window layout mode.
    pub float_mode: FloatMode,
    /// Last known floating-window position.
    pub float_position: Option<FloatPosition>,
    /// Whether to check GitHub Releases for new versions on startup and
    /// periodically. Non-critical; never blocks quota monitoring.
    pub update_checks_enabled: bool,
    /// Whether discovered stable updates download immediately. A user
    /// confirmation is always required before anything is installed.
    pub auto_download_updates: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            monitor_enabled: true,
            monitor_interval_secs: 300,
            thresholds: Thresholds::default(),
            opencode_workspace_id: None,
            close_behavior: CloseBehavior::MinimizeToTray,
            float_open: false,
            float_mode: FloatMode::Full,
            float_position: None,
            update_checks_enabled: true,
            auto_download_updates: true,
        }
    }
}

impl AppConfig {
    /// Validates and normalizes persisted settings.
    pub fn validate(mut self) -> Result<Self, OpenCodeError> {
        if !(30..=3_600).contains(&self.monitor_interval_secs) {
            return Err(OpenCodeError::Config(
                "monitor interval must be between 30 and 3600 seconds".to_owned(),
            ));
        }
        self.thresholds = self.thresholds.validate()?;
        self.schema_version = SCHEMA_VERSION;
        Ok(self)
    }
}

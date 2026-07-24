use serde::{Deserialize, Serialize};
use volc_core::{Thresholds, VolcError};

/// Current configuration schema version.
pub const SCHEMA_VERSION: u32 = 1;

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
            Self::Full => iced::Size::new(344.0, 404.0),
            Self::Compact => iced::Size::new(344.0, 128.0),
            Self::Docked => iced::Size::new(344.0, 52.0),
        }
    }
}

/// Persisted logical screen position of the floating window.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct FloatPosition {
    /// Logical horizontal coordinate.
    pub x: i32,
    /// Logical vertical coordinate.
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
    /// Main window close behavior.
    pub close_behavior: CloseBehavior,
    /// Whether a legacy settings file has been imported.
    pub legacy_migration_complete: bool,
    /// Whether the floating window should be restored on launch.
    pub float_open: bool,
    /// Floating-window layout mode.
    pub float_mode: FloatMode,
    /// Last known logical floating-window position.
    pub float_position: Option<FloatPosition>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            monitor_enabled: true,
            monitor_interval_secs: 300,
            thresholds: Thresholds::default(),
            close_behavior: CloseBehavior::MinimizeToTray,
            legacy_migration_complete: false,
            float_open: false,
            float_mode: FloatMode::Full,
            float_position: None,
        }
    }
}

impl AppConfig {
    /// Validates and normalizes persisted settings.
    pub fn validate(mut self) -> Result<Self, VolcError> {
        if !(30..=3_600).contains(&self.monitor_interval_secs) {
            return Err(VolcError::Config(
                "monitor interval must be between 30 and 3600 seconds".to_owned(),
            ));
        }
        self.thresholds = self.thresholds.validate()?;
        self.schema_version = SCHEMA_VERSION;
        Ok(self)
    }
}

use opencode_core::{OpenCodeError, Thresholds};
use serde::{Deserialize, Serialize};

/// Current configuration schema version.
///
/// Kept at 2 for the first Tauri release: the schema is identical to the
/// archived Iced client so existing `config.json` files load unchanged.
/// A future schema bump must implement an explicit migration in `store.rs`.
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
///
/// `Full` and `Compact` are persisted configured modes; `Docked` is a
/// transient presentation entered by snapping to the monitor top and is
/// never written to disk (`AppConfig::validate` normalizes it away).
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

/// Persisted screen position of the floating window.
///
/// Stored in physical pixels on every platform so the position remains stable
/// across monitors with different DPI (matches the archived Windows client,
/// which persisted native virtual-desktop pixels).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct FloatPosition {
    /// Physical horizontal coordinate.
    pub x: i32,
    /// Physical vertical coordinate.
    pub y: i32,
}

/// Persisted, non-sensitive application configuration.
///
/// The auth cookie is never part of this structure; it lives in the system
/// keyring only.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AppConfig {
    /// Configuration schema version.
    pub schema_version: u32,
    /// Whether background monitoring is enabled.
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
    ///
    /// This is the single validation gate for both the config file and every
    /// IPC write: frontend validation is only UX, the final check is here.
    pub fn validate(mut self) -> Result<Self, OpenCodeError> {
        if !(30..=3_600).contains(&self.monitor_interval_secs) {
            return Err(OpenCodeError::Config(
                "monitor interval must be between 30 and 3600 seconds".to_owned(),
            ));
        }
        self.thresholds = self.thresholds.validate()?;
        // `Docked` is a transient presentation, never a persisted mode.
        if self.float_mode == FloatMode::Docked {
            self.float_mode = FloatMode::Compact;
        }
        self.schema_version = SCHEMA_VERSION;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid() {
        let config = AppConfig::default().validate().expect("defaults are valid");
        assert_eq!(config.schema_version, SCHEMA_VERSION);
        assert_eq!(config.monitor_interval_secs, 300);
    }

    #[test]
    fn invalid_interval_is_rejected() {
        let config = AppConfig {
            monitor_interval_secs: 10,
            ..AppConfig::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn invalid_thresholds_are_rejected() {
        let config = AppConfig {
            thresholds: Thresholds {
                five_hour: f64::NAN,
                ..Thresholds::default()
            },
            ..AppConfig::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn docked_mode_is_normalized_to_compact() {
        let config = AppConfig {
            float_mode: FloatMode::Docked,
            ..AppConfig::default()
        };
        assert_eq!(
            config.validate().expect("valid").float_mode,
            FloatMode::Compact
        );
    }
}

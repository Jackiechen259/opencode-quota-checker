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

/// Persisted floating-window layout.
///
/// Only the two user-selectable layouts live here. `Docked` is a transient
/// presentation entered by snapping the window to the monitor top; it is
/// never persisted, so it deliberately has no variant in this type.
///
/// Legacy configuration files may still contain `"docked"` (the old
/// three-variant enum). The alias maps it to [`FloatLayout::Compact`], which
/// matches the historical `validate` normalization.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum FloatLayout {
    /// All quota windows and controls.
    #[default]
    Full,
    /// Highest-risk quota summary.
    #[serde(alias = "docked")]
    Compact,
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
    /// OpenCode workspace identifier, sent as `x-opencode-org-id` (non-sensitive).
    ///
    /// Optional: a Console API key already targets the workspace it was created
    /// in, so this is only an override (and can be recognised by the old login
    /// page URL, `opencode.ai/workspace/<id>/go`).
    pub opencode_workspace_id: Option<String>,
    /// Main window close behavior.
    pub close_behavior: CloseBehavior,
    /// Whether the floating window should be restored on launch.
    pub float_open: bool,
    /// Floating-window layout mode.
    pub float_mode: FloatLayout,
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
            float_mode: FloatLayout::Full,
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
        self.schema_version = SCHEMA_VERSION;
        Ok(self)
    }

    /// Restores the floating-window-owned fields from `current`.
    ///
    /// `float_open`, `float_mode` and `float_position` are mutated by the
    /// floating-window controller only. Settings/round-trip saves must not
    /// overwrite them with a possibly stale frontend snapshot.
    pub fn preserve_float_fields(&mut self, current: &Self) {
        self.float_open = current.float_open;
        self.float_mode = current.float_mode;
        self.float_position = current.float_position;
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
    fn legacy_docked_float_mode_deserializes_as_compact() {
        // Schema 2 files written by the old three-variant enum may contain
        // `"docked"`; it must keep loading, normalized to the persisted
        // Compact layout, without requiring a schema bump.
        let config: AppConfig =
            serde_json::from_str(r#"{"float_mode":"docked"}"#).expect("legacy config loads");
        assert_eq!(config.float_mode, FloatLayout::Compact);
    }

    #[test]
    fn float_layout_serializes_without_the_transient_docked_variant() {
        let config = AppConfig {
            float_mode: FloatLayout::Compact,
            ..AppConfig::default()
        };
        let json = serde_json::to_string(&config).expect("config serializes");
        assert!(json.contains(r#""float_mode":"compact""#));
    }

    #[test]
    fn preserve_float_fields_keeps_controller_owned_state() {
        let current = AppConfig {
            float_open: true,
            float_mode: FloatLayout::Compact,
            float_position: Some(FloatPosition { x: 12, y: 34 }),
            ..AppConfig::default()
        };
        let mut incoming = AppConfig {
            float_open: false,
            float_mode: FloatLayout::Full,
            float_position: None,
            monitor_interval_secs: 600,
            ..AppConfig::default()
        };

        incoming.preserve_float_fields(&current);

        assert!(incoming.float_open);
        assert_eq!(incoming.float_mode, FloatLayout::Compact);
        assert_eq!(incoming.float_position, current.float_position);
        assert_eq!(
            incoming.monitor_interval_secs, 600,
            "non-float settings must still be applied"
        );
    }
}

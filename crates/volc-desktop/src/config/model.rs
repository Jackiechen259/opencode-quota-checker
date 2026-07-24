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

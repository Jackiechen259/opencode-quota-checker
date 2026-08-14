//! JSON configuration store in the platform-standard user config directory.
//!
//! The path and format are identical to the archived Iced client
//! (`opencode-quota-checker/config.json`), so an existing installation keeps
//! its settings across the upgrade. Writes are atomic: a same-directory
//! temporary file is fsynced and renamed over the target.

use crate::config::model::AppConfig;
use directories::BaseDirs;
use opencode_core::OpenCodeError;
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;

/// JSON configuration store in the platform-standard user config directory.
#[derive(Debug, Clone)]
pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    /// Discovers the platform-standard configuration path.
    pub fn discover() -> Result<Self, OpenCodeError> {
        let base = BaseDirs::new().ok_or_else(|| {
            OpenCodeError::Config("user configuration directory is unavailable".into())
        })?;
        Ok(Self {
            path: base
                .config_dir()
                .join("opencode-quota-checker")
                .join("config.json"),
        })
    }

    #[cfg(test)]
    fn at(path: PathBuf) -> Self {
        Self { path }
    }

    /// Loads the current configuration or writes a fresh default.
    pub fn load_or_default(&self) -> Result<AppConfig, OpenCodeError> {
        if self.path.exists() {
            return self.load();
        }
        let config = AppConfig::default();
        self.save(&config)?;
        Ok(config)
    }

    /// Loads and validates the current JSON file.
    pub fn load(&self) -> Result<AppConfig, OpenCodeError> {
        let file = File::open(&self.path)
            .map_err(|error| OpenCodeError::Config(format!("cannot open config: {error}")))?;
        let config = serde_json::from_reader::<_, AppConfig>(BufReader::new(file))
            .map_err(|error| OpenCodeError::Config(format!("cannot parse config: {error}")))?;
        config.validate()
    }

    /// Writes configuration through a same-directory temporary file and rename.
    pub fn save(&self, config: &AppConfig) -> Result<(), OpenCodeError> {
        let config = config.clone().validate()?;
        let parent = self
            .path
            .parent()
            .ok_or_else(|| OpenCodeError::Config("configuration path has no parent".into()))?;
        fs::create_dir_all(parent).map_err(|error| {
            OpenCodeError::Config(format!("cannot create config directory: {error}"))
        })?;
        let temp = self
            .path
            .with_extension(format!("json.{}.tmp", std::process::id()));
        if temp.exists() {
            fs::remove_file(&temp).map_err(|error| {
                OpenCodeError::Config(format!("cannot clear stale temp config: {error}"))
            })?;
        }
        let result = (|| {
            let file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)
                .map_err(|error| {
                    OpenCodeError::Config(format!("cannot create temp config: {error}"))
                })?;
            let mut writer = BufWriter::new(file);
            serde_json::to_writer_pretty(&mut writer, &config).map_err(|error| {
                OpenCodeError::Config(format!("cannot serialize config: {error}"))
            })?;
            writer.flush().map_err(|error| {
                OpenCodeError::Config(format!("cannot flush temp config: {error}"))
            })?;
            writer
                .into_inner()
                .map_err(|error| {
                    OpenCodeError::Config(format!("cannot finalize temp config: {error}"))
                })?
                .sync_all()
                .map_err(|error| {
                    OpenCodeError::Config(format!("cannot sync temp config: {error}"))
                })?;
            fs::rename(&temp, &self.path)
                .map_err(|error| OpenCodeError::Config(format!("cannot replace config: {error}")))
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::SCHEMA_VERSION;

    #[test]
    fn saves_and_loads_without_sensitive_fields() {
        let directory = tempfile::tempdir().expect("temporary directory is available");
        let path = directory.path().join("config.json");
        let store = ConfigStore::at(path.clone());
        store.save(&AppConfig::default()).expect("config saves");
        let updated = AppConfig {
            monitor_enabled: false,
            ..AppConfig::default()
        };
        store.save(&updated).expect("config atomically replaces");
        assert_eq!(store.load().expect("config loads"), updated);
        let raw = fs::read_to_string(path).expect("saved config is readable");
        assert!(!raw.to_ascii_lowercase().contains("secret"));
        assert!(!raw.contains("auth_cookie"));
    }

    #[test]
    fn missing_config_is_replaced_by_defaults() {
        let directory = tempfile::tempdir().expect("temporary directory is available");
        let path = directory.path().join("config.json");
        let store = ConfigStore::at(path);
        let config = store.load_or_default().expect("defaults are written");
        assert_eq!(config, AppConfig::default());
    }

    #[test]
    fn invalid_interval_is_rejected() {
        let config = AppConfig {
            monitor_interval_secs: 1,
            ..AppConfig::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn schema_1_config_migrates_with_new_update_defaults() {
        let directory = tempfile::tempdir().expect("temporary directory is available");
        let path = directory.path().join("config.json");
        // A pre-update config file: no update fields, old schema number.
        fs::write(&path, r#"{"schema_version":1,"monitor_enabled":true}"#)
            .expect("legacy config is written");
        let store = ConfigStore::at(path);

        let config = store.load().expect("legacy config loads");
        assert_eq!(config.schema_version, SCHEMA_VERSION);
        assert!(config.monitor_enabled);
        assert!(
            config.update_checks_enabled,
            "new checks default to enabled"
        );
        assert!(
            config.auto_download_updates,
            "new auto-download defaults to enabled"
        );
        assert_eq!(config.monitor_interval_secs, 300);
    }

    #[test]
    fn schema_2_config_keeps_every_existing_field() {
        let directory = tempfile::tempdir().expect("temporary directory is available");
        let path = directory.path().join("config.json");
        fs::write(
            &path,
            r#"{
                "schema_version": 2,
                "monitor_enabled": false,
                "monitor_interval_secs": 600,
                "thresholds": { "five_hour": 75.0, "weekly": 80.0, "monthly": 90.0 },
                "opencode_workspace_id": "workspace-test-123",
                "close_behavior": "minimize_to_tray",
                "float_open": true,
                "float_mode": "full",
                "float_position": { "x": 100, "y": 200 },
                "update_checks_enabled": false,
                "auto_download_updates": false
            }"#,
        )
        .expect("legacy config is written");
        let store = ConfigStore::at(path);

        let config = store.load().expect("schema 2 config loads unchanged");
        assert_eq!(config.schema_version, 2);
        assert!(!config.monitor_enabled);
        assert_eq!(config.monitor_interval_secs, 600);
        assert_eq!(config.thresholds.five_hour, 75.0);
        assert_eq!(config.thresholds.weekly, 80.0);
        assert_eq!(config.thresholds.monthly, 90.0);
        assert_eq!(
            config.opencode_workspace_id.as_deref(),
            Some("workspace-test-123")
        );
        assert_eq!(config.close_behavior, crate::config::CloseBehavior::MinimizeToTray);
        assert!(config.float_open);
        assert_eq!(config.float_position, Some(crate::config::FloatPosition { x: 100, y: 200 }));
        assert!(!config.update_checks_enabled);
        assert!(!config.auto_download_updates);
    }

    #[test]
    fn workspace_id_round_trips_without_the_cookie() {
        let config = AppConfig {
            opencode_workspace_id: Some("workspace-test-123".to_owned()),
            ..AppConfig::default()
        };
        let encoded = serde_json::to_string(&config).expect("config serializes");
        assert!(encoded.contains("workspace-test-123"));
        assert!(!encoded.contains("auth_cookie"));

        let decoded: AppConfig = serde_json::from_str(&encoded).expect("config decodes");
        assert_eq!(
            decoded.opencode_workspace_id.as_deref(),
            Some("workspace-test-123")
        );
    }
}

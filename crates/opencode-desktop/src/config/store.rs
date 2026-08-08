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

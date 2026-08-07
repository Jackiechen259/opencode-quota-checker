use crate::config::model::{AppConfig, FloatMode};
use directories::BaseDirs;
use serde_json::Value;
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use volc_core::{Thresholds, VolcError};

/// JSON configuration store in the platform-standard user config directory.
#[derive(Debug, Clone)]
pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    /// Discovers the platform-standard configuration path.
    pub fn discover() -> Result<Self, VolcError> {
        let base = BaseDirs::new().ok_or_else(|| {
            VolcError::Config("user configuration directory is unavailable".into())
        })?;
        Ok(Self {
            path: base.config_dir().join("volc-status").join("config.json"),
        })
    }

    #[cfg(test)]
    fn at(path: PathBuf) -> Self {
        Self { path }
    }

    /// Loads current configuration or imports compatible legacy settings.
    pub fn load_or_migrate(&self) -> Result<AppConfig, VolcError> {
        if self.path.exists() {
            return self.load();
        }
        if let Some(mut migrated) = self.load_legacy()? {
            migrated.legacy_migration_complete = true;
            self.save(&migrated)?;
            return Ok(migrated);
        }
        let config = AppConfig::default();
        self.save(&config)?;
        Ok(config)
    }

    /// Loads and validates the current JSON file.
    pub fn load(&self) -> Result<AppConfig, VolcError> {
        let file = File::open(&self.path)
            .map_err(|error| VolcError::Config(format!("cannot open config: {error}")))?;
        let config = serde_json::from_reader::<_, AppConfig>(BufReader::new(file))
            .map_err(|error| VolcError::Config(format!("cannot parse config: {error}")))?;
        config.validate()
    }

    /// Writes configuration through a same-directory temporary file and rename.
    pub fn save(&self, config: &AppConfig) -> Result<(), VolcError> {
        let config = config.clone().validate()?;
        let parent = self
            .path
            .parent()
            .ok_or_else(|| VolcError::Config("configuration path has no parent".into()))?;
        fs::create_dir_all(parent).map_err(|error| {
            VolcError::Config(format!("cannot create config directory: {error}"))
        })?;
        let temp = self
            .path
            .with_extension(format!("json.{}.tmp", std::process::id()));
        if temp.exists() {
            fs::remove_file(&temp).map_err(|error| {
                VolcError::Config(format!("cannot clear stale temp config: {error}"))
            })?;
        }
        let result = (|| {
            let file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)
                .map_err(|error| {
                    VolcError::Config(format!("cannot create temp config: {error}"))
                })?;
            let mut writer = BufWriter::new(file);
            serde_json::to_writer_pretty(&mut writer, &config)
                .map_err(|error| VolcError::Config(format!("cannot serialize config: {error}")))?;
            writer
                .flush()
                .map_err(|error| VolcError::Config(format!("cannot flush temp config: {error}")))?;
            writer
                .into_inner()
                .map_err(|error| {
                    VolcError::Config(format!("cannot finalize temp config: {error}"))
                })?
                .sync_all()
                .map_err(|error| VolcError::Config(format!("cannot sync temp config: {error}")))?;
            fs::rename(&temp, &self.path)
                .map_err(|error| VolcError::Config(format!("cannot replace config: {error}")))
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        result
    }

    fn load_legacy(&self) -> Result<Option<AppConfig>, VolcError> {
        for candidate in legacy_candidates(&self.path) {
            if !candidate.exists() {
                continue;
            }
            let file = File::open(&candidate).map_err(|error| {
                VolcError::Config(format!("cannot open legacy config: {error}"))
            })?;
            let value: Value = serde_json::from_reader(BufReader::new(file)).map_err(|error| {
                VolcError::Config(format!("cannot parse legacy config: {error}"))
            })?;
            return legacy_to_config(&value).map(Some);
        }
        Ok(None)
    }
}

fn legacy_candidates(new_path: &Path) -> Vec<PathBuf> {
    let Some(config_root) = new_path.parent().and_then(Path::parent) else {
        return Vec::new();
    };
    vec![
        config_root.join("com.volcstatus.app").join("settings.json"),
        config_root.join("VOLC Status").join("settings.json"),
    ]
}

fn legacy_to_config(value: &Value) -> Result<AppConfig, VolcError> {
    let mut config = AppConfig::default();
    if let Some(enabled) = value.get("monitor_enabled").and_then(Value::as_bool) {
        config.monitor_enabled = enabled;
    }
    if let Some(interval) = value.get("monitor_interval").and_then(Value::as_u64) {
        config.monitor_interval_secs = interval;
    }
    if let Some(thresholds) = value.get("monitor_thresholds") {
        config.thresholds = serde_json::from_value::<Thresholds>(thresholds.clone())
            .map_err(|error| VolcError::Config(format!("invalid legacy thresholds: {error}")))?;
    }
    if let Some(float_open) = value.get("float_open").and_then(Value::as_bool) {
        config.float_open = float_open;
    }
    if value
        .get("float_compact")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        config.float_mode = FloatMode::Compact;
    }
    config.validate()
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
        assert!(!raw.contains("\"ak\""));
    }

    #[test]
    fn legacy_monitor_fields_are_migrated() {
        let value = serde_json::json!({
            "monitor_enabled": false,
            "monitor_interval": 120,
            "monitor_thresholds": {
                "five_hour": 70.0,
                "weekly": 75.0,
                "monthly": 80.0
            }
        });
        let config = legacy_to_config(&value).expect("legacy config is compatible");
        assert!(!config.monitor_enabled);
        assert_eq!(config.monitor_interval_secs, 120);
        assert_eq!(config.thresholds.five_hour, 70.0);
    }

    #[test]
    fn invalid_interval_is_rejected() {
        let config = AppConfig {
            monitor_interval_secs: 1,
            ..AppConfig::default()
        };
        assert!(config.validate().is_err());
    }
}

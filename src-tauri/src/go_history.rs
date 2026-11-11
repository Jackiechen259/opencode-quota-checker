//! Local, persisted daily Go usage history for the year heatmap.
//!
//! The Console activity endpoint only reports the current paid period (about
//! 30 days), so a GitHub-style year grid needs the application to remember what
//! it has already seen. Every successful Go activity fetch merges its daily
//! rows into this store; a row for an existing date is overwritten because the
//! upstream value for a day is cumulative (today keeps growing while the period
//! runs).
//!
//! Storage is a small JSON document in the platform config directory next to
//! `config.json`, written atomically (same-directory temporary file, fsync,
//! rename) exactly like the configuration store. The store is a leaf resource:
//! [`GoHistory`] never acquires another `AppState` lock, and callers must drop
//! the `go_history` guard before any other lock or file IO (see the lock
//! ordering rules on `AppState`).
//!
//! Failures are never fatal: a missing file is a normal first run, a corrupt
//! file is logged and starts empty, and a failed write is logged and retried on
//! the next successful activity fetch.

use chrono::{Duration, NaiveDate, Utc};
use opencode_core::GoActivityDay;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

/// Days retained on disk; the grid renders the last 53 weeks (~371 days).
pub const HISTORY_DAYS: i64 = 400;

/// Current on-disk schema version; an unknown version starts an empty history.
const HISTORY_VERSION: u32 = 1;

/// File name inside the `opencode-quota-checker` config directory.
const FILE_NAME: &str = "go-history.json";

/// Calendar date format used by the Console and by the file keys.
const DATE_FORMAT: &str = "%Y-%m-%d";

/// Persisted document.
#[derive(Debug, Deserialize, Serialize)]
struct HistoryFile {
    version: u32,
    days: Vec<GoActivityDay>,
}

/// In-memory daily history; `path` is `None` for a detached store (tests and
/// any failure to discover the config directory), which never touches disk.
#[derive(Debug, Clone, Default)]
pub struct GoHistory {
    path: Option<PathBuf>,
    days: BTreeMap<String, GoActivityDay>,
}

impl GoHistory {
    /// Loads the standard history file, or returns an empty store when it is
    /// missing, unreadable, corrupt, or of an unknown version.
    pub fn load_default() -> Self {
        let Some(path) = default_path() else {
            tracing::warn!("user config directory is unavailable; Go history is memory-only");
            return Self::default();
        };
        Self::load_at(path)
    }

    /// Loads one specific file; used by [`Self::load_default`] and the tests.
    fn load_at(path: PathBuf) -> Self {
        let mut history = Self {
            path: Some(path),
            days: BTreeMap::new(),
        };
        match history.read() {
            Ok(()) => tracing::info!(days = history.days.len(), "Go usage history loaded"),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                tracing::warn!(%error, "failed to load Go usage history; starting empty");
            }
        }
        history
    }

    /// Reads and prunes the file backing this store.
    fn read(&mut self) -> std::io::Result<()> {
        let path = self
            .path
            .as_ref()
            .ok_or_else(|| invalid("history has no path"))?;
        let file = File::open(path)?;
        let document: HistoryFile = serde_json::from_reader(BufReader::new(file))
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        if document.version != HISTORY_VERSION {
            tracing::warn!(
                version = document.version,
                "unknown Go usage history version; starting empty"
            );
            return Ok(());
        }
        for day in document.days {
            self.days.insert(day.date.clone(), day);
        }
        let today = today_utc();
        self.prune(
            today - Duration::days(HISTORY_DAYS - 1),
            today + Duration::days(1),
        );
        Ok(())
    }

    /// Merges upstream daily rows and prunes everything outside the window.
    ///
    /// Rows with an invalid or out-of-window date are ignored rather than
    /// rejected: one bad row must not discard the rest of the response. The
    /// values that do land replace any previous value for the same date.
    pub fn record(&mut self, days: &[GoActivityDay], today: NaiveDate) {
        let earliest = today - Duration::days(HISTORY_DAYS - 1);
        let latest = today + Duration::days(1);
        for day in days {
            let Ok(date) = NaiveDate::parse_from_str(&day.date, DATE_FORMAT) else {
                continue;
            };
            if date < earliest || date > latest {
                continue;
            }
            self.days.insert(day.date.clone(), day.clone());
        }
        self.prune(earliest, latest);
    }

    /// Drops rows whose date is missing, malformed, or outside the window.
    fn prune(&mut self, earliest: NaiveDate, latest: NaiveDate) {
        self.days.retain(|key, _| {
            NaiveDate::parse_from_str(key, DATE_FORMAT)
                .is_ok_and(|date| date >= earliest && date <= latest)
        });
    }

    /// Snapshot for the IPC DTO, oldest first.
    pub fn entries(&self) -> Vec<GoActivityDay> {
        self.days.values().cloned().collect()
    }

    /// Whether nothing has been recorded yet.
    pub fn is_empty(&self) -> bool {
        self.days.is_empty()
    }

    /// Writes the store atomically; failures are logged, never propagated.
    pub fn save(&self) {
        let Some(path) = self.path.as_ref() else {
            return;
        };
        if let Err(error) = self.write(path) {
            tracing::warn!(%error, "failed to persist Go usage history");
        }
    }

    /// Atomic write: temp file in the same directory, fsync, rename.
    fn write(&self, path: &Path) -> std::io::Result<()> {
        let parent = path
            .parent()
            .ok_or_else(|| invalid("history path has no parent directory"))?;
        fs::create_dir_all(parent)?;
        let temp = path.with_extension(format!("json.{}.tmp", std::process::id()));
        if temp.exists() {
            let _ = fs::remove_file(&temp);
        }
        let result = (|| {
            let file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)?;
            let mut writer = BufWriter::new(file);
            let document = HistoryFile {
                version: HISTORY_VERSION,
                days: self.entries(),
            };
            serde_json::to_writer_pretty(&mut writer, &document)
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
            writer.flush()?;
            writer
                .into_inner()
                .map_err(|error| error.into_error())?
                .sync_all()?;
            fs::rename(&temp, path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        result
    }
}

/// Today's UTC calendar date; Console activity days are UTC dates.
fn today_utc() -> NaiveDate {
    Utc::now().date_naive()
}

/// Platform-standard history path, next to `config.json`.
fn default_path() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|base| {
        base.config_dir()
            .join("opencode-quota-checker")
            .join(FILE_NAME)
    })
}

fn invalid(message: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(date: &str, tokens: u64) -> GoActivityDay {
        GoActivityDay {
            date: date.to_owned(),
            cost_microcents: tokens / 2,
            requests: 1,
            tokens,
        }
    }

    fn date(value: &str) -> NaiveDate {
        NaiveDate::parse_from_str(value, DATE_FORMAT).expect("valid test date")
    }

    #[test]
    fn record_merges_sorts_and_overwrites_the_same_day() {
        let mut history = GoHistory::default();
        let today = date("2026-11-11");
        history.record(&[day("2026-11-10", 100), day("2026-11-11", 40)], today);
        // A later fetch of the same (still growing) day replaces the row.
        history.record(&[day("2026-11-11", 90)], today);

        let entries = history.entries();
        assert_eq!(
            entries
                .iter()
                .map(|day| day.date.as_str())
                .collect::<Vec<_>>(),
            ["2026-11-10", "2026-11-11"]
        );
        assert_eq!(entries[1].tokens, 90, "same-day row must be overwritten");
    }

    #[test]
    fn record_ignores_invalid_and_out_of_window_dates() {
        let mut history = GoHistory::default();
        let today = date("2026-11-11");
        history.record(
            &[
                day("not-a-date", 1),
                day("2026-02-30", 1),
                day("2024-01-01", 1), // stale
                day("2026-11-13", 1), // future beyond tomorrow
                day("2026-11-12", 1), // tomorrow is tolerated (UTC skew)
            ],
            today,
        );

        let entries = history.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].date, "2026-11-12");
    }

    #[test]
    fn record_prunes_days_older_than_the_retention_window() {
        let mut history = GoHistory::default();
        let today = date("2026-11-11");
        let cutoff = today - Duration::days(HISTORY_DAYS - 1);
        history.record(
            &[
                day(&(cutoff - Duration::days(1)).to_string(), 5),
                day(&cutoff.to_string(), 7),
            ],
            today,
        );

        let entries = history.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].tokens, 7);
    }

    #[test]
    fn save_and_load_round_trip() {
        let directory = tempfile::tempdir().expect("temporary directory is available");
        let path = directory.path().join(FILE_NAME);
        let today = date("2026-11-11");

        {
            let mut history = GoHistory::load_at(path.clone());
            history.record(&[day("2026-11-10", 100), day("2026-11-11", 40)], today);
            history.save();
        }
        assert!(path.exists(), "save must create the history file");

        let loaded = GoHistory::load_at(path);
        let entries = loaded.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].date, "2026-11-10");
        assert_eq!(entries[1].tokens, 40);
    }

    #[test]
    fn a_corrupt_or_unknown_file_starts_empty_but_stays_writable() {
        let directory = tempfile::tempdir().expect("temporary directory is available");
        let path = directory.path().join(FILE_NAME);
        fs::write(&path, "{not json").expect("write corrupt history");

        let mut history = GoHistory::load_at(path.clone());
        assert!(history.is_empty(), "corrupt file must not fail startup");

        history.record(&[day("2026-11-11", 10)], date("2026-11-11"));
        history.save();
        assert_eq!(
            GoHistory::load_at(path.clone()).entries().len(),
            1,
            "a corrupt file is replaced by the next successful save"
        );

        fs::write(&path, r#"{"version":99,"days":[]}"#).expect("write unknown version");
        assert!(GoHistory::load_at(path).is_empty());
    }

    #[test]
    fn a_detached_store_never_touches_disk() {
        let mut history = GoHistory::default();
        history.record(&[day("2026-11-11", 1)], date("2026-11-11"));
        history.save();
        assert_eq!(history.entries().len(), 1);
    }
}

//! Parses the OpenCode Go usage API into the shared quota model.
//!
//! `GET https://opencode.ai/zen/go/v1/usage` answers with one meter per quota
//! window:
//!
//! ```json
//! {
//!   "usage": {
//!     "rolling": { "status": "ok",           "percent": 78.0,  "resetsAt": "2026-10-27T05:31:52.000Z" },
//!     "weekly":  { "status": "ok",           "percent": 52.0,  "resetsAt": "2026-10-31T00:00:00.000Z" },
//!     "monthly": { "status": "rate-limited", "percent": 100.0, "resetsAt": "2026-11-01T00:00:00.000Z" }
//!   }
//! }
//! ```
//!
//! Mapping rules:
//!
//! * `percent` is the share of the window's allowance already used. OpenCode only
//!   reports percentages, so the shared 100-point model is kept (`quota = 100`,
//!   `used = percent`).
//! * `resetsAt` is an absolute RFC 3339 instant, so `reset_time` is no longer
//!   derived from a relative countdown plus the local clock.
//! * `status: "rate-limited"` becomes [`WindowReport::limited`]: an authoritative
//!   "this window is exhausted" flag that does not have to be guessed from a
//!   percentage.
//!
//! Unknown fields are ignored (the API may grow), a window that is absent is
//! simply skipped, and a payload that cannot be understood is a typed
//! [`OpenCodeError::Parse`] error — a parser failure is never reported as zero
//! usage.

use crate::models::{UsageReport, WindowReport};
use crate::OpenCodeError;
use chrono::{DateTime, Utc};
use serde::Deserialize;

/// API window tokens mapped to normalized keys and localized labels.
const WINDOWS: [(&str, &str); 3] = [
    ("rolling-5h", "5 小时"),
    ("weekly", "近一周"),
    ("monthly", "近一月"),
];

/// The `status` value that means the window's allowance is exhausted.
const RATE_LIMITED: &str = "rate-limited";

/// Parses the usage API response into a normalized OpenCode Go usage report.
///
/// `now_ms` is used only to derive `reset_in_secs` for display; the authoritative
/// `reset_time` always comes from the server.
pub fn parse_usage(json: &str, now_ms: i64) -> Result<UsageReport, OpenCodeError> {
    let response: UsageResponse = serde_json::from_str(json)
        .map_err(|error| OpenCodeError::Parse(format!("invalid usage response: {error}")))?;

    let mut windows = Vec::new();
    for (key, label, meter) in [
        (WINDOWS[0].0, WINDOWS[0].1, response.usage.rolling),
        (WINDOWS[1].0, WINDOWS[1].1, response.usage.weekly),
        (WINDOWS[2].0, WINDOWS[2].1, response.usage.monthly),
    ] {
        let Some(meter) = meter else {
            continue;
        };
        windows.push(meter.into_report(key, label, now_ms)?);
    }

    if windows.is_empty() {
        return Err(OpenCodeError::Parse(
            "the usage response contains no quota windows".to_owned(),
        ));
    }

    Ok(UsageReport {
        plan_type: String::new(),
        windows,
        fetched_at: now_ms,
    })
}

/// Top-level usage response.
#[derive(Debug, Deserialize)]
struct UsageResponse {
    usage: UsageMeters,
}

/// One meter per quota window. Missing windows are tolerated.
#[derive(Debug, Deserialize)]
struct UsageMeters {
    #[serde(default)]
    rolling: Option<Meter>,
    #[serde(default)]
    weekly: Option<Meter>,
    #[serde(default)]
    monthly: Option<Meter>,
}

/// A single allowance window as the API reports it.
#[derive(Debug, Deserialize)]
struct Meter {
    /// `ok` or `rate-limited`; unknown values are treated as `ok`.
    status: String,
    /// Share of the window already used, 0-100.
    percent: f64,
    /// RFC 3339 instant at which the window resets.
    #[serde(rename = "resetsAt")]
    resets_at: String,
}

impl Meter {
    /// Normalizes onto the shared 100-point model.
    fn into_report(
        self,
        key: &'static str,
        label: &'static str,
        now_ms: i64,
    ) -> Result<WindowReport, OpenCodeError> {
        if !self.percent.is_finite() {
            return Err(OpenCodeError::Parse(format!(
                "the {key} window has a non-finite usage percentage"
            )));
        }
        let reset_time = parse_instant(&self.resets_at)?;
        let used = self.percent.clamp(0.0, 100.0);
        Ok(WindowReport {
            key: key.to_owned(),
            label: label.to_owned(),
            quota: 100.0,
            used,
            remaining: (100.0 - used).max(0.0),
            percent: used,
            // The reset instant doubles as the alert-deduplication cycle: one
            // alert per window, per reset.
            subscribe_time: reset_time,
            reset_time,
            reset_in_secs: seconds_until(reset_time, now_ms),
            limited: self.status.trim().eq_ignore_ascii_case(RATE_LIMITED),
        })
    }
}

/// Parses an RFC 3339 instant into Unix milliseconds.
fn parse_instant(value: &str) -> Result<i64, OpenCodeError> {
    DateTime::parse_from_rfc3339(value.trim())
        .map(|instant| instant.with_timezone(&Utc).timestamp_millis())
        .map_err(|error| OpenCodeError::Parse(format!("invalid resetsAt timestamp: {error}")))
}

/// Whole seconds from `now_ms` until `reset_time_ms`, clamped at zero.
fn seconds_until(reset_time_ms: i64, now_ms: i64) -> i64 {
    if reset_time_ms <= 0 {
        return 0;
    }
    (reset_time_ms - now_ms).div_euclid(1_000).max(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW_MS: i64 = 1_778_800_000_000;

    fn response(rolling: &str, weekly: &str, monthly: &str) -> String {
        format!(r#"{{"usage":{{"rolling":{rolling},"weekly":{weekly},"monthly":{monthly}}}}}"#)
    }

    fn meter(status: &str, percent: f64, resets_at: &str) -> String {
        format!(r#"{{"status":"{status}","percent":{percent},"resetsAt":"{resets_at}"}}"#)
    }

    #[test]
    fn parses_all_three_windows() {
        let json = response(
            &meter("ok", 78.0, "2026-05-01T01:42:12.000Z"),
            &meter("ok", 52.0, "2026-05-02T00:00:00.000Z"),
            &meter("ok", 19.0, "2026-06-01T00:00:00.000Z"),
        );
        let report = parse_usage(&json, NOW_MS).expect("the payload is valid");

        assert_eq!(report.plan_type, "");
        assert_eq!(report.fetched_at, NOW_MS);
        assert_eq!(report.windows.len(), 3);

        let keys: Vec<&str> = report
            .windows
            .iter()
            .map(|window| window.key.as_str())
            .collect();
        assert_eq!(keys, vec!["rolling-5h", "weekly", "monthly"]);

        let rolling = &report.windows[0];
        assert_eq!(rolling.label, "5 小时");
        assert_eq!(rolling.quota, 100.0);
        assert_eq!(rolling.used, 78.0);
        assert_eq!(rolling.percent, 78.0);
        assert_eq!(rolling.remaining, 22.0);
        assert!(!rolling.limited);
    }

    #[test]
    fn reset_time_comes_from_the_server_not_from_now() {
        let json = response(
            &meter("ok", 10.0, "2026-05-01T01:42:12.000Z"),
            &meter("ok", 10.0, "2026-05-02T00:00:00.000Z"),
            &meter("ok", 10.0, "2026-06-01T00:00:00.000Z"),
        );
        let report = parse_usage(&json, NOW_MS).expect("the payload is valid");
        let windows = &report.windows[0];

        let expected = DateTime::parse_from_rfc3339("2026-05-01T01:42:12.000Z")
            .expect("the fixture instant is valid")
            .timestamp_millis();
        // The instant is the server's, and the countdown is derived from it.
        assert_eq!(windows.reset_time, expected);
        assert_eq!(windows.subscribe_time, expected);
        assert_eq!(windows.reset_in_secs, seconds_until(expected, NOW_MS));
    }

    #[test]
    fn rate_limited_status_sets_the_flag() {
        let json = response(
            &meter("ok", 30.0, "2026-05-01T01:42:12.000Z"),
            &meter("rate-limited", 100.0, "2026-05-02T00:00:00.000Z"),
            &meter("ok", 5.0, "2026-06-01T00:00:00.000Z"),
        );
        let report = parse_usage(&json, NOW_MS).expect("the payload is valid");
        assert!(!report.windows[0].limited);
        assert!(report.windows[1].limited);
        assert!(!report.windows[2].limited);
    }

    #[test]
    fn unknown_status_is_not_treated_as_limited() {
        let json = response(
            &meter("throttled", 30.0, "2026-05-01T01:42:12.000Z"),
            &meter("ok", 30.0, "2026-05-02T00:00:00.000Z"),
            &meter("ok", 30.0, "2026-06-01T00:00:00.000Z"),
        );
        let report = parse_usage(&json, NOW_MS).expect("the payload is valid");
        assert!(!report.windows[0].limited);
    }

    #[test]
    fn percentages_are_clamped_to_the_valid_range() {
        let json = response(
            &meter("ok", -5.0, "2026-05-01T01:42:12.000Z"),
            &meter("ok", 150.0, "2026-05-02T00:00:00.000Z"),
            &meter("ok", 42.5, "2026-06-01T00:00:00.000Z"),
        );
        let report = parse_usage(&json, NOW_MS).expect("clamped windows are valid");
        assert_eq!(report.windows[0].percent, 0.0);
        assert_eq!(report.windows[1].percent, 100.0);
        assert_eq!(report.windows[2].percent, 42.5);
        assert_eq!(report.windows[1].remaining, 0.0);
    }

    #[test]
    fn missing_one_window_keeps_the_rest() {
        let json = r#"{"usage":{
            "rolling":{"status":"ok","percent":11,"resetsAt":"2026-05-01T01:42:12.000Z"},
            "weekly":{"status":"ok","percent":22,"resetsAt":"2026-05-02T00:00:00.000Z"}
        }}"#;
        let report = parse_usage(json, NOW_MS).expect("a partial payload is valid");
        assert_eq!(report.windows.len(), 2);
        assert_eq!(report.windows[0].key, "rolling-5h");
        assert_eq!(report.windows[1].key, "weekly");
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let json = r#"{"extra":1,"usage":{
            "rolling":{"status":"ok","percent":11,"resetsAt":"2026-05-01T01:42:12.000Z","plan":"go"},
            "weekly":{"status":"ok","percent":22,"resetsAt":"2026-05-02T00:00:00.000Z"},
            "monthly":{"status":"ok","percent":33,"resetsAt":"2026-06-01T00:00:00.000Z"}
        }}"#;
        let report = parse_usage(json, NOW_MS).expect("unknown fields are ignored");
        assert_eq!(report.windows.len(), 3);
    }

    #[test]
    fn expired_reset_clamps_the_countdown_to_zero() {
        let json = response(
            &meter("ok", 10.0, "2026-05-01T00:00:00.000Z"),
            &meter("ok", 10.0, "2026-05-02T00:00:00.000Z"),
            &meter("ok", 10.0, "2026-06-01T00:00:00.000Z"),
        );
        let report = parse_usage(&json, NOW_MS).expect("the payload is valid");
        assert_eq!(report.windows[0].reset_in_secs, 0);
        assert!(report.windows[0].reset_time > 0);
    }

    #[test]
    fn invalid_timestamp_is_a_parse_error() {
        let json = response(
            &meter("ok", 10.0, "not-a-timestamp"),
            &meter("ok", 10.0, "2026-05-02T00:00:00.000Z"),
            &meter("ok", 10.0, "2026-06-01T00:00:00.000Z"),
        );
        let error = parse_usage(&json, NOW_MS).expect_err("a broken timestamp must fail");
        assert!(matches!(error, OpenCodeError::Parse(_)));
    }

    #[test]
    fn empty_usage_is_a_parse_error() {
        let error = parse_usage(r#"{"usage":{}}"#, NOW_MS).expect_err("no windows is an error");
        assert!(matches!(error, OpenCodeError::Parse(_)));
    }

    #[test]
    fn non_json_is_a_parse_error() {
        let error = parse_usage("<html>login</html>", NOW_MS).expect_err("HTML must fail");
        assert!(matches!(error, OpenCodeError::Parse(_)));
    }

    #[test]
    fn seconds_until_is_never_negative() {
        assert_eq!(seconds_until(2_000, 1_000), 1);
        assert_eq!(seconds_until(1_000, 2_000), 0);
        assert_eq!(seconds_until(0, 1_000), 0);
    }
}

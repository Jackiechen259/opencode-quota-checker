//! Shared quota domain model for the OpenCode Quota Checker.

use serde::{Deserialize, Serialize};

/// Display-ready quota window.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindowReport {
    /// Stable internal key.
    pub key: String,
    /// Localized display label.
    pub label: String,
    /// Total quota.
    pub quota: f64,
    /// Used quota.
    pub used: f64,
    /// Non-negative remaining quota.
    pub remaining: f64,
    /// Percentage clamped to 0-100.
    pub percent: f64,
    /// Subscription-cycle timestamp in milliseconds.
    pub subscribe_time: i64,
    /// Reset timestamp in milliseconds.
    pub reset_time: i64,
    /// Seconds until reset, negative when already expired.
    pub reset_in_secs: i64,
}

/// Display-ready OpenCode Go usage report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UsageReport {
    /// Subscribed plan type. OpenCode does not currently report a plan type.
    pub plan_type: String,
    /// Rolling, weekly, and monthly windows that were present.
    pub windows: Vec<WindowReport>,
    /// Fetch timestamp in milliseconds.
    pub fetched_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_round_trips_through_json() {
        let report = UsageReport {
            plan_type: String::new(),
            windows: vec![WindowReport {
                key: "weekly".to_owned(),
                label: "近一周".to_owned(),
                quota: 100.0,
                used: 52.0,
                remaining: 48.0,
                percent: 52.0,
                subscribe_time: 0,
                reset_time: 1_778_806_132_000,
                reset_in_secs: 6_132,
            }],
            fetched_at: 1_778_800_000_000,
        };
        let json = serde_json::to_string(&report).expect("report serializes");
        let decoded: UsageReport = serde_json::from_str(&json).expect("report deserializes");
        assert_eq!(decoded, report);
    }
}

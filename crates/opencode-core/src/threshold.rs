use crate::{OpenCodeError, UsageReport};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Alert thresholds for each displayed quota window.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Thresholds {
    /// Five-hour threshold percentage.
    pub five_hour: f64,
    /// Weekly threshold percentage.
    pub weekly: f64,
    /// Monthly threshold percentage.
    pub monthly: f64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            five_hour: 80.0,
            weekly: 85.0,
            monthly: 85.0,
        }
    }
}

impl Thresholds {
    /// Validates that every threshold is finite and between 0 and 100.
    pub fn validate(self) -> Result<Self, OpenCodeError> {
        for (name, value) in [
            ("five_hour", self.five_hour),
            ("weekly", self.weekly),
            ("monthly", self.monthly),
        ] {
            if !value.is_finite() || !(0.0..=100.0).contains(&value) {
                return Err(OpenCodeError::Config(format!(
                    "{name} threshold must be between 0 and 100"
                )));
            }
        }
        Ok(self)
    }

    /// Returns the threshold for a report window key.
    pub fn for_key(self, key: &str) -> f64 {
        match key {
            "five_hour" | "rolling-5h" => self.five_hour,
            "weekly" => self.weekly,
            "monthly" => self.monthly,
            _ => 80.0,
        }
    }
}

/// One desktop alert to deliver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertDecision {
    /// Window that triggered the alert.
    pub window_key: String,
    /// Notification title.
    pub title: String,
    /// Notification body.
    pub body: String,
    /// Subscription cycle used for deduplication.
    pub alert_cycle: i64,
}

/// Deterministic output of threshold evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertEvaluation {
    /// Alerts that should be delivered now.
    pub decisions: Vec<AlertDecision>,
    /// Alert-cycle state to keep for the next evaluation.
    pub next_alerted: HashMap<String, i64>,
}

/// Evaluates thresholds and returns both new alerts and deduplication state.
pub fn evaluate_alerts(
    report: &UsageReport,
    thresholds: &Thresholds,
    last_alerted: &HashMap<String, i64>,
) -> AlertEvaluation {
    let mut next_alerted = last_alerted.clone();
    let mut decisions = Vec::new();

    for window in &report.windows {
        if window.percent >= thresholds.for_key(&window.key) {
            if last_alerted.get(&window.key).copied() != Some(window.subscribe_time) {
                decisions.push(AlertDecision {
                    window_key: window.key.clone(),
                    title: format!("配额告警：{}", window.label),
                    body: format!(
                        "已用 {:.1} / {:.1} ({:.0}%)，剩余 {:.1}",
                        window.used, window.quota, window.percent, window.remaining
                    ),
                    alert_cycle: window.subscribe_time,
                });
                next_alerted.insert(window.key.clone(), window.subscribe_time);
            }
        } else {
            next_alerted.remove(&window.key);
        }
    }

    AlertEvaluation {
        decisions,
        next_alerted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WindowReport;

    fn report(percent: f64, cycle: i64) -> UsageReport {
        UsageReport {
            plan_type: "Large".to_owned(),
            windows: vec![WindowReport {
                key: "five_hour".to_owned(),
                label: "5 小时".to_owned(),
                quota: 100.0,
                used: percent,
                remaining: 100.0 - percent,
                percent,
                subscribe_time: cycle,
                reset_time: 0,
                reset_in_secs: 0,
            }],
            fetched_at: 0,
        }
    }

    #[test]
    fn alerts_once_per_subscription_cycle() {
        let thresholds = Thresholds::default();
        let first = evaluate_alerts(&report(90.0, 10), &thresholds, &HashMap::new());
        assert_eq!(first.decisions.len(), 1);

        let duplicate = evaluate_alerts(&report(95.0, 10), &thresholds, &first.next_alerted);
        assert!(duplicate.decisions.is_empty());

        let next_cycle = evaluate_alerts(&report(90.0, 11), &thresholds, &duplicate.next_alerted);
        assert_eq!(next_cycle.decisions.len(), 1);
    }

    #[test]
    fn dropping_below_threshold_clears_deduplication() {
        let thresholds = Thresholds::default();
        let first = evaluate_alerts(&report(90.0, 10), &thresholds, &HashMap::new());
        let below = evaluate_alerts(&report(20.0, 10), &thresholds, &first.next_alerted);
        assert!(below.next_alerted.is_empty());
        let above = evaluate_alerts(&report(90.0, 10), &thresholds, &below.next_alerted);
        assert_eq!(above.decisions.len(), 1);
    }

    #[test]
    fn invalid_thresholds_are_rejected() {
        assert!(Thresholds {
            five_hour: f64::NAN,
            ..Thresholds::default()
        }
        .validate()
        .is_err());
    }

    #[test]
    fn opencode_rolling_window_uses_five_hour_threshold() {
        let thresholds = Thresholds {
            five_hour: 62.0,
            ..Thresholds::default()
        };
        assert_eq!(thresholds.for_key("rolling-5h"), 62.0);
        assert_eq!(thresholds.for_key("weekly"), thresholds.weekly);
        assert_eq!(thresholds.for_key("monthly"), thresholds.monthly);
        assert_eq!(thresholds.for_key("unknown"), 80.0);
    }
}

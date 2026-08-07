use crate::VolcError;
use serde::{Deserialize, Serialize};

/// Full `GetAFPUsage` API response.
#[derive(Debug, Clone, Deserialize)]
pub struct AfpResponse {
    /// Response metadata supplied by the provider.
    #[serde(default, rename = "ResponseMetadata")]
    pub response_metadata: Option<ResponseMetadata>,
    /// Successful response payload.
    #[serde(default, rename = "Result")]
    pub result: Option<AfpResult>,
    /// Structured API error.
    #[serde(default, rename = "Error")]
    pub error: Option<AfpApiError>,
}

/// Provider response metadata.
#[derive(Debug, Clone, Deserialize)]
pub struct ResponseMetadata {
    /// Provider request identifier.
    #[serde(default, rename = "RequestId")]
    pub request_id: Option<String>,
}

/// Structured provider error.
#[derive(Debug, Clone, Deserialize)]
pub struct AfpApiError {
    /// Provider error code.
    #[serde(default, rename = "Code")]
    pub code: Option<String>,
    /// Provider error message.
    #[serde(default, rename = "Message")]
    pub message: Option<String>,
}

/// Successful AFP result.
#[derive(Debug, Clone, Deserialize)]
pub struct AfpResult {
    /// Subscribed plan type.
    #[serde(default, rename = "PlanType")]
    pub plan_type: String,
    /// Five-hour rolling window.
    #[serde(default, rename = "AFPFiveHour")]
    pub five_hour: Option<QuotaWindow>,
    /// Daily response field retained for compatibility but not displayed.
    #[serde(default, rename = "AFPDaily")]
    pub daily: Option<QuotaWindow>,
    /// Weekly rolling window.
    #[serde(default, rename = "AFPWeekly")]
    pub weekly: Option<QuotaWindow>,
    /// Monthly rolling window.
    #[serde(default, rename = "AFPMonthly")]
    pub monthly: Option<QuotaWindow>,
}

/// Raw quota window from the API.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuotaWindow {
    /// Total quota.
    #[serde(rename = "Quota")]
    pub quota: f64,
    /// Used quota.
    #[serde(rename = "Used")]
    pub used: f64,
    /// Subscription-cycle timestamp in milliseconds.
    #[serde(rename = "SubscribeTime")]
    pub subscribe_time: i64,
    /// Reset timestamp in milliseconds.
    #[serde(rename = "ResetTime")]
    pub reset_time: i64,
}

impl QuotaWindow {
    fn validate(&self, key: &str) -> Result<(), VolcError> {
        if !self.quota.is_finite() || !self.used.is_finite() {
            return Err(VolcError::ResponseValue(format!(
                "{key} contains a non-finite number"
            )));
        }
        if self.quota < 0.0 || self.used < 0.0 {
            return Err(VolcError::ResponseValue(format!(
                "{key} contains a negative quota value"
            )));
        }
        Ok(())
    }

    fn remaining(&self) -> f64 {
        (self.quota - self.used).max(0.0)
    }

    fn percent(&self) -> f64 {
        if self.quota > 0.0 {
            (self.used / self.quota * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        }
    }
}

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

/// Display-ready AFP usage report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UsageReport {
    /// Subscribed plan type.
    pub plan_type: String,
    /// Five-hour, weekly, and monthly windows that were present.
    pub windows: Vec<WindowReport>,
    /// Fetch timestamp in milliseconds.
    pub fetched_at: i64,
}

impl AfpResponse {
    /// Converts the provider response using the current UTC timestamp.
    pub fn into_report(self) -> Result<UsageReport, VolcError> {
        self.into_report_at(chrono::Utc::now().timestamp_millis())
    }

    /// Converts the provider response using a deterministic timestamp.
    pub fn into_report_at(self, now_ms: i64) -> Result<UsageReport, VolcError> {
        if let Some(error) = self.error {
            return Err(VolcError::Api {
                code: error.code.unwrap_or_else(|| "Unknown".to_owned()),
                message: error
                    .message
                    .unwrap_or_else(|| "provider returned no error message".to_owned()),
            });
        }
        let result = self
            .result
            .ok_or_else(|| VolcError::ResponseValue("response is missing Result".to_owned()))?;
        result.into_report_at(now_ms)
    }
}

impl AfpResult {
    fn into_report_at(self, now_ms: i64) -> Result<UsageReport, VolcError> {
        let entries = [
            ("five_hour", "5 小时", self.five_hour),
            ("weekly", "近一周", self.weekly),
            ("monthly", "近一月", self.monthly),
        ];
        let mut windows = Vec::with_capacity(entries.len());

        for (key, label, window) in entries {
            let Some(window) = window else {
                continue;
            };
            window.validate(key)?;
            windows.push(WindowReport {
                key: key.to_owned(),
                label: label.to_owned(),
                quota: window.quota,
                used: window.used,
                remaining: window.remaining(),
                percent: window.percent(),
                subscribe_time: window.subscribe_time,
                reset_time: window.reset_time,
                reset_in_secs: window.reset_time.saturating_sub(now_ms) / 1_000,
            });
        }

        Ok(UsageReport {
            plan_type: self.plan_type,
            windows,
            fetched_at: now_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = include_str!("../../../tests/fixtures/usage-small.json");

    #[test]
    fn parses_expected_display_windows() {
        let response: AfpResponse = serde_json::from_str(SAMPLE).expect("fixture is valid JSON");
        let report = response
            .into_report_at(1_778_800_000_000)
            .expect("fixture is a valid response");
        assert_eq!(report.plan_type, "Large");
        assert_eq!(report.windows.len(), 3);
        assert_eq!(report.windows[0].key, "five_hour");
        assert_eq!(report.windows[0].percent, 25.0);
        assert_eq!(report.windows[0].remaining, 37.5);
    }

    #[test]
    fn missing_result_is_an_error() {
        let response: AfpResponse =
            serde_json::from_str(r#"{"ResponseMetadata":{"RequestId":"x"}}"#)
                .expect("minimal response decodes");
        assert!(matches!(
            response.into_report_at(0),
            Err(VolcError::ResponseValue(_))
        ));
    }

    #[test]
    fn missing_windows_produce_an_empty_report() {
        let response: AfpResponse = serde_json::from_str(r#"{"Result":{"PlanType":"Small"}}"#)
            .expect("minimal result decodes");
        let report = response.into_report_at(0).expect("empty windows are valid");
        assert!(report.windows.is_empty());
    }

    #[test]
    fn percentage_is_clamped_and_zero_quota_is_safe() {
        let over: AfpResponse = serde_json::from_str(
            r#"{"Result":{"PlanType":"x","AFPFiveHour":{"Quota":100,"Used":150,"SubscribeTime":0,"ResetTime":0}}}"#,
        )
        .expect("over-quota response decodes");
        assert_eq!(
            over.into_report_at(0).expect("over quota is valid").windows[0].percent,
            100.0
        );

        let zero: AfpResponse = serde_json::from_str(
            r#"{"Result":{"PlanType":"x","AFPFiveHour":{"Quota":0,"Used":0,"SubscribeTime":0,"ResetTime":0}}}"#,
        )
        .expect("zero-quota response decodes");
        assert_eq!(
            zero.into_report_at(0).expect("zero quota is valid").windows[0].percent,
            0.0
        );
    }

    #[test]
    fn negative_values_are_rejected() {
        let response: AfpResponse = serde_json::from_str(
            r#"{"Result":{"PlanType":"x","AFPFiveHour":{"Quota":-1,"Used":0,"SubscribeTime":0,"ResetTime":0}}}"#,
        )
        .expect("negative values still decode");
        assert!(matches!(
            response.into_report_at(0),
            Err(VolcError::ResponseValue(_))
        ));
    }
}

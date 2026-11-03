//! Parses the OpenCode Console Go subscription and activity endpoints.
//!
//! Two Console endpoints back the Go overview page:
//!
//! * `GET /console/api/go/status` returns the subscription product and the
//!   three allowance meters (5-hour, weekly, monthly) plus the paid-period
//!   start. Meter values are `BigInt` decimal strings.
//! * `GET /console/api/activity/go` returns the Go-only daily cost/request/token
//!   series for the current period and the per-model request counts.
//!
//! Both payloads may be `null` when the workspace has no active Go
//! subscription. Parsers fail closed: missing required fields, negative or
//! fractional numbers, invalid calendar dates, and overflowing totals are
//! contract errors, never zero usage.

use crate::serde_util::{deserialize_u64, normalize_date, parse_instant};
use crate::OpenCodeError;
use serde::{Deserialize, Serialize};

/// One Go allowance meter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GoMeter {
    /// Used allowance in microcents (`1 USD = 100,000,000`).
    pub used_microcents: u64,
    /// Total allowance in microcents.
    pub limit_microcents: u64,
    /// Reset timestamp in milliseconds.
    pub resets_at: i64,
    /// Used share, clamped to 0-100.
    pub percent: f64,
    /// Whether the meter is exhausted.
    pub limited: bool,
}

/// The paid-period access block of the Go status response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GoAccess {
    /// Paid-period start timestamp in milliseconds.
    pub starts_at: i64,
    /// 5-hour rolling meter.
    pub five_hour: Option<GoMeter>,
    /// Weekly meter.
    pub week: Option<GoMeter>,
    /// Monthly meter.
    pub month: Option<GoMeter>,
}

/// Go subscription status used by the overview page.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GoStatusReport {
    /// `go`, `go-plus`, or `go-teams`.
    pub product: String,
    /// Whether the subscription ends at the period end.
    pub cancel_at_period_end: bool,
    /// Whether Extra usage (balance past the limit) is enabled.
    pub use_balance: bool,
    /// Paid-period access; absent when the subscription is inactive.
    pub access: Option<GoAccess>,
    /// Fetch timestamp in milliseconds.
    pub fetched_at: i64,
}

/// One day of the Go-only usage series.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GoActivityDay {
    /// UTC date, `YYYY-MM-DD`.
    pub date: String,
    /// Cost in microcents.
    pub cost_microcents: u64,
    /// Requests on that day.
    pub requests: u64,
    /// Tokens on that day.
    pub tokens: u64,
}

/// Per-model request count for the current period.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GoModelUsage {
    /// Provider identifier (e.g. `opencode`).
    pub provider: String,
    /// Model identifier (e.g. `deepseek-v4.1-flash`).
    pub model: String,
    /// Requests attributed to the model.
    pub requests: u64,
}

/// Go-only activity for the current paid period.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GoActivityReport {
    /// Period start timestamp in milliseconds.
    pub starts_at: i64,
    /// Daily Go usage, oldest first.
    pub daily: Vec<GoActivityDay>,
    /// Per-model request counts.
    pub models: Vec<GoModelUsage>,
    /// Sum of `daily[].cost_microcents`.
    pub total_cost_microcents: u64,
    /// Sum of `daily[].requests`.
    pub total_requests: u64,
    /// Sum of `daily[].tokens`.
    pub total_tokens: u64,
    /// Fetch timestamp in milliseconds.
    pub fetched_at: i64,
}

/// Parses `/console/api/go/status`; `null` means no active subscription.
pub fn parse_go_status(json: &str, now_ms: i64) -> Result<Option<GoStatusReport>, OpenCodeError> {
    let response: Option<GoStatusResponse> = serde_json::from_str(json)
        .map_err(|error| OpenCodeError::Parse(format!("invalid Go status response: {error}")))?;
    let Some(response) = response else {
        return Ok(None);
    };

    let access = response
        .access
        .map(|access| {
            let starts_at = parse_instant(&access.starts_at).map_err(|error| {
                OpenCodeError::Parse(format!("invalid Go period start: {error}"))
            })?;
            Ok::<_, OpenCodeError>(GoAccess {
                starts_at,
                five_hour: access.meters.five_hour.map(convert_meter).transpose()?,
                week: access.meters.week.map(convert_meter).transpose()?,
                month: access.meters.month.map(convert_meter).transpose()?,
            })
        })
        .transpose()?;

    Ok(Some(GoStatusReport {
        product: if response.product.trim().is_empty() {
            "go".to_owned()
        } else {
            response.product
        },
        cancel_at_period_end: response.cancel_at_period_end,
        use_balance: response.use_balance,
        access,
        fetched_at: now_ms,
    }))
}

/// Parses `/console/api/activity/go`; `null` means no activity payload.
pub fn parse_go_activity(
    json: &str,
    now_ms: i64,
) -> Result<Option<GoActivityReport>, OpenCodeError> {
    let response: Option<GoActivityResponse> = serde_json::from_str(json)
        .map_err(|error| OpenCodeError::Parse(format!("invalid Go activity response: {error}")))?;
    let Some(response) = response else {
        return Ok(None);
    };

    let starts_at = parse_instant(&response.starts_at)
        .map_err(|error| OpenCodeError::Parse(format!("invalid Go activity start: {error}")))?;

    let mut daily = Vec::with_capacity(response.daily.len());
    let mut total_cost_microcents = 0_u64;
    let mut total_requests = 0_u64;
    let mut total_tokens = 0_u64;
    for day in response.daily {
        let date = normalize_date(&day.date)
            .map_err(|error| OpenCodeError::Parse(format!("invalid Go activity date: {error}")))?;
        total_cost_microcents = total_cost_microcents
            .checked_add(day.cost_microcents)
            .ok_or_else(|| OpenCodeError::Parse("Go activity cost overflowed u64".to_owned()))?;
        total_requests = total_requests.checked_add(day.requests).ok_or_else(|| {
            OpenCodeError::Parse("Go activity requests overflowed u64".to_owned())
        })?;
        total_tokens = total_tokens
            .checked_add(day.tokens)
            .ok_or_else(|| OpenCodeError::Parse("Go activity tokens overflowed u64".to_owned()))?;
        daily.push(GoActivityDay {
            date,
            cost_microcents: day.cost_microcents,
            requests: day.requests,
            tokens: day.tokens,
        });
    }

    let models = response
        .models
        .into_iter()
        .map(|model| GoModelUsage {
            provider: model.provider,
            model: model.model,
            requests: model.requests,
        })
        .collect();

    Ok(Some(GoActivityReport {
        starts_at,
        daily,
        models,
        total_cost_microcents,
        total_requests,
        total_tokens,
        fetched_at: now_ms,
    }))
}

fn convert_meter(meter: GoMeterResponse) -> Result<GoMeter, OpenCodeError> {
    let resets_at = parse_instant(&meter.resets_at)
        .map_err(|error| OpenCodeError::Parse(format!("invalid Go meter reset: {error}")))?;
    let percent = if meter.limit_microcents == 0 {
        0.0
    } else {
        (meter.used_microcents as f64 / meter.limit_microcents as f64 * 100.0).clamp(0.0, 100.0)
    };
    Ok(GoMeter {
        used_microcents: meter.used_microcents,
        limit_microcents: meter.limit_microcents,
        resets_at,
        percent,
        limited: meter.limit_microcents > 0 && meter.used_microcents >= meter.limit_microcents,
    })
}

#[derive(Debug, Deserialize)]
struct GoStatusResponse {
    #[serde(default)]
    product: String,
    #[serde(rename = "cancelAtPeriodEnd", default)]
    cancel_at_period_end: bool,
    #[serde(rename = "useBalance", default)]
    use_balance: bool,
    #[serde(default)]
    access: Option<GoAccessResponse>,
}

#[derive(Debug, Deserialize)]
struct GoAccessResponse {
    #[serde(rename = "startsAt")]
    starts_at: String,
    #[serde(default)]
    meters: GoMetersResponse,
}

#[derive(Debug, Default, Deserialize)]
struct GoMetersResponse {
    #[serde(rename = "fiveHour")]
    five_hour: Option<GoMeterResponse>,
    week: Option<GoMeterResponse>,
    month: Option<GoMeterResponse>,
}

#[derive(Debug, Deserialize)]
struct GoMeterResponse {
    #[serde(rename = "usedMicroCents", deserialize_with = "deserialize_u64")]
    used_microcents: u64,
    #[serde(rename = "limitMicroCents", deserialize_with = "deserialize_u64")]
    limit_microcents: u64,
    #[serde(rename = "resetsAt")]
    resets_at: String,
}

#[derive(Debug, Deserialize)]
struct GoActivityResponse {
    #[serde(rename = "startsAt")]
    starts_at: String,
    daily: Vec<GoActivityDayResponse>,
    models: Vec<GoModelResponse>,
}

#[derive(Debug, Deserialize)]
struct GoActivityDayResponse {
    date: String,
    #[serde(rename = "totalCostMicroCents", deserialize_with = "deserialize_u64")]
    cost_microcents: u64,
    #[serde(rename = "totalRequests", deserialize_with = "deserialize_u64")]
    requests: u64,
    #[serde(rename = "totalTokens", deserialize_with = "deserialize_u64")]
    tokens: u64,
}

#[derive(Debug, Deserialize)]
struct GoModelResponse {
    provider: String,
    model: String,
    #[serde(rename = "totalRequests", deserialize_with = "deserialize_u64")]
    requests: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW_MS: i64 = 1_778_800_000_000;

    fn status_json() -> String {
        r#"{
            "product": "go",
            "cancelAtPeriodEnd": false,
            "useBalance": false,
            "access": {
                "startsAt": "2026-10-26T00:00:00.000Z",
                "meters": {
                    "fiveHour": {"usedMicroCents": "120000000", "limitMicroCents": "1200000000", "resetsAt": "2026-11-03T21:00:00.000Z"},
                    "week": {"usedMicroCents": "420000000", "limitMicroCents": "6000000000", "resetsAt": "2026-11-09T00:00:00.000Z"},
                    "month": {"usedMicroCents": "14400000000", "limitMicroCents": "60000000000", "resetsAt": "2026-11-26T00:00:00.000Z"}
                }
            }
        }"#
        .to_owned()
    }

    fn activity_json() -> String {
        r#"{
            "startsAt": "2026-10-26T00:00:00.000Z",
            "daily": [
                {"date": "2026-10-26", "totalCostMicroCents": "100", "totalTokens": "1000", "totalRequests": "2"},
                {"date": "2026-10-27", "totalCostMicroCents": "200", "totalTokens": "3000", "totalRequests": "3"}
            ],
            "models": [
                {"provider": "opencode", "model": "deepseek-v4.1-flash", "totalRequests": "4"},
                {"provider": "opencode", "model": "glm-5.3", "totalRequests": "1"}
            ]
        }"#
        .to_owned()
    }

    #[test]
    fn parses_the_go_status_meters_and_percentages() {
        let status = parse_go_status(&status_json(), NOW_MS)
            .expect("valid status")
            .expect("active subscription");

        assert_eq!(status.product, "go");
        assert!(!status.use_balance);
        let access = status.access.expect("access block");
        assert_eq!(access.starts_at, 1_792_972_800_000);
        let month = access.month.expect("monthly meter");
        assert_eq!(month.used_microcents, 14_400_000_000);
        assert_eq!(month.limit_microcents, 60_000_000_000);
        assert_eq!(month.percent, 24.0);
        assert!(!month.limited);
        assert_eq!(month.resets_at, 1_795_651_200_000);
    }

    #[test]
    fn null_status_means_no_active_subscription() {
        assert!(parse_go_status("null", NOW_MS)
            .expect("null is valid")
            .is_none());
    }

    #[test]
    fn status_without_access_still_parses() {
        let status = parse_go_status(
            r#"{"product":"go","cancelAtPeriodEnd":true,"useBalance":true}"#,
            NOW_MS,
        )
        .expect("valid status")
        .expect("status exists");
        assert!(status.access.is_none());
        assert!(status.cancel_at_period_end);
        assert!(status.use_balance);
    }

    #[test]
    fn rejects_an_invalid_meter_value() {
        let json = status_json().replace("\"1200000000\"", "\"-1\"");
        let error = parse_go_status(&json, NOW_MS).expect_err("negative meter must fail");
        assert!(matches!(error, OpenCodeError::Parse(_)));
    }

    #[test]
    fn parses_the_go_activity_sums_and_dates() {
        let activity = parse_go_activity(&activity_json(), NOW_MS)
            .expect("valid activity")
            .expect("activity exists");

        assert_eq!(activity.starts_at, 1_792_972_800_000);
        assert_eq!(activity.daily.len(), 2);
        assert_eq!(activity.daily[0].date, "2026-10-26");
        assert_eq!(activity.total_cost_microcents, 300);
        assert_eq!(activity.total_requests, 5);
        assert_eq!(activity.total_tokens, 4000);
        assert_eq!(activity.models.len(), 2);
    }

    #[test]
    fn accepts_an_empty_activity_window() {
        let activity = parse_go_activity(
            r#"{"startsAt":"2026-10-26","daily":[],"models":[]}"#,
            NOW_MS,
        )
        .expect("empty activity is valid")
        .expect("activity exists");
        assert!(activity.daily.is_empty());
        assert_eq!(activity.total_requests, 0);
    }

    #[test]
    fn rejects_a_missing_activity_field() {
        let json = r#"{"startsAt":"2026-10-26","daily":[{"date":"2026-10-26","totalCostMicroCents":"1","totalRequests":"1"}],"models":[]}"#;
        let error = parse_go_activity(json, NOW_MS).expect_err("missing tokens must fail");
        assert!(matches!(error, OpenCodeError::Parse(_)));
    }

    #[test]
    fn rejects_invalid_dates_and_overflow() {
        let invalid_date = activity_json().replace("2026-10-27", "2026-02-30");
        assert!(matches!(
            parse_go_activity(&invalid_date, NOW_MS),
            Err(OpenCodeError::Parse(_))
        ));

        let overflow = activity_json().replace("\"100\"", "\"18446744073709551615\"");
        assert!(matches!(
            parse_go_activity(&overflow, NOW_MS),
            Err(OpenCodeError::Parse(_))
        ));
    }

    #[test]
    fn meter_percentages_are_clamped_and_limited_is_authoritative() {
        let json = status_json().replace("\"14400000000\"", "\"72000000000\"");
        let month = parse_go_status(&json, NOW_MS)
            .expect("valid status")
            .expect("active subscription")
            .access
            .expect("access")
            .month
            .expect("month");
        assert_eq!(month.percent, 100.0);
        assert!(month.limited);
    }
}

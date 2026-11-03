//! Parses the OpenCode Console workspace token-usage summary.
//!
//! `GET /console/api/usage/summary?range=30d&includeLegacyKeys=true` returns
//! workspace-wide totals for the requested window:
//!
//! ```json
//! {
//!   "totalRequests": "123",
//!   "totalInputTokens": "1000",
//!   "totalOutputTokens": "2000",
//!   "totalCacheReadTokens": "3000",
//!   "totalCacheWrite5mTokens": "40",
//!   "totalCacheWrite1hTokens": "60",
//!   "totalCostMicroCents": "30000000",
//!   "services": []
//! }
//! ```
//!
//! Every numeric field is serialized as a decimal string because the Console
//! stores `BigInt` values. The parser accepts those strings or plain JSON
//! integers and rejects negative, fractional, non-numeric, missing, and
//! overflowing values with a typed [`OpenCodeError::Parse`] error. A payload
//! that cannot be understood is never reported as zero usage.
//!
//! Mapping rules:
//!
//! * `totalCacheWrite5mTokens` + `totalCacheWrite1hTokens` become
//!   [`TokenUsageReport::cache_write_tokens`].
//! * `total_tokens` sums input, output, cache read and cache write, matching
//!   the Console's own `totalTokens` getter.
//! * `totalCostMicroCents` is kept in microcents (`1 USD = 100,000,000`) so
//!   the frontend owns the display rounding and no precision is lost here.

use crate::serde_util::deserialize_u64;
use crate::OpenCodeError;
use serde::{Deserialize, Serialize};

/// Query range requested by the application.
pub const TOKEN_USAGE_RANGE: &str = "30d";

/// Display-ready workspace token usage for one period.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenUsageReport {
    /// Period the server aggregated, e.g. `"30d"`.
    pub range: String,
    /// Total requests in the period.
    pub requests: u64,
    /// Uncached input tokens.
    pub input_tokens: u64,
    /// Output tokens.
    pub output_tokens: u64,
    /// Cached input tokens read.
    pub cache_read_tokens: u64,
    /// Cached input tokens written (5-minute + 1-hour buckets).
    pub cache_write_tokens: u64,
    /// Sum of every token bucket.
    pub total_tokens: u64,
    /// Estimated cost in microcents (`1 USD = 100,000,000`).
    pub cost_microcents: u64,
    /// Fetch timestamp in milliseconds.
    pub fetched_at: i64,
}

/// Parses the Console usage summary into the shared token report.
///
/// `range` is the query value the caller requested (the response does not echo
/// it), and `now_ms` is stored as the fetch timestamp for display.
pub fn parse_token_usage(
    json: &str,
    range: &str,
    now_ms: i64,
) -> Result<TokenUsageReport, OpenCodeError> {
    let response: TokenUsageResponse = serde_json::from_str(json)
        .map_err(|error| OpenCodeError::Parse(format!("invalid token usage response: {error}")))?;

    let cache_write_tokens = response
        .total_cache_write_5m_tokens
        .checked_add(response.total_cache_write_1h_tokens)
        .ok_or_else(|| OpenCodeError::Parse("token cache-write total overflowed u64".to_owned()))?;
    let total_tokens = response
        .total_input_tokens
        .checked_add(response.total_output_tokens)
        .and_then(|total| total.checked_add(response.total_cache_read_tokens))
        .and_then(|total| total.checked_add(cache_write_tokens))
        .ok_or_else(|| OpenCodeError::Parse("token total overflowed u64".to_owned()))?;

    Ok(TokenUsageReport {
        range: range.to_owned(),
        requests: response.total_requests,
        input_tokens: response.total_input_tokens,
        output_tokens: response.total_output_tokens,
        cache_read_tokens: response.total_cache_read_tokens,
        cache_write_tokens,
        total_tokens,
        cost_microcents: response.total_cost_micro_cents,
        fetched_at: now_ms,
    })
}

/// The subset of the Console summary the application consumes.
///
/// Required fields are not `Option`: a missing field is a contract change and
/// must fail rather than silently read as zero.
#[derive(Debug, Deserialize)]
struct TokenUsageResponse {
    #[serde(rename = "totalRequests", deserialize_with = "deserialize_u64")]
    total_requests: u64,
    #[serde(rename = "totalInputTokens", deserialize_with = "deserialize_u64")]
    total_input_tokens: u64,
    #[serde(rename = "totalOutputTokens", deserialize_with = "deserialize_u64")]
    total_output_tokens: u64,
    #[serde(rename = "totalCacheReadTokens", deserialize_with = "deserialize_u64")]
    total_cache_read_tokens: u64,
    #[serde(
        rename = "totalCacheWrite5mTokens",
        deserialize_with = "deserialize_u64"
    )]
    total_cache_write_5m_tokens: u64,
    #[serde(
        rename = "totalCacheWrite1hTokens",
        deserialize_with = "deserialize_u64"
    )]
    total_cache_write_1h_tokens: u64,
    #[serde(rename = "totalCostMicroCents", deserialize_with = "deserialize_u64")]
    total_cost_micro_cents: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW_MS: i64 = 1_778_800_000_000;

    fn summary(
        requests: &str,
        input: &str,
        output: &str,
        cache_read: &str,
        cache_write_5m: &str,
        cache_write_1h: &str,
        cost: &str,
    ) -> String {
        format!(
            r#"{{"totalRequests":{requests},"totalInputTokens":{input},"totalOutputTokens":{output},"totalCacheReadTokens":{cache_read},"totalCacheWrite5mTokens":{cache_write_5m},"totalCacheWrite1hTokens":{cache_write_1h},"totalCostMicroCents":{cost},"services":[]}}"#
        )
    }

    fn valid() -> String {
        summary(
            r#""123""#,
            r#""1000""#,
            r#""2000""#,
            r#""3000""#,
            r#""40""#,
            r#""60""#,
            r#""30000000""#,
        )
    }

    #[test]
    fn parses_string_totals_and_computes_the_buckets() {
        let report =
            parse_token_usage(&valid(), TOKEN_USAGE_RANGE, NOW_MS).expect("the summary is valid");

        assert_eq!(report.range, "30d");
        assert_eq!(report.requests, 123);
        assert_eq!(report.input_tokens, 1000);
        assert_eq!(report.output_tokens, 2000);
        assert_eq!(report.cache_read_tokens, 3000);
        assert_eq!(report.cache_write_tokens, 100);
        assert_eq!(report.total_tokens, 6100);
        assert_eq!(report.cost_microcents, 30_000_000);
        assert_eq!(report.fetched_at, NOW_MS);
    }

    #[test]
    fn accepts_plain_json_integers() {
        let json = summary("1", "10", "20", "30", "4", "6", "100");
        let report =
            parse_token_usage(&json, TOKEN_USAGE_RANGE, NOW_MS).expect("integers are accepted");
        assert_eq!(report.total_tokens, 70);
        assert_eq!(report.cache_write_tokens, 10);
    }

    #[test]
    fn rejects_a_missing_required_field() {
        let json = r#"{"totalRequests":"1","totalInputTokens":"10","totalOutputTokens":"20","totalCacheReadTokens":"30","totalCacheWrite5mTokens":"4","totalCostMicroCents":"0"}"#;
        let error = parse_token_usage(json, TOKEN_USAGE_RANGE, NOW_MS)
            .expect_err("a missing cache-write bucket must fail");
        assert!(matches!(error, OpenCodeError::Parse(_)));
    }

    #[test]
    fn rejects_negative_fractional_and_non_numeric_values() {
        for value in [r#""-1""#, r#""1.5""#, r#""abc""#, "1.5", "-1"] {
            let json = summary("1", "10", "20", "30", "4", "6", value);
            let error = parse_token_usage(&json, TOKEN_USAGE_RANGE, NOW_MS).expect_err(value);
            assert!(
                matches!(error, OpenCodeError::Parse(_)),
                "value {value} must fail, got {error:?}"
            );
        }
    }

    #[test]
    fn rejects_overflowing_totals() {
        let json = summary(
            "1",
            "10",
            "20",
            "30",
            r#""18446744073709551615""#,
            r#""1""#,
            "0",
        );
        let error = parse_token_usage(&json, TOKEN_USAGE_RANGE, NOW_MS)
            .expect_err("cache-write overflow must fail");
        assert!(matches!(error, OpenCodeError::Parse(_)));
    }

    #[test]
    fn keeps_zero_usage_as_a_valid_report() {
        let json = summary("0", "0", "0", "0", "0", "0", "0");
        let report =
            parse_token_usage(&json, TOKEN_USAGE_RANGE, NOW_MS).expect("an empty window is valid");
        assert_eq!(report.total_tokens, 0);
        assert_eq!(report.cost_microcents, 0);
    }
}

//! Parses the subset of the Console billing status used by the Go overview.
//!
//! `GET /console/api/billing/status` returns the workspace balance and
//! available credit. The Go overview only needs the credit display; billing
//! mutations and invoice history stay in the Console. The endpoint requires
//! the key's `billing:read` scope, so callers must treat failures as optional.

use crate::serde_util::deserialize_u64;
use crate::OpenCodeError;
use serde::{Deserialize, Serialize};

/// Workspace billing summary for the Go overview page.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BillingReport {
    /// Credit available to pay for usage beyond a Go limit.
    pub available_microcents: u64,
    /// Current balance in microcents.
    pub balance_microcents: u64,
    /// Billing mode reported by the Console (e.g. `prepaid`).
    pub billing_mode: String,
    /// Fetch timestamp in milliseconds.
    pub fetched_at: i64,
}

/// Parses `/console/api/billing/status`.
pub fn parse_billing_status(json: &str, now_ms: i64) -> Result<BillingReport, OpenCodeError> {
    let response: BillingStatusResponse = serde_json::from_str(json)
        .map_err(|error| OpenCodeError::Parse(format!("invalid billing response: {error}")))?;
    Ok(BillingReport {
        available_microcents: response.available_micro_cents,
        balance_microcents: response.balance_micro_cents,
        billing_mode: response.billing_mode,
        fetched_at: now_ms,
    })
}

#[derive(Debug, Deserialize)]
struct BillingStatusResponse {
    #[serde(rename = "availableMicroCents", deserialize_with = "deserialize_u64")]
    available_micro_cents: u64,
    #[serde(
        rename = "balanceMicroCents",
        default,
        deserialize_with = "deserialize_u64"
    )]
    balance_micro_cents: u64,
    #[serde(rename = "billingMode", default)]
    billing_mode: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW_MS: i64 = 1_778_800_000_000;

    #[test]
    fn parses_credit_and_balance() {
        let report = parse_billing_status(
            r#"{"billingMode":"prepaid","availableMicroCents":"123","balanceMicroCents":"45"}"#,
            NOW_MS,
        )
        .expect("valid billing status");
        assert_eq!(report.available_microcents, 123);
        assert_eq!(report.balance_microcents, 45);
        assert_eq!(report.billing_mode, "prepaid");
        assert_eq!(report.fetched_at, NOW_MS);
    }

    #[test]
    fn optional_balance_and_mode_default_to_zero_and_empty() {
        let report = parse_billing_status(r#"{"availableMicroCents":0}"#, NOW_MS)
            .expect("only available credit is required");
        assert_eq!(report.available_microcents, 0);
        assert_eq!(report.balance_microcents, 0);
        assert!(report.billing_mode.is_empty());
    }

    #[test]
    fn rejects_missing_or_invalid_available_credit() {
        assert!(matches!(
            parse_billing_status(r#"{"billingMode":"prepaid"}"#, NOW_MS),
            Err(OpenCodeError::Parse(_))
        ));
        assert!(matches!(
            parse_billing_status(r#"{"availableMicroCents":"-1"}"#, NOW_MS),
            Err(OpenCodeError::Parse(_))
        ));
    }
}

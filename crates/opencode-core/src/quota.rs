//! Quota service that ties the usage client and parser together.
//!
//! This is the single entry point the desktop application uses to refresh
//! quota. It validates credentials before making any request and never exposes
//! the raw response body or the API key to callers.

use crate::billing::{parse_billing_status, BillingReport};
use crate::client::OpenCodeClient;
use crate::error::OpenCodeError;
use crate::go_usage::{parse_go_activity, parse_go_status, GoActivityReport, GoStatusReport};
use crate::models::UsageReport;
use crate::token_usage::{parse_token_usage, TokenUsageReport, TOKEN_USAGE_RANGE};
use crate::usage::parse_usage;

/// Fetches and normalizes OpenCode Go quota for the desktop application.
#[derive(Clone, Debug)]
pub struct QuotaService {
    client: OpenCodeClient,
}

impl QuotaService {
    /// Creates a service for the production usage endpoint.
    pub fn new() -> Result<Self, OpenCodeError> {
        Ok(Self {
            client: OpenCodeClient::new()?,
        })
    }

    /// Creates a service backed by an existing client (used in tests).
    pub fn with_client(client: OpenCodeClient) -> Self {
        Self { client }
    }

    /// Fetches and parses the Go allowance into a normalized report.
    ///
    /// The `api_key` is treated as a secret and only ever transmitted as a
    /// request header.
    pub async fn fetch_quota(
        &self,
        api_key: &str,
        workspace_id: Option<&str>,
    ) -> Result<UsageReport, OpenCodeError> {
        if api_key.trim().is_empty() {
            return Err(OpenCodeError::CredentialsMissing);
        }
        let body = self
            .client
            .fetch_usage(api_key, normalize_workspace(workspace_id))
            .await?;
        parse_usage(&body, chrono::Utc::now().timestamp_millis())
    }

    /// Fetches and parses the workspace's 30-day token usage.
    ///
    /// This is the Console Activity summary, not the Go allowance endpoint.
    /// The `api_key` is treated as a secret and only ever transmitted as a
    /// request header.
    pub async fn fetch_token_usage(
        &self,
        api_key: &str,
    ) -> Result<TokenUsageReport, OpenCodeError> {
        if api_key.trim().is_empty() {
            return Err(OpenCodeError::CredentialsMissing);
        }
        let body = self.client.fetch_token_usage(api_key).await?;
        parse_token_usage(
            &body,
            TOKEN_USAGE_RANGE,
            chrono::Utc::now().timestamp_millis(),
        )
    }

    /// Fetches and parses the Go subscription status (meters + period start).
    ///
    /// `fresh` bypasses the Console's short-lived status cache; the overview
    /// page uses it for manual refreshes.
    pub async fn fetch_go_status(
        &self,
        api_key: &str,
        fresh: bool,
    ) -> Result<Option<GoStatusReport>, OpenCodeError> {
        if api_key.trim().is_empty() {
            return Err(OpenCodeError::CredentialsMissing);
        }
        let body = self.client.fetch_go_status(api_key, fresh).await?;
        parse_go_status(&body, chrono::Utc::now().timestamp_millis())
    }

    /// Fetches and parses the Go-only daily and per-model activity.
    pub async fn fetch_go_activity(
        &self,
        api_key: &str,
    ) -> Result<Option<GoActivityReport>, OpenCodeError> {
        if api_key.trim().is_empty() {
            return Err(OpenCodeError::CredentialsMissing);
        }
        let body = self.client.fetch_go_activity(api_key).await?;
        parse_go_activity(&body, chrono::Utc::now().timestamp_millis())
    }

    /// Fetches and parses the workspace billing status (available credit).
    pub async fn fetch_billing_status(
        &self,
        api_key: &str,
    ) -> Result<BillingReport, OpenCodeError> {
        if api_key.trim().is_empty() {
            return Err(OpenCodeError::CredentialsMissing);
        }
        let body = self.client.fetch_billing_status(api_key).await?;
        parse_billing_status(&body, chrono::Utc::now().timestamp_millis())
    }

    /// Fetches the raw usage JSON for the debug response view.
    ///
    /// The `api_key` is treated as a secret and only ever transmitted as a
    /// request header.
    pub async fn fetch_raw_usage(
        &self,
        api_key: &str,
        workspace_id: Option<&str>,
    ) -> Result<String, OpenCodeError> {
        if api_key.trim().is_empty() {
            return Err(OpenCodeError::CredentialsMissing);
        }
        self.client
            .fetch_usage(api_key, normalize_workspace(workspace_id))
            .await
    }
}

/// Treats a blank workspace identifier as "not configured".
fn normalize_workspace(workspace_id: Option<&str>) -> Option<&str> {
    workspace_id
        .map(str::trim)
        .filter(|workspace_id| !workspace_id.is_empty())
}

impl Default for QuotaService {
    fn default() -> Self {
        Self::new().expect("the built-in endpoint and HTTP client settings are valid")
    }
}

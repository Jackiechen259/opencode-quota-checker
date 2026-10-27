//! Quota service that ties the usage client and parser together.
//!
//! This is the single entry point the desktop application uses to refresh
//! quota. It validates credentials before making any request and never exposes
//! the raw response body or the API key to callers.

use crate::client::OpenCodeClient;
use crate::error::OpenCodeError;
use crate::models::UsageReport;
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

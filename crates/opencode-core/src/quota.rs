//! Quota service that ties the dashboard client and parser together.
//!
//! This is the single entry point the desktop application uses to refresh
//! quota. It validates credentials before making any request and never exposes
//! the raw dashboard HTML or the `auth` cookie to callers.

use crate::client::OpenCodeClient;
use crate::error::OpenCodeError;
use crate::models::UsageReport;
use crate::parser::parse_quota;

/// Fetches and normalizes OpenCode Go quota for the desktop application.
#[derive(Clone, Debug)]
pub struct QuotaService {
    client: OpenCodeClient,
}

impl QuotaService {
    /// Creates a service for the production dashboard endpoint.
    pub fn new() -> Result<Self, OpenCodeError> {
        Ok(Self {
            client: OpenCodeClient::new()?,
        })
    }

    /// Creates a service backed by an existing client (used in tests).
    pub fn with_client(client: OpenCodeClient) -> Self {
        Self { client }
    }

    /// Fetches and parses the workspace dashboard into a normalized report.
    ///
    /// The `auth_cookie` is treated as a secret and only ever transmitted as a
    /// request header.
    pub async fn fetch_quota(
        &self,
        workspace_id: &str,
        auth_cookie: &str,
    ) -> Result<UsageReport, OpenCodeError> {
        if workspace_id.trim().is_empty() {
            return Err(OpenCodeError::CredentialsMissing);
        }
        if auth_cookie.trim().is_empty() {
            return Err(OpenCodeError::CredentialsMissing);
        }
        let html = self
            .client
            .fetch_dashboard(workspace_id, auth_cookie)
            .await?;
        parse_quota(&html, chrono::Utc::now().timestamp_millis())
    }

    /// Fetches the raw dashboard HTML for the debug response view.
    ///
    /// The `auth_cookie` is treated as a secret and only ever transmitted as a
    /// request header.
    pub async fn fetch_raw_dashboard(
        &self,
        workspace_id: &str,
        auth_cookie: &str,
    ) -> Result<String, OpenCodeError> {
        self.client.fetch_dashboard(workspace_id, auth_cookie).await
    }
}

impl Default for QuotaService {
    fn default() -> Self {
        Self::new().expect("the built-in endpoint and HTTP client settings are valid")
    }
}

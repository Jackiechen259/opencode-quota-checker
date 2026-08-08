//! Provider adapter for OpenCode Go.
//!
//! Combines the workspace configuration, the dashboard client, and the parser
//! into a single `fetch_quota` entry point that produces the shared quota
//! model. The adapter never touches the raw HTML itself; all OpenCode-specific
//! parsing stays in the isolated [`super::parser`] module.

use super::client::OpenCodeGoClient;
use super::parser::parse_open_code_go_quota;
use crate::models::UsageReport;
use crate::VolcError;

/// Adapter that turns OpenCode Go workspace configuration into a usage report.
#[derive(Clone, Debug)]
pub struct OpenCodeGoProvider {
    client: OpenCodeGoClient,
}

impl OpenCodeGoProvider {
    /// Creates a provider for the production endpoint.
    pub fn new() -> Result<Self, VolcError> {
        Ok(Self {
            client: OpenCodeGoClient::new()?,
        })
    }

    /// Creates a provider backed by an existing client (used in tests).
    pub fn with_client(client: OpenCodeGoClient) -> Self {
        Self { client }
    }

    /// Fetches and parses the workspace dashboard into a normalized report.
    ///
    /// The `auth_cookie` is treated as a secret and never leaves the request.
    pub async fn fetch_quota(
        &self,
        workspace_id: &str,
        auth_cookie: &str,
    ) -> Result<UsageReport, VolcError> {
        if workspace_id.trim().is_empty() {
            return Err(VolcError::CredentialsMissing);
        }
        if auth_cookie.trim().is_empty() {
            return Err(VolcError::CredentialsMissing);
        }
        let html = self
            .client
            .fetch_dashboard(workspace_id, auth_cookie)
            .await?;
        parse_open_code_go_quota(&html, chrono::Utc::now().timestamp_millis())
    }
}

impl Default for OpenCodeGoProvider {
    fn default() -> Self {
        Self::new().expect("the built-in endpoint and HTTP client settings are valid")
    }
}

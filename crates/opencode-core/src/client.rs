//! HTTP client for the OpenCode Go and Console usage endpoints.
//!
//! All requests authenticate with an OpenCode Console API key (`oc_sk_...`)
//! sent as a bearer token; the key is never included in returned values or
//! error messages. The legacy `x-opencode-org-id` header is only used by the
//! Go allowance endpoint when the user configured a workspace override.
//!
//! Endpoints:
//!
//! * `GET /zen/go/v1/usage` - Go allowance percentages.
//! * `GET /console/api/usage/summary` - workspace token totals.
//! * `GET /console/api/go/status` - Go subscription meters and period start.
//! * `GET /console/api/activity/go` - Go-only daily and per-model activity.
//! * `GET /console/api/billing/status` - available credit (optional).

use crate::token_usage::TOKEN_USAGE_RANGE;
use crate::OpenCodeError;
use std::time::Duration;
use url::Url;

const DEFAULT_ENDPOINT: &str = "https://opencode.ai";
const USAGE_PATH: &str = "/zen/go/v1/usage";
const CONSOLE_USAGE_PATH: &str = "/console/api/usage/summary";
const GO_STATUS_PATH: &str = "/console/api/go/status";
const GO_ACTIVITY_PATH: &str = "/console/api/activity/go";
const BILLING_STATUS_PATH: &str = "/console/api/billing/status";
const INCLUDE_LEGACY_KEYS: &str = "true";
const TIMEOUT: Duration = Duration::from_secs(15);

/// User agent required by the Go terms: clients must identify themselves
/// instead of presenting as a generic HTTP library.
const USER_AGENT: &str = concat!("opencode-quota-checker/", env!("CARGO_PKG_VERSION"));

/// How a 403 from a given endpoint should be interpreted.
#[derive(Debug, Clone, Copy)]
enum Forbidden {
    /// The Go allowance endpoint: no subscription or missing Usage permission.
    NotSubscribed,
    /// Console usage/status/activity: missing `usage:read` or workspace access.
    Usage,
    /// Billing status: missing `billing:read`.
    Billing,
}

impl Forbidden {
    fn into_error(self) -> OpenCodeError {
        match self {
            Self::NotSubscribed => OpenCodeError::NotSubscribed,
            Self::Usage => OpenCodeError::UsagePermissionDenied,
            Self::Billing => OpenCodeError::BillingPermissionDenied,
        }
    }
}

/// Reusable client for the OpenCode Go and Console usage endpoints.
#[derive(Clone, Debug)]
pub struct OpenCodeClient {
    http: reqwest::Client,
    endpoint: Url,
}

impl OpenCodeClient {
    /// Creates a client for the production endpoint.
    pub fn new() -> Result<Self, OpenCodeError> {
        let endpoint = Url::parse(DEFAULT_ENDPOINT).map_err(|error| {
            OpenCodeError::Config(format!("invalid built-in endpoint: {error}"))
        })?;
        Self::with_endpoint(endpoint)
    }

    /// Creates a client with an alternate endpoint for integration tests.
    pub fn with_endpoint(endpoint: Url) -> Result<Self, OpenCodeError> {
        Self::with_endpoint_and_timeout(endpoint, TIMEOUT)
    }

    /// Creates a client with an alternate endpoint and timeout.
    pub fn with_endpoint_and_timeout(
        endpoint: Url,
        timeout: Duration,
    ) -> Result<Self, OpenCodeError> {
        if endpoint.host_str().is_none() {
            return Err(OpenCodeError::Config(
                "usage endpoint must include a host".to_owned(),
            ));
        }
        let http = reqwest::Client::builder()
            .connect_timeout(timeout)
            .timeout(timeout)
            .build()?;
        Ok(Self { http, endpoint })
    }

    /// Fetches the raw Go allowance JSON for the workspace behind `api_key`.
    ///
    /// `workspace_id` selects the workspace through `x-opencode-org-id` and is
    /// omitted when empty, because a personal API key is already bound to the
    /// workspace it was created in.
    pub async fn fetch_usage(
        &self,
        api_key: &str,
        workspace_id: Option<&str>,
    ) -> Result<String, OpenCodeError> {
        self.get_json(
            api_key,
            USAGE_PATH,
            &[],
            workspace_id,
            Forbidden::NotSubscribed,
        )
        .await
    }

    /// Fetches the raw workspace token-usage summary for the last 30 days.
    pub async fn fetch_token_usage(&self, api_key: &str) -> Result<String, OpenCodeError> {
        self.get_json(
            api_key,
            CONSOLE_USAGE_PATH,
            &[
                ("range", TOKEN_USAGE_RANGE),
                ("includeLegacyKeys", INCLUDE_LEGACY_KEYS),
            ],
            None,
            Forbidden::Usage,
        )
        .await
    }

    /// Fetches the Go subscription status.
    ///
    /// `fresh` asks the server to bypass its short-lived status cache; the
    /// overview page uses it for manual refreshes.
    pub async fn fetch_go_status(
        &self,
        api_key: &str,
        fresh: bool,
    ) -> Result<String, OpenCodeError> {
        let query: &[(&str, &str)] = if fresh { &[("fresh", "true")] } else { &[] };
        self.get_json(api_key, GO_STATUS_PATH, query, None, Forbidden::Usage)
            .await
    }

    /// Fetches the Go-only daily and per-model activity for the paid period.
    pub async fn fetch_go_activity(&self, api_key: &str) -> Result<String, OpenCodeError> {
        self.get_json(api_key, GO_ACTIVITY_PATH, &[], None, Forbidden::Usage)
            .await
    }

    /// Fetches the workspace billing status (available credit).
    ///
    /// Requires `billing:read`; callers must treat a 403 as an optional card
    /// failure rather than a page failure.
    pub async fn fetch_billing_status(&self, api_key: &str) -> Result<String, OpenCodeError> {
        self.get_json(api_key, BILLING_STATUS_PATH, &[], None, Forbidden::Billing)
            .await
    }

    /// Shared GET implementation with bounded error bodies.
    async fn get_json(
        &self,
        api_key: &str,
        path: &str,
        query: &[(&str, &str)],
        workspace_id: Option<&str>,
        forbidden: Forbidden,
    ) -> Result<String, OpenCodeError> {
        let url = self
            .endpoint
            .join(path)
            .map_err(|error| OpenCodeError::Config(format!("invalid endpoint path: {error}")))?;

        let mut request = self
            .http
            .get(url)
            .query(query)
            .header("Accept", "application/json")
            .header("User-Agent", USER_AGENT)
            .bearer_auth(api_key);
        if let Some(workspace_id) = workspace_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            request = request.header("x-opencode-org-id", workspace_id);
        }

        let response = request.send().await?;
        let status = response.status();
        let body = response.text().await?;

        if status.is_success() {
            return Ok(body);
        }
        Err(match status.as_u16() {
            401 => OpenCodeError::AuthenticationFailed,
            403 => forbidden.into_error(),
            429 => OpenCodeError::RateLimited,
            _ => OpenCodeError::Http {
                status,
                body: truncate(&body, 500),
            },
        })
    }
}

impl Default for OpenCodeClient {
    fn default() -> Self {
        Self::new().expect("the built-in endpoint and HTTP client settings are valid")
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut characters = value.chars();
    let prefix = characters.by_ref().take(max_chars).collect::<String>();
    if characters.next().is_some() {
        format!("{prefix}…(truncated)")
    } else {
        prefix
    }
}

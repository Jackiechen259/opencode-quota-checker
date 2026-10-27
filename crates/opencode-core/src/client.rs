//! HTTP client for the OpenCode Go usage API.
//!
//! `GET https://opencode.ai/zen/go/v1/usage` returns the three Go allowance
//! windows as JSON. It authenticates with an OpenCode Console API key
//! (`oc_sk_...`) sent as a bearer token; the key is never included in returned
//! values or error messages.

use crate::OpenCodeError;
use std::time::Duration;
use url::Url;

const DEFAULT_ENDPOINT: &str = "https://opencode.ai";
const USAGE_PATH: &str = "/zen/go/v1/usage";
const TIMEOUT: Duration = Duration::from_secs(15);

/// User agent required by the Go terms: clients must identify themselves
/// instead of presenting as a generic HTTP library.
const USER_AGENT: &str = concat!("opencode-quota-checker/", env!("CARGO_PKG_VERSION"));

/// Reusable client for the OpenCode Go usage endpoint.
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

    /// Fetches the raw usage JSON for the workspace behind `api_key`.
    ///
    /// `api_key` is a secret and only ever transmitted as the `Authorization`
    /// header. `workspace_id` selects the workspace through `x-opencode-org-id`
    /// and is omitted when empty, because a personal API key is already bound to
    /// the workspace it was created in.
    pub async fn fetch_usage(
        &self,
        api_key: &str,
        workspace_id: Option<&str>,
    ) -> Result<String, OpenCodeError> {
        let url = self.endpoint.join(USAGE_PATH).map_err(|error| {
            OpenCodeError::Config(format!("invalid usage endpoint path: {error}"))
        })?;

        let mut request = self
            .http
            .get(url)
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
            // A key that cannot read the workspace's Go allowance either belongs
            // to a workspace without a Go subscription or lacks the permission.
            403 => OpenCodeError::NotSubscribed,
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

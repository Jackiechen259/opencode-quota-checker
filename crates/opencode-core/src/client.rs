//! HTTP client for the OpenCode Go workspace dashboard.
//!
//! OpenCode Go has no documented public quota API, so quota is read from the
//! authenticated HTML dashboard. The `auth` cookie is sent as a request header
//! and is never included in returned values or error messages.

use crate::OpenCodeError;
use std::time::Duration;
use url::Url;
use urlencoding::encode;

const DEFAULT_ENDPOINT: &str = "https://opencode.ai";
const TIMEOUT: Duration = Duration::from_secs(15);

/// Reusable client for the OpenCode Go dashboard endpoint.
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
                "dashboard endpoint must include a host".to_owned(),
            ));
        }
        let http = reqwest::Client::builder()
            .connect_timeout(timeout)
            .timeout(timeout)
            .build()?;
        Ok(Self { http, endpoint })
    }

    /// Fetches the authenticated workspace dashboard and returns its HTML body.
    ///
    /// The `auth_cookie` is only ever transmitted as a request header. Failed
    /// responses map to typed errors and never leak the cookie.
    pub async fn fetch_dashboard(
        &self,
        workspace_id: &str,
        auth_cookie: &str,
    ) -> Result<String, OpenCodeError> {
        let path = format!("/workspace/{}/go", encode(workspace_id));
        let url = self
            .endpoint
            .join(&path)
            .map_err(|error| OpenCodeError::Config(format!("invalid dashboard path: {error}")))?;

        let response = self
            .http
            .get(url)
            .header("Accept", "text/html")
            .header("Cookie", format!("auth={auth_cookie}"))
            .send()
            .await?;
        let status = response.status();
        let body = response.text().await?;

        if status.is_success() {
            return Ok(body);
        }
        Err(match status.as_u16() {
            401 | 403 => OpenCodeError::AuthenticationFailed,
            404 => OpenCodeError::WorkspaceNotFound,
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

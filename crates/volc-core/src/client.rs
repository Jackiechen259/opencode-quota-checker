use crate::models::AfpResponse;
use crate::signing::sign_v4;
use crate::{Credentials, UsageReport, VolcError};
use std::time::Duration;
use url::Url;

const DEFAULT_ENDPOINT: &str = "https://open.volcengineapi.com/";
const REGION: &str = "cn-beijing";
const SERVICE: &str = "ark";
const CONTENT_TYPE: &str = "application/json; charset=UTF-8";
const ACTION: &str = "GetAFPUsage";
const VERSION: &str = "2024-01-01";
const TIMEOUT: Duration = Duration::from_secs(15);

/// Reusable client for the Volcano Ark usage API.
#[derive(Clone, Debug)]
pub struct ArkClient {
    http: reqwest::Client,
    endpoint: Url,
}

impl ArkClient {
    /// Creates a client for the production endpoint.
    pub fn new() -> Result<Self, VolcError> {
        let endpoint = Url::parse(DEFAULT_ENDPOINT)
            .map_err(|error| VolcError::Config(format!("invalid built-in endpoint: {error}")))?;
        Self::with_endpoint(endpoint)
    }

    /// Creates a client with an alternate endpoint for integration tests.
    pub fn with_endpoint(endpoint: Url) -> Result<Self, VolcError> {
        if endpoint.host_str().is_none() {
            return Err(VolcError::Config(
                "API endpoint must include a host".to_owned(),
            ));
        }
        let http = reqwest::Client::builder()
            .connect_timeout(TIMEOUT)
            .timeout(TIMEOUT)
            .build()?;
        Ok(Self { http, endpoint })
    }

    /// Fetches and converts the AFP usage report.
    pub async fn fetch_usage(&self, credentials: &Credentials) -> Result<UsageReport, VolcError> {
        let raw = self.fetch_usage_raw(credentials).await?;
        serde_json::from_str::<AfpResponse>(&raw)?.into_report()
    }

    /// Fetches the unpersisted raw AFP response.
    pub async fn fetch_usage_raw(&self, credentials: &Credentials) -> Result<String, VolcError> {
        let host = authority(&self.endpoint)?;
        let path = if self.endpoint.path().is_empty() {
            "/"
        } else {
            self.endpoint.path()
        };
        let body = b"{}";
        let query = [("Action", ACTION), ("Version", VERSION)];
        let format_date = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        let signed = sign_v4(
            "POST",
            path,
            &host,
            &query,
            body,
            credentials.access_key(),
            credentials.secret_key(),
            REGION,
            SERVICE,
            CONTENT_TYPE,
            &format_date,
        )?;

        let response = self
            .http
            .post(self.endpoint.clone())
            .query(&query)
            .header("Host", host)
            .header("Content-Type", CONTENT_TYPE)
            .header("X-Date", signed.x_date)
            .header("X-Content-Sha256", signed.x_content_sha256)
            .header("Authorization", signed.authorization)
            .body(body.to_vec())
            .send()
            .await?;
        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            return Err(VolcError::Http {
                status,
                body: truncate(&text, 500),
            });
        }
        Ok(text)
    }
}

impl Default for ArkClient {
    fn default() -> Self {
        Self::new().expect("the built-in endpoint and HTTP client settings are valid")
    }
}

fn authority(endpoint: &Url) -> Result<String, VolcError> {
    let host = endpoint
        .host_str()
        .ok_or_else(|| VolcError::Config("API endpoint must include a host".to_owned()))?;
    Ok(endpoint
        .port()
        .map_or_else(|| host.to_owned(), |port| format!("{host}:{port}")))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncation_respects_unicode_boundaries() {
        assert_eq!(truncate("火山方舟", 2), "火山…(truncated)");
        assert_eq!(truncate("火山", 2), "火山");
    }

    #[test]
    fn endpoint_requires_a_host() {
        let endpoint = Url::parse("file:///tmp/socket").expect("test URL parses");
        assert!(ArkClient::with_endpoint(endpoint).is_err());
    }
}

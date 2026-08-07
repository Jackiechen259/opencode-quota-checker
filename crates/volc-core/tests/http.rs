use std::time::Duration;
use volc_core::{ArkClient, Credentials, VolcError};
use wiremock::matchers::{header_exists, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn credentials() -> Credentials {
    Credentials::new("test-access-key", "test-secret-key").expect("test credentials are valid")
}

async fn client(server: &MockServer) -> ArkClient {
    ArkClient::with_endpoint(
        format!("{}/", server.uri())
            .parse()
            .expect("mock server URL is valid"),
    )
    .expect("mock endpoint creates a client")
}

#[tokio::test]
async fn sends_signed_request_and_parses_success() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .and(query_param("Action", "GetAFPUsage"))
        .and(query_param("Version", "2024-01-01"))
        .and(header_exists("authorization"))
        .and(header_exists("x-date"))
        .and(header_exists("x-content-sha256"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("../../../tests/fixtures/usage-small.json")),
        )
        .expect(1)
        .mount(&server)
        .await;

    let report = client(&server)
        .await
        .fetch_usage(&credentials())
        .await
        .expect("mock response is successful");
    assert_eq!(report.plan_type, "Large");
    assert_eq!(report.windows.len(), 3);
}

#[tokio::test]
async fn maps_common_http_errors_without_unbounded_bodies() {
    for status in [401, 403, 429, 500] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(status).set_body_string("x".repeat(800)))
            .mount(&server)
            .await;

        let error = client(&server)
            .await
            .fetch_usage_raw(&credentials())
            .await
            .expect_err("non-success response must fail");
        match error {
            VolcError::Http {
                status: actual,
                body,
            } => {
                assert_eq!(actual.as_u16(), status);
                assert!(body.chars().count() < 550);
            }
            other => panic!("expected HTTP error, got {other}"),
        }
    }
}

#[tokio::test]
async fn rejects_non_json_success_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<html>not json</html>"))
        .mount(&server)
        .await;

    let error = client(&server)
        .await
        .fetch_usage(&credentials())
        .await
        .expect_err("invalid JSON must fail");
    assert!(matches!(error, VolcError::Response(_)));
}

#[tokio::test]
async fn enforces_request_timeout() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(150))
                .set_body_string(include_str!("../../../tests/fixtures/usage-small.json")),
        )
        .mount(&server)
        .await;
    let endpoint = format!("{}/", server.uri())
        .parse()
        .expect("mock server URL is valid");
    let client = ArkClient::with_endpoint_and_timeout(endpoint, Duration::from_millis(25))
        .expect("test timeout creates a client");

    let error = client
        .fetch_usage(&credentials())
        .await
        .expect_err("delayed response must time out");
    assert!(matches!(error, VolcError::Request(ref source) if source.is_timeout()));
}
